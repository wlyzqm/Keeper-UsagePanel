use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use serde::Serialize;
use serde_json::Value;

pub fn n(v: &Value, key: &str) -> i64 {
    v[key].as_i64().unwrap_or(0)
}
pub fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
pub fn rows<'a>(v: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    v[key].as_array().into_iter().flatten()
}
pub fn beijing() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).unwrap()
}
#[derive(Clone, Debug)]
pub struct Reading {
    pub day: NaiveDate,
    pub input: i64,
    pub output: i64,
    pub at: DateTime<Utc>,
}
#[derive(Default, Debug, Serialize)]
pub struct Totals {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub reasoning_tokens: i64,
}
pub fn totals(data: &Value) -> Totals {
    let mut sum = Totals::default();
    for row in rows(data, "token_usage") {
        sum.input_tokens += n(row, "input_tokens");
        sum.output_tokens += n(row, "output_tokens");
        sum.total_tokens += n(row, "total_tokens");
        sum.cache_read_tokens += n(row, "cache_read_tokens");
        sum.cache_creation_tokens += n(row, "cache_creation_tokens");
        sum.reasoning_tokens += n(row, "reasoning_tokens");
    }
    sum
}
#[derive(Debug, Serialize)]
pub struct Delta {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub seconds: f64,
    pub baseline: bool,
    pub reset: bool,
}
#[derive(Default)]
pub struct Counter {
    pub previous: Option<Reading>,
}
impl Counter {
    pub fn accept(&mut self, current: Reading, closed: Option<Totals>) -> Delta {
        let previous = self.previous.replace(current.clone());
        let empty = |reset| Delta {
            input_tokens: 0,
            output_tokens: 0,
            seconds: 0.,
            baseline: true,
            reset,
        };
        let Some(previous) = previous else {
            return empty(false);
        };
        let (input, output) = if current.day == previous.day {
            (
                current.input - previous.input,
                current.output - previous.output,
            )
        } else if current.day > previous.day {
            let Some(c) = closed else { return empty(true) };
            (
                c.input_tokens - previous.input + current.input,
                c.output_tokens - previous.output + current.output,
            )
        } else {
            return empty(true);
        };
        let seconds = (current.at - previous.at).num_milliseconds() as f64 / 1000.;
        if input < 0 || output < 0 || seconds < 0. {
            return empty(true);
        }
        Delta {
            input_tokens: input,
            output_tokens: output,
            seconds,
            baseline: false,
            reset: false,
        }
    }
}
pub fn health(success: i64, failure: i64) -> &'static str {
    let total = success + failure;
    if total == 0 {
        return "安静";
    }
    if failure > success {
        return "异常";
    }
    let threshold = 0.99_f64.min(0.9 + 0.045 * ((total as f64 / 10.).log10()).max(0.));
    if success as f64 / total as f64 >= threshold {
        "健康"
    } else {
        "波动"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reading(day: u32, input: i64, output: i64, seconds: i64) -> Reading {
        Reading {
            day: NaiveDate::from_ymd_opt(2026, 8, day).unwrap(),
            input,
            output,
            at: DateTime::from_timestamp(seconds, 0).unwrap(),
        }
    }
    #[test]
    fn interval_and_zero() {
        let mut c = Counter::default();
        assert!(c.accept(reading(26, 100, 40, 100), None).baseline);
        let d = c.accept(reading(26, 125, 49, 102), None);
        assert_eq!((d.input_tokens, d.output_tokens, d.seconds), (25, 9, 2.));
        let d = c.accept(reading(26, 125, 49, 104), None);
        assert_eq!((d.input_tokens, d.output_tokens), (0, 0));
    }
    #[test]
    fn midnight_bridge() {
        let mut c = Counter::default();
        c.accept(reading(26, 100, 40, 100), None);
        let d = c.accept(
            reading(27, 8, 2, 106),
            Some(Totals {
                input_tokens: 110,
                output_tokens: 50,
                ..Default::default()
            }),
        );
        assert_eq!((d.input_tokens, d.output_tokens, d.seconds), (18, 12, 6.));
    }
    #[test]
    fn reset_and_missing_bridge() {
        let mut c = Counter::default();
        c.accept(reading(26, 100, 40, 100), None);
        assert!(c.accept(reading(26, 50, 10, 102), None).reset);
        assert!(c.accept(reading(27, 5, 1, 104), None).reset);
        assert!(c.accept(reading(26, 8, 2, 106), None).reset);
    }
    #[test]
    fn long_precision() {
        let mut c = Counter::default();
        c.accept(reading(26, 9007199254740992, 40, 100), None);
        assert_eq!(
            c.accept(reading(26, 9007199254740993, 41, 102), None)
                .input_tokens,
            1
        );
    }
    #[test]
    fn keeper_health_formula() {
        assert_eq!(health(0, 0), "安静");
        assert_eq!(health(1, 2), "异常");
        assert_eq!(health(9, 1), "健康");
        assert_eq!(health(90, 10), "波动");
        assert_eq!(health(99, 1), "健康");
    }
}
