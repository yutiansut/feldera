//! Test framework for the `adapters` crate.

use crate::{
    controller::InputEndpointConfig, transport::InputReader, Catalog, CircuitCatalog,
    DbspCircuitHandle, DeserializeWithContext, FormatConfig, InputTransport, SqlSerdeConfig,
};
use anyhow::Result as AnyResult;
use dbsp::Runtime;
use log::{Log, Metadata, Record};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

mod data;

#[cfg(feature = "with-kafka")]
pub mod kafka;

pub mod http;

mod mock_dezset;
mod mock_input_consumer;
mod mock_output_consumer;

pub use data::{
    generate_test_batch, generate_test_batches, generate_test_batches_with_weights, TestStruct,
};
pub use mock_dezset::{MockDeZSet, MockUpdate};
pub use mock_input_consumer::MockInputConsumer;
pub use mock_output_consumer::MockOutputConsumer;

pub struct TestLogger;
pub static TEST_LOGGER: TestLogger = TestLogger;

pub static DEFAULT_TIMEOUT_MS: u128 = 600_000;

impl Log for TestLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        println!("{} - {}", record.level(), record.args());
    }

    fn flush(&self) {}
}

/// Wait for `predicate` to become `true`.
///
/// Returns the number of milliseconds elapsed or `None` on timeout.
pub fn wait<P>(mut predicate: P, timeout_ms: u128) -> Option<u128>
where
    P: FnMut() -> bool,
{
    let start = Instant::now();

    while !predicate() {
        if start.elapsed().as_millis() >= timeout_ms {
            return None;
        }
        sleep(Duration::from_millis(10));
    }

    Some(start.elapsed().as_millis())
}

/// Build an input pipeline that allows testing a parser
/// standalone, without a DBSP circuit or controller.
///
/// ```text
/// ┌─────────────────┐   ┌──────┐   ┌──────────┐
/// │MockInputConsumer├──►│parser├──►│MockDeZSet│
/// └─────────────────┘   └──────┘   └──────────┘
/// ```
pub fn mock_parser_pipeline<T, U>(
    config: &FormatConfig,
) -> AnyResult<(MockInputConsumer, MockDeZSet<T, U>)>
where
    T: for<'de> DeserializeWithContext<'de, SqlSerdeConfig> + Send + 'static,
    U: for<'de> DeserializeWithContext<'de, SqlSerdeConfig> + Send + 'static,
{
    let input_handle = <MockDeZSet<T, U>>::new();
    let consumer = MockInputConsumer::from_handle(&input_handle, config);
    Ok((consumer, input_handle))
}

/// Build an input pipeline that allows testing a transport endpoint and parser
/// standalone, without a DBSP circuit or controller.
///
/// Creates a mock `Catalog` with a single input handle with name `name`
/// and record type `T` backed by `MockDeZSet` and instantiates the following
/// test pipeline:
///
/// ```text
/// ┌────────┐   ┌─────────────────┐   ┌──────┐   ┌──────────┐
/// │endpoint├──►│MockInputConsumer├──►│parser├──►│MockDeZSet│
/// └────────┘   └─────────────────┘   └──────┘   └──────────┘
/// ```
pub fn mock_input_pipeline<T, U>(
    config: InputEndpointConfig,
) -> AnyResult<(Box<dyn InputReader>, MockInputConsumer, MockDeZSet<T, U>)>
where
    T: for<'de> DeserializeWithContext<'de, SqlSerdeConfig> + Send + 'static,
    U: for<'de> DeserializeWithContext<'de, SqlSerdeConfig> + Send + 'static,
{
    let (consumer, input_handle) = mock_parser_pipeline(&config.connector_config.format)?;

    let transport =
        <dyn InputTransport>::get_transport(&config.connector_config.transport.name).unwrap();
    let endpoint = transport.new_endpoint(&config.connector_config.transport.config)?;

    let reader = endpoint.open(Box::new(consumer.clone()), 0)?;

    Ok((reader, consumer, input_handle))
}

/// Create a simple test circuit that passes the input stream right through to
/// the output.
// TODO: parameterize with the number (and types?) of input and output streams.
pub fn test_circuit(workers: usize) -> (Box<dyn DbspCircuitHandle>, Box<dyn CircuitCatalog>) {
    let (circuit, catalog) = Runtime::init_circuit(workers, |circuit| {
        let mut catalog = Catalog::new();
        let (input, hinput) = circuit.add_input_zset::<TestStruct, i32>();

        catalog.register_input_zset("test_input1", input.clone(), hinput);
        catalog.register_output_zset("test_output1", input);

        Ok(catalog)
    })
    .unwrap();
    (Box::new(circuit), Box::new(catalog))
}
