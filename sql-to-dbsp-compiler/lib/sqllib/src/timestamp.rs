//! Support for SQL Timestamp and Date data types.

use crate::interval::{LongInterval, ShortInterval};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};
use core::fmt::Formatter;
use dbsp::num_entries_scalar;
use dbsp_adapters::{
    DateFormat, DeserializeWithContext, SerializeWithContext, SqlSerdeConfig, TimeFormat,
    TimestampFormat,
};
use serde::{de::Error as _, ser::Error as _, Deserialize, Deserializer, Serializer};
use size_of::SizeOf;
use std::{
    borrow::Cow,
    fmt::{self, Debug},
    ops::Add,
};

use crate::{
    operators::{eq, gt, gte, lt, lte, neq},
    polymorphic_return_function2, some_existing_operator, some_function2, some_operator,
    some_polymorphic_function1, some_polymorphic_function2, some_polymorphic_function3,
};

/// Similar to a unix timestamp: a positive time interval between Jan 1 1970 and
/// the current time. The supported range is limited (e.g., up to 2038 in
/// MySQL). We use milliseconds to represent the interval.
#[derive(
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    SizeOf,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
#[archive_attr(derive(Clone, Ord, Eq, PartialEq, PartialOrd))]
#[archive(compare(PartialEq, PartialOrd))]
pub struct Timestamp {
    // since unix epoch
    milliseconds: i64,
}

impl Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let datetime = NaiveDateTime::from_timestamp_millis(self.milliseconds).ok_or(fmt::Error)?;

        f.write_str(&datetime.format("%F %T%.f").to_string())
    }
}

impl SerializeWithContext<SqlSerdeConfig> for Timestamp {
    fn serialize_with_context<S>(
        &self,
        serializer: S,
        context: &SqlSerdeConfig,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let datetime = DateTime::from_timestamp(
            self.milliseconds / 1000,
            ((self.milliseconds % 1000) * 1000) as u32,
        )
        .ok_or_else(|| {
            S::Error::custom(format!(
                "timestamp value '{}' out of range",
                self.milliseconds
            ))
        })?;
        match context.timestamp_format {
            TimestampFormat::String(format_string) => {
                serializer.serialize_str(&datetime.format(format_string).to_string())
            }
        }
    }
}

impl<'de> DeserializeWithContext<'de, SqlSerdeConfig> for Timestamp {
    fn deserialize_with_context<D>(
        deserializer: D,
        config: &'de SqlSerdeConfig,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match config.timestamp_format {
            TimestampFormat::String(format) => {
                // `timestamp_str: &'de` doesn't work for JSON, which escapes strings
                // and can only deserialize into an owned string.
                let timestamp_str: Cow<'de, str> = Deserialize::deserialize(deserializer)?;

                let timestamp = NaiveDateTime::parse_from_str(timestamp_str.trim(), format)
                    .map_err(|e| {
                        D::Error::custom(format!("invalid timestamp string '{timestamp_str}': {e}"))
                    })?;

                Ok(Self::new(timestamp.timestamp_millis()))
            }
        }
    }
}

/// Deserialize timestamp from the `YYYY-MM-DD HH:MM:SS.fff` format.
/// For a flexible deserialization framework use deserialize_with_context.
impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // `timestamp_str: &'de` doesn't work for JSON, which escapes strings
        // and can only deserialize into an owned string.
        let timestamp_str: Cow<'de, str> = Deserialize::deserialize(deserializer)?;

        let timestamp =
            NaiveDateTime::parse_from_str(timestamp_str.trim(), "%F %T%.f").map_err(|e| {
                D::Error::custom(format!("invalid timestamp string '{timestamp_str}': {e}"))
            })?;

        Ok(Self::new(timestamp.timestamp_millis()))
    }
}

impl Timestamp {
    pub const fn new(milliseconds: i64) -> Self {
        Self { milliseconds }
    }

    pub fn milliseconds(&self) -> i64 {
        self.milliseconds
    }

    pub fn to_dateTime(&self) -> DateTime<Utc> {
        Utc.timestamp_opt(
            self.milliseconds / 1000,
            ((self.milliseconds % 1000i64) as u32) * 1_000_000,
        )
        .single()
        .unwrap()
    }

    pub fn from_dateTime(date: DateTime<Utc>) -> Self {
        Self {
            milliseconds: date.timestamp_millis(),
        }
    }
}

impl<T> From<T> for Timestamp
where
    i64: From<T>,
{
    fn from(value: T) -> Self {
        Self {
            milliseconds: i64::from(value),
        }
    }
}

impl Add<i64> for Timestamp {
    type Output = Self;

    fn add(self, value: i64) -> Self {
        Self {
            milliseconds: self.milliseconds + value,
        }
    }
}

pub fn plus_Timestamp_ShortInterval(left: Timestamp, right: ShortInterval) -> Timestamp {
    left.add(right.milliseconds())
}

pub fn minus_Timestamp_Timestamp_ShortInterval(left: Timestamp, right: Timestamp) -> ShortInterval {
    ShortInterval::from(left.milliseconds() - right.milliseconds())
}

polymorphic_return_function2!(
    minus,
    Timestamp,
    Timestamp,
    Timestamp,
    Timestamp,
    ShortInterval,
    ShortInterval
);

pub fn minus_Timestamp_ShortInterval_Timestamp(left: Timestamp, right: ShortInterval) -> Timestamp {
    Timestamp::new(left.milliseconds() - right.milliseconds())
}

polymorphic_return_function2!(
    minus,
    Timestamp,
    Timestamp,
    ShortInterval,
    ShortInterval,
    Timestamp,
    Timestamp
);

pub fn minus_Timestamp_Timestamp_LongInterval(left: Timestamp, right: Timestamp) -> LongInterval {
    let ldate = left.to_dateTime();
    let rdate = right.to_dateTime();
    let ly = ldate.year();
    let lm = ldate.month() as i32;
    let ld = ldate.day() as i32;
    let lt = ldate.time();

    let ry = rdate.year();
    let mut rm = rdate.month() as i32;
    let rd = rdate.day() as i32;
    let rt = rdate.time();
    if (ld < rd) || ((ld == rd) && lt < rt) {
        // the full month is not yet elapsed
        rm += 1;
    }
    LongInterval::from((ly - ry) * 12 + lm - rm)
}

polymorphic_return_function2!(
    minus,
    Timestamp,
    Timestamp,
    Timestamp,
    Timestamp,
    LongInterval,
    LongInterval
);

pub fn extract_year_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    date.year().into()
}

pub fn extract_month_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    date.month().into()
}

pub fn extract_day_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    date.day().into()
}

pub fn extract_quarter_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    (date.month0() / 3 + 1).into()
}

pub fn extract_decade_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    (date.year() / 10).into()
}

pub fn extract_century_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    ((date.year() + 99) / 100).into()
}

pub fn extract_millennium_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    ((date.year() + 999) / 1000).into()
}

pub fn extract_isoyear_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    date.iso_week().year().into()
}

pub fn extract_week_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    date.iso_week().week().into()
}

pub fn extract_dow_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    (date.weekday().num_days_from_sunday() as i64) + Date::first_day_of_week()
}

pub fn extract_isodow_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    (date.weekday().num_days_from_monday() as i64) + 1
}

pub fn extract_doy_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    date.ordinal().into()
}

pub fn extract_epoch_Timestamp(t: Timestamp) -> i64 {
    t.milliseconds() / 1000
}

pub fn extract_millisecond_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    (date.second() * 1000 + date.timestamp_subsec_millis()).into()
}

pub fn extract_microsecond_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    (date.second() * 1_000_000 + date.timestamp_subsec_micros()).into()
}

pub fn extract_second_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    date.second().into()
}

pub fn extract_minute_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    date.minute().into()
}

pub fn extract_hour_Timestamp(value: Timestamp) -> i64 {
    let date = value.to_dateTime();
    date.hour().into()
}

some_polymorphic_function1!(extract_year, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_month, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_day, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_quarter, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_decade, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_century, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_millennium, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_isoyear, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_week, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_dow, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_isodow, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_doy, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_epoch, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_millisecond, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_microsecond, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_second, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_minute, Timestamp, Timestamp, i64);
some_polymorphic_function1!(extract_hour, Timestamp, Timestamp, i64);

some_operator!(lt, Timestamp, Timestamp, bool);
some_operator!(gt, Timestamp, Timestamp, bool);
some_operator!(eq, Timestamp, Timestamp, bool);
some_operator!(neq, Timestamp, Timestamp, bool);
some_operator!(gte, Timestamp, Timestamp, bool);
some_operator!(lte, Timestamp, Timestamp, bool);

pub fn floor_week_Timestamp(value: Timestamp) -> Timestamp {
    let wd = extract_dow_Timestamp(value) - Date::first_day_of_week();
    let ts = value.to_dateTime();
    let notime = ts
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let notimeTs = Timestamp::from_dateTime(notime);
    let interval = ShortInterval::new(wd * 86400 * 1000);

    minus_Timestamp_ShortInterval_Timestamp(notimeTs, interval)
}

some_polymorphic_function1!(floor_week, Timestamp, Timestamp, Timestamp);

pub fn tumble_Timestamp_ShortInterval(ts: Timestamp, i: ShortInterval) -> Timestamp {
    let ts_ms = ts.milliseconds();
    let i_ms = i.milliseconds();
    let round = ts_ms - ts_ms % i_ms;
    Timestamp::new(round)
}

some_polymorphic_function2!(
    tumble,
    Timestamp,
    Timestamp,
    ShortInterval,
    ShortInterval,
    Timestamp
);

pub fn tumble_Timestamp_ShortInterval_Time(ts: Timestamp, i: ShortInterval, t: Time) -> Timestamp {
    let t_ms = (t.nanoseconds() / 1000000) as i64;
    let ts_ms = ts.milliseconds() - t_ms;
    let i_ms = i.milliseconds();
    let round = ts_ms - ts_ms % i_ms;
    Timestamp::new(round + t_ms)
}

some_polymorphic_function3!(
    tumble,
    Timestamp,
    Timestamp,
    ShortInterval,
    ShortInterval,
    Time,
    Time,
    Timestamp
);

//////////////////////////// Date

#[derive(
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    SizeOf,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
#[archive_attr(derive(Clone, Ord, Eq, PartialEq, PartialOrd))]
#[archive(compare(PartialEq, PartialOrd))]
pub struct Date {
    // since unix epoch
    days: i32,
}

impl Date {
    pub const fn new(days: i32) -> Self {
        Self { days }
    }

    pub fn days(&self) -> i32 {
        self.days
    }

    pub fn to_timestamp(&self) -> Timestamp {
        Timestamp::new((self.days as i64) * 86400 * 1000)
    }

    pub fn to_dateTime(&self) -> DateTime<Utc> {
        Utc.timestamp_opt(self.days() as i64 * 86400, 0)
            .single()
            .unwrap()
    }

    pub fn first_day_of_week() -> i64 {
        // This should depend on the SQL dialect, but the calcite
        // optimizer seems to imply this for all dialects.
        1
    }
}

impl fmt::Debug for Date {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let millis = (self.days as i64) * 86400 * 1000;
        let ndt = NaiveDateTime::from_timestamp_millis(millis);
        match ndt {
            Some(ndt) => ndt.date().fmt(f),
            _ => write!(f, "date value '{}' out of range", self.days),
        }
    }
}

impl<T> From<T> for Date
where
    i32: From<T>,
{
    fn from(value: T) -> Self {
        Self {
            days: i32::from(value),
        }
    }
}

impl SerializeWithContext<SqlSerdeConfig> for Date {
    fn serialize_with_context<S>(
        &self,
        serializer: S,
        context: &SqlSerdeConfig,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match context.date_format {
            DateFormat::String(format_string) => {
                let millis = (self.days as i64) * 86400 * 1000;
                let datetime = NaiveDateTime::from_timestamp_millis(millis).ok_or_else(|| {
                    S::Error::custom(format!("date value '{}' out of range", self.days))
                })?;

                serializer.serialize_str(&datetime.format(format_string).to_string())
            }
            DateFormat::DaysSinceEpoch => serializer.serialize_i32(self.days),
        }
    }
}

impl<'de> DeserializeWithContext<'de, SqlSerdeConfig> for Date {
    fn deserialize_with_context<D>(
        deserializer: D,
        config: &'de SqlSerdeConfig,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match config.date_format {
            DateFormat::String(format) => {
                let str: &'de str = Deserialize::deserialize(deserializer)?;
                let date = NaiveDate::parse_from_str(str.trim(), format)
                    .map_err(|e| D::Error::custom(format!("invalid date string '{str}': {e}")))?;
                Ok(Self::new(
                    (date.and_time(NaiveTime::default()).timestamp() / 86400) as i32,
                ))
            }
            DateFormat::DaysSinceEpoch => Ok(Self {
                days: i32::deserialize(deserializer)?,
            }),
        }
    }
}

/// Deserialize date from the `YYYY-MM-DD` format.
impl<'de> Deserialize<'de> for Date {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str: &'de str = Deserialize::deserialize(deserializer)?;
        let date = NaiveDate::parse_from_str(str.trim(), "%Y-%m-%d")
            .map_err(|e| D::Error::custom(format!("invalid date string '{str}': {e}")))?;
        Ok(Self::new(
            (date.and_time(NaiveTime::default()).timestamp() / 86400) as i32,
        ))
    }
}

some_operator!(lt, Date, Date, bool);
some_operator!(gt, Date, Date, bool);
some_operator!(eq, Date, Date, bool);
some_operator!(neq, Date, Date, bool);
some_operator!(gte, Date, Date, bool);
some_operator!(lte, Date, Date, bool);

// right - left
pub fn minus_Date_Date_LongInterval(left: Date, right: Date) -> LongInterval {
    // Logic adapted from https://github.com/mysql/mysql-server/blob/ea1efa9822d81044b726aab20c857d5e1b7e046a/sql/item_timefunc.cc
    let ld = left.to_dateTime();
    let rd = right.to_dateTime();

    let (beg, end, neg) = if ld > rd { (rd, ld, 1) } else { (ld, rd, -1) };

    let month_end = end.month() as i32;
    let month_beg = beg.month() as i32;
    let day_end = end.day();
    let day_beg = beg.day();

    // compute years
    let mut years = end.year() - beg.year();
    let adjust = month_end < month_beg || (month_end == month_beg && day_end < day_beg);
    if adjust {
        years -= 1;
    }

    // compute months
    let mut months = 12 * years;
    if adjust {
        months += 12 - (month_beg - month_end);
    } else {
        months += month_end - month_beg;
    }

    if day_end < day_beg {
        months -= 1;
    }

    LongInterval::from(months * neg)
}

pub fn minus_DateN_Date_LongIntervaNl(left: Option<Date>, right: Date) -> Option<LongInterval> {
    left.map(|x| LongInterval::new(x.days() - right.days()))
}

pub fn minus_Date_DateN_LongInterval(left: Date, right: Option<Date>) -> Option<LongInterval> {
    right.map(|x| LongInterval::new(left.days() - x.days()))
}

pub fn minus_DateN_DateN_LongIntervalN(
    left: Option<Date>,
    right: Option<Date>,
) -> Option<LongInterval> {
    match (left, right) {
        (None, _) => None,
        (_, None) => None,
        (Some(x), Some(y)) => Some(LongInterval::new(x.days() - y.days())),
    }
}

pub fn minus_Date_Date_ShortInterval(left: Date, right: Date) -> ShortInterval {
    let ld = left.days() as i64;
    let rd = right.days() as i64;
    ShortInterval::new((ld - rd) * 86400 * 1000)
}

pub fn minus_DateN_Date_ShortIntervalN(left: Option<Date>, right: Date) -> Option<ShortInterval> {
    left.map(|x| minus_Date_Date_ShortInterval(x, right))
}

pub fn minus_Date_DateN_ShortIntervalN(left: Date, right: Option<Date>) -> Option<ShortInterval> {
    right.map(|x| minus_Date_Date_ShortInterval(left, x))
}

pub fn minus_DateN_DateN_ShortIntervalN(
    left: Option<Date>,
    right: Option<Date>,
) -> Option<ShortInterval> {
    match (left, right) {
        (None, _) => None,
        (_, None) => None,
        (Some(x), Some(y)) => Some(minus_Date_Date_ShortInterval(x, y)),
    }
}

pub fn extract_year_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    date.year().into()
}

pub fn extract_month_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    date.month().into()
}

pub fn extract_day_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    date.day().into()
}

pub fn extract_quarter_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    (date.month0() / 3 + 1).into()
}

pub fn extract_decade_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    (date.year() / 10).into()
}

pub fn extract_century_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    ((date.year() + 99) / 100).into()
}

pub fn extract_millennium_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    ((date.year() + 999) / 1000).into()
}

pub fn extract_isoyear_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    date.iso_week().year().into()
}

pub fn extract_week_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    date.iso_week().week().into()
}

pub fn extract_dow_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    (date.weekday().num_days_from_sunday() as i64) + Date::first_day_of_week()
}

pub fn extract_isodow_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    (date.weekday().num_days_from_monday() as i64) + 1
}

pub fn extract_doy_Date(value: Date) -> i64 {
    let date = value.to_dateTime();
    date.ordinal().into()
}

pub fn extract_epoch_Date(value: Date) -> i64 {
    (value.days() as i64) * 86400
}

pub fn extract_second_Date(_value: Date) -> i64 {
    0
}

pub fn extract_minute_Date(_value: Date) -> i64 {
    0
}

pub fn extract_hour_Date(_value: Date) -> i64 {
    0
}

pub fn extract_millisecond_Date(_value: Date) -> i64 {
    0
}

pub fn extract_microsecond_Date(_value: Date) -> i64 {
    0
}

some_polymorphic_function1!(extract_year, Date, Date, i64);
some_polymorphic_function1!(extract_month, Date, Date, i64);
some_polymorphic_function1!(extract_day, Date, Date, i64);
some_polymorphic_function1!(extract_quarter, Date, Date, i64);
some_polymorphic_function1!(extract_decade, Date, Date, i64);
some_polymorphic_function1!(extract_century, Date, Date, i64);
some_polymorphic_function1!(extract_millennium, Date, Date, i64);
some_polymorphic_function1!(extract_isoyear, Date, Date, i64);
some_polymorphic_function1!(extract_week, Date, Date, i64);
some_polymorphic_function1!(extract_dow, Date, Date, i64);
some_polymorphic_function1!(extract_isodow, Date, Date, i64);
some_polymorphic_function1!(extract_doy, Date, Date, i64);
some_polymorphic_function1!(extract_epoch, Date, Date, i64);
some_polymorphic_function1!(extract_millisecond, Date, Date, i64);
some_polymorphic_function1!(extract_microsecond, Date, Date, i64);
some_polymorphic_function1!(extract_second, Date, Date, i64);
some_polymorphic_function1!(extract_minute, Date, Date, i64);
some_polymorphic_function1!(extract_hour, Date, Date, i64);

pub fn datediff_day_Date_Date(left: Date, right: Date) -> i32 {
    left.days() - right.days()
}

some_polymorphic_function2!(datediff_day, Date, Date, Date, Date, i32);

pub fn format_date__(format: String, date: Date) -> String {
    return date.to_dateTime().format(&format).to_string();
}

some_function2!(format_date, String, Date, String);

//////////////////////////// Time

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    SizeOf,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
#[archive_attr(derive(Clone, Ord, Eq, PartialEq, PartialOrd))]
#[archive(compare(PartialEq, PartialOrd))]
pub struct Time {
    nanoseconds: u64,
}

const BILLION: u64 = 1000000000;

impl Time {
    pub const fn new(nanoseconds: u64) -> Self {
        Self { nanoseconds }
    }

    pub fn nanoseconds(&self) -> u64 {
        self.nanoseconds
    }

    pub fn from_time(time: NaiveTime) -> Self {
        Self {
            nanoseconds: (time.num_seconds_from_midnight() as u64) * BILLION
                + (time.nanosecond() as u64),
        }
    }

    pub fn to_time(&self) -> NaiveTime {
        NaiveTime::from_num_seconds_from_midnight_opt(
            (self.nanoseconds / BILLION) as u32,
            (self.nanoseconds % BILLION) as u32,
        )
        .unwrap()
    }
}

impl<T> From<T> for Time
where
    u64: From<T>,
{
    fn from(value: T) -> Self {
        Self {
            nanoseconds: u64::from(value),
        }
    }
}

impl SerializeWithContext<SqlSerdeConfig> for Time {
    fn serialize_with_context<S>(
        &self,
        serializer: S,
        context: &SqlSerdeConfig,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match context.time_format {
            TimeFormat::String(format_string) => {
                let time = self.to_time();
                serializer.serialize_str(&time.format(format_string).to_string())
            }
            TimeFormat::Micros => serializer.serialize_u64(self.nanoseconds / 1_000),
        }
    }
}

impl<'de> DeserializeWithContext<'de, SqlSerdeConfig> for Time {
    fn deserialize_with_context<D>(
        deserializer: D,
        config: &'de SqlSerdeConfig,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match config.time_format {
            TimeFormat::String(format) => {
                let str: &'de str = Deserialize::deserialize(deserializer)?;
                let time = NaiveTime::parse_from_str(str.trim(), format)
                    .map_err(|e| D::Error::custom(format!("invalid time string '{str}': {e}")))?;
                Ok(Self::from_time(time))
            }
            TimeFormat::Micros => Ok(Self {
                nanoseconds: u64::deserialize(deserializer)? * 1_000,
            }),
        }
    }
}

impl<'de> Deserialize<'de> for Time {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str: &'de str = Deserialize::deserialize(deserializer)?;
        let time = NaiveTime::parse_from_str(str.trim(), "%H:%M:%S%.f")
            .map_err(|e| D::Error::custom(format!("invalid time string '{str}': {e}")))?;
        Ok(Self::from_time(time))
    }
}

some_operator!(lt, Time, Time, bool);
some_operator!(gt, Time, Time, bool);
some_operator!(eq, Time, Time, bool);
some_operator!(neq, Time, Time, bool);
some_operator!(gte, Time, Time, bool);
some_operator!(lte, Time, Time, bool);

pub fn extract_millisecond_Time(value: Time) -> i64 {
    let time = value.to_time();
    (time.second() * 1000 + time.nanosecond() / 1_000_000).into()
}

pub fn extract_microsecond_Time(value: Time) -> i64 {
    let time = value.to_time();
    (time.second() * 1_000_000 + time.nanosecond() / 1000).into()
}

pub fn extract_second_Time(value: Time) -> i64 {
    let time = value.to_time();
    time.second().into()
}

pub fn extract_minute_Time(value: Time) -> i64 {
    let time = value.to_time();
    time.minute().into()
}

pub fn extract_hour_Time(value: Time) -> i64 {
    let time = value.to_time();
    time.hour().into()
}

#[cfg(test)]
mod test {
    use super::{Date, Time, Timestamp};
    use dbsp_adapters::{
        deserialize_table_record, serialize_table_record, DeserializeWithContext,
        SerializeWithContext, SqlSerdeConfig,
    };
    use lazy_static::lazy_static;
    use pipeline_types::format::json::JsonFlavor;

    #[derive(Debug, Eq, PartialEq)]
    #[allow(non_snake_case)]
    struct TestStruct {
        date: Date,
        time: Time,
        timestamp: Timestamp,
    }

    deserialize_table_record!(TestStruct["TestStruct", 3] {
        (date, "DATE", false, Date, None),
        (time, "TIME", false, Time, None),
        (timestamp, "TIMESTAMP", false, Timestamp, None)
    });
    serialize_table_record!(TestStruct[3] {
        date["DATE"]: Date,
        time["TIME"]: Time,
        timestamp["TIMESTAMP"]:  Timestamp
    });

    lazy_static! {
        static ref DEFAULT_CONFIG: SqlSerdeConfig = SqlSerdeConfig::default();
        static ref DEBEZIUM_CONFIG: SqlSerdeConfig =
            SqlSerdeConfig::from(JsonFlavor::DebeziumMySql);
        static ref SNOWFLAKE_CONFIG: SqlSerdeConfig = SqlSerdeConfig::from(JsonFlavor::Snowflake);
    }

    fn deserialize_with_default_config<'de, T>(json: &'de str) -> Result<T, serde_json::Error>
    where
        T: DeserializeWithContext<'de, SqlSerdeConfig>,
    {
        T::deserialize_with_context(
            &mut serde_json::Deserializer::from_str(json),
            &DEFAULT_CONFIG,
        )
    }

    fn deserialize_with_debezium_config<'de, T>(json: &'de str) -> Result<T, serde_json::Error>
    where
        T: DeserializeWithContext<'de, SqlSerdeConfig>,
    {
        T::deserialize_with_context(
            &mut serde_json::Deserializer::from_str(json),
            &DEBEZIUM_CONFIG,
        )
    }

    fn serialize_with_default_config<'de, T>(val: &T) -> Result<String, serde_json::Error>
    where
        T: SerializeWithContext<SqlSerdeConfig>,
    {
        let mut data = Vec::new();
        val.serialize_with_context(&mut serde_json::Serializer::new(&mut data), &DEFAULT_CONFIG)
            .unwrap();
        Ok(String::from_utf8(data).unwrap())
    }

    fn serialize_with_snowflake_config<'de, T>(val: &T) -> Result<String, serde_json::Error>
    where
        T: SerializeWithContext<SqlSerdeConfig>,
    {
        let mut data = Vec::new();
        val.serialize_with_context(
            &mut serde_json::Serializer::new(&mut data),
            &SNOWFLAKE_CONFIG,
        )
        .unwrap();
        Ok(String::from_utf8(data).unwrap())
    }

    #[test]
    fn debezium() {
        assert_eq!(
            deserialize_with_debezium_config::<TestStruct>(
                r#"{"date": 16816, "time": 10800000000, "timestamp": "2018-06-20T13:37:03Z"}"#
            )
            .unwrap(),
            TestStruct {
                date: Date::new(16816),
                time: Time::new(10800000000000),
                timestamp: Timestamp::new(1529501823000),
            }
        );
    }

    #[test]
    fn snowflake_json() {
        assert_eq!(
            serialize_with_snowflake_config(&TestStruct {
                date: Date::new(19628),
                time: Time::new(84075123000000),
                timestamp: Timestamp::new(1529501823000),
            })
            .unwrap(),
            r#"{"DATE":"2023-09-28","TIME":"23:21:15.123","TIMESTAMP":"2018-06-20T13:37:03+00:00"}"#
        );
    }

    #[test]
    fn default_json() {
        assert_eq!(
            deserialize_with_default_config::<TestStruct>(
                r#"{"date": "2023-09-28", "time": "23:21:15.123", "timestamp": "2018-06-20 13:37:03"}"#
            )
            .unwrap(),
            TestStruct {
                date: Date::new(19628),
                time: Time::new(84075123000000),
                timestamp: Timestamp::new(1529501823000),
            }
        );

        assert_eq!(
            serialize_with_default_config(&TestStruct {
                date: Date::new(19628),
                time: Time::new(84075123000000),
                timestamp: Timestamp::new(1529501823000),
            })
            .unwrap(),
            r#"{"DATE":"2023-09-28","TIME":"23:21:15.123","TIMESTAMP":"2018-06-20 13:37:03"}"#
        );
    }
}

num_entries_scalar! {
    Timestamp,
    Date,
    Time,
}
