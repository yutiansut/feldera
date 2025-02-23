use super::{process_time, NexmarkStream};
use crate::model::Event;
use dbsp::{
    operator::FilterMap,
    utils::{Tup2, Tup3, Tup5, Tup7},
    OrdZSet, RootCircuit, Stream,
};

use csv;
use std::{
    fs::File,
    io::{BufReader, Read, Result},
};

/// Query 13: Bounded Side Input Join (Not in original suite)
///
/// Joins a stream to a bounded side input, modeling basic stream enrichment.
///
/// ```sql
/// CREATE TABLE side_input (
///   key BIGINT,
///   `value` VARCHAR
/// ) WITH (
///   'connector.type' = 'filesystem',
///   'connector.path' = 'file://${FLINK_HOME}/data/side_input.txt',
///   'format.type' = 'csv'
/// );
///
/// CREATE TABLE discard_sink (
///   auction  BIGINT,
///   bidder  BIGINT,
///   price  BIGINT,
///   dateTime  TIMESTAMP(3),
///   `value`  VARCHAR
/// ) WITH (
///   'connector' = 'blackhole'
/// );
///
/// INSERT INTO discard_sink
/// SELECT
///     B.auction,
///     B.bidder,
///     B.price,
///     B.dateTime,
///     S.`value`
/// FROM (SELECT *, PROCTIME() as p_time FROM bid) B
/// JOIN side_input FOR SYSTEM_TIME AS OF B.p_time AS S
/// ON mod(B.auction, 10000) = S.key;
/// ```

type Q13Stream = Stream<RootCircuit, OrdZSet<Tup5<u64, u64, u64, u64, String>, i64>>;

type SideInputStream = Stream<RootCircuit, OrdZSet<Tup3<u64, String, u64>, i64>>;

const Q13_SIDE_INPUT_CSV: &str = "benches/nexmark/data/side_input.txt";

fn read_side_input<R: Read>(reader: R) -> Result<Vec<(u64, String)>> {
    let reader = BufReader::new(reader);
    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);
    Ok(csv_reader.deserialize().map(|r| r.unwrap()).collect())
}

pub fn q13_side_input() -> Vec<(Tup3<u64, String, u64>, i64)> {
    let p_time = process_time();
    read_side_input(File::open(Q13_SIDE_INPUT_CSV).unwrap())
        .unwrap()
        .into_iter()
        .map(|(k, v)| (Tup3(k, v, p_time), 1))
        .collect()
}

pub fn q13(input: NexmarkStream, side_input: SideInputStream) -> Q13Stream {
    // Index bids by the modulo value.
    let bids_by_auction_mod = input.flat_map_index(move |event| match event {
        Event::Bid(b) => Some((
            b.auction % 10_000,
            Tup5(b.auction, b.bidder, b.price, b.date_time, process_time()),
        )),
        _ => None,
    });

    // Index the side_input by the key.
    let side_input_indexed = side_input.map_index(|Tup3(k, v, t)| (*k, Tup2(v.clone(), *t)));

    // Join on the key from the side input
    bids_by_auction_mod
        .join(
            &side_input_indexed,
            |&_,
             &Tup5(auction, bidder, price, date_time, b_p_time),
             Tup2(input_value, input_p_time)| {
                Tup7(
                    auction,
                    bidder,
                    price,
                    date_time,
                    input_value.clone(),
                    b_p_time,
                    *input_p_time,
                )
            },
        )
        .flat_map(
            |Tup7(auction, bidder, price, date_time, input_value, b_p_time, input_p_time)| {
                if b_p_time >= input_p_time {
                    Some(Tup5(
                        *auction,
                        *bidder,
                        *price,
                        *date_time,
                        input_value.clone(),
                    ))
                } else {
                    None
                }
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generator::tests::make_bid, model::Bid};
    use dbsp::zset;

    #[test]
    fn test_q13() {
        let input_vecs = vec![vec![
            (
                Event::Bid(Bid {
                    auction: 1005,
                    ..make_bid()
                }),
                1,
            ),
            (
                Event::Bid(Bid {
                    auction: 10005,
                    ..make_bid()
                }),
                1,
            ),
        ]]
        .into_iter();

        let (circuit, (input_handle, side_input_handle)) = RootCircuit::build(move |circuit| {
            let (stream, input_handle) = circuit.add_input_zset::<Event, i64>();
            let (side_stream, side_input_handle) =
                circuit.add_input_zset::<Tup3<u64, String, u64>, i64>();

            let mut expected_output = vec![zset![
                Tup5(1_005, 1, 99, 0, String::from("1005")) => 1,
                Tup5(10_005, 1, 99, 0, String::from("5")) => 1,
            ]]
            .into_iter();

            let output = q13(stream, side_stream);

            output.inspect(move |batch| assert_eq!(batch, &expected_output.next().unwrap()));

            Ok((input_handle, side_input_handle))
        })
        .unwrap();

        side_input_handle.append(&mut q13_side_input());
        for mut vec in input_vecs {
            input_handle.append(&mut vec);
            circuit.step().unwrap();
        }
    }

    #[test]
    fn test_read_side_input() {
        let reader = "1,five\n2,four\n3,three".as_bytes();

        let got = read_side_input(reader).unwrap();

        assert_eq!(
            vec![
                (1, String::from("five")),
                (2, String::from("four")),
                (3, String::from("three")),
            ],
            got
        );
    }
}
