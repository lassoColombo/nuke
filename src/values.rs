//! The typed values a column can hold, and the readers that produce them.
//!
//! Everything here is pure and infallible: absent and unparseable map to
//! `None`, which a column shows as `nothing`. The Rust type decides the
//! nushell type:
//!
//! - `&str` / `String`              → `string`
//! - `i64`                           → `int` (counts; CPU in millicores)
//! - `f64`                           → `float`
//! - `bool`                          → `bool`
//! - [`Date`] (RFC 3339 timestamp)   → `date`
//! - [`Duration`] (Go duration)      → `duration`
//! - `Filesize` (memory quantity)    → `filesize`
//! - `Option<T>`                     → `T`, or `nothing` when `None`
//! - `Vec<T>`                        → `list`
//! - [`StrMap`]                      → `record` of strings
//! - `serde_json::Value`             → the JSON's own shape
//!
//! The `json_*` walkers navigate a `serde_json::Value` by a path of keys;
//! the `meta_*` readers are the `metadata.*` fields every object has; the
//! `parse_*` functions are the one place that decides which nushell type a
//! Kubernetes quantity, duration or timestamp becomes.

use nu_protocol::{Filesize, IntoValue, Record, Span, Value};
use serde_json::Value as Json;

/// Two rows as one record: the columns of the first, then the second's.
pub struct Concat<A, B>(pub A, pub B);

impl<A: IntoValue, B: IntoValue> IntoValue for Concat<A, B> {
    fn into_value(self, span: Span) -> Value {
        let mut rec = Record::new();
        for row in [self.0.into_value(span), self.1.into_value(span)] {
            if let Ok(cols) = row.into_record() {
                rec.extend(cols);
            }
        }
        Value::record(rec, span)
    }
}

// ---------------------------------------------------------------------------
// JSON walkers
// ---------------------------------------------------------------------------

/// Walk a path of keys into a `serde_json::Value`.
///
/// Returns `None` if any segment is missing or the final node is JSON null.
///
/// ```text
/// json_at(&item.data, &["status", "phase"])
/// json_at(&container, &["state", "waiting", "reason"])
/// ```
pub fn json_at<'a, const N: usize>(root: &'a Json, segments: &[&str; N]) -> Option<&'a Json> {
    let mut cur = root;
    for seg in segments {
        cur = cur.get(seg)?;
    }
    if cur.is_null() { None } else { Some(cur) }
}

/// The string at `path`, or `None` when absent or not a string. An empty
/// string stays `Some("")`: the column form for the fields where `""` and
/// unset differ (a claim's `storageClassName: ""` disables dynamic
/// provisioning). Most string columns want [`json_str_nonempty`].
pub fn json_str<'a, const N: usize>(root: &'a Json, path: &[&str; N]) -> Option<&'a str> {
    json_at(root, path).and_then(|v| v.as_str())
}

/// The string at `path`, `None` when absent or empty: the form of a string
/// column. Folding `""` into nothing is right for most fields, whose Go
/// type has `omitempty`.
pub fn json_str_nonempty<'a, const N: usize>(root: &'a Json, path: &[&str; N]) -> Option<&'a str> {
    json_str(root, path).filter(|s| !s.is_empty())
}

/// The integer at `path`, or `None` when absent or not an integer. A field
/// the API defaults supplies its default:
///
/// ```rust
/// json_i64(data, &["spec", "replicas"]).unwrap_or(1)
/// json_i64(data, &["status", "replicas"]).unwrap_or(0)
/// ```
pub fn json_i64<const N: usize>(root: &Json, path: &[&str; N]) -> Option<i64> {
    json_at(root, path).and_then(|v| v.as_i64())
}

/// The boolean at `path`, or `None` when absent or not a boolean. A field
/// the API defaults supplies its default:
///
/// ```rust
/// json_bool(data, &["spec", "paused"]).unwrap_or(false)
/// json_bool(data, &["spec", "attachRequired"]).unwrap_or(true)
/// ```
pub fn json_bool<const N: usize>(root: &Json, path: &[&str; N]) -> Option<bool> {
    json_at(root, path).and_then(|v| v.as_bool())
}

/// The array at `path`, or an empty slice when absent or not an array.
pub fn json_array<'a, const N: usize>(root: &'a Json, path: &[&str; N]) -> &'a [Json] {
    json_at(root, path)
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

/// A map of strings → `record`, one string column per key: labels,
/// annotations, a flat selector, a `matchLabels`. Empty when absent. Keys come
/// out sorted, as `serde_json` is built without `preserve_order`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StrMap<'a>(pub Vec<(&'a str, &'a str)>);

impl IntoValue for StrMap<'_> {
    fn into_value(self, span: Span) -> Value {
        let rec = self
            .0
            .into_iter()
            .map(|(k, v)| (k.to_string(), Value::string(v, span)))
            .collect();
        Value::record(rec, span)
    }
}

/// A timestamp column → `date`, held in UTC as the apiserver writes it.
/// A newtype because `IntoValue` is not ours. Ordered, so
/// `Option<Date>::max` picks the latest and `a.or(b)` the first present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date(pub chrono::DateTime<chrono::Utc>);

impl IntoValue for Date {
    fn into_value(self, span: Span) -> Value {
        Value::date(self.0.fixed_offset(), span)
    }
}

/// A duration column in nanoseconds → `duration`. Unlike
/// `std::time::Duration` it may be negative, as nushell's and Go's may.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(pub i64);

impl IntoValue for Duration {
    fn into_value(self, span: Span) -> Value {
        Value::duration(self.0, span)
    }
}

/// Parse an RFC 3339 timestamp (`2026-01-02T03:04:05Z`), or `None` on
/// failure; any offset is kept as the instant it names.
pub fn parse_date(s: &str) -> Option<Date> {
    s.parse::<chrono::DateTime<chrono::FixedOffset>>()
        .ok()
        .map(|t| Date(t.to_utc()))
}

/// Parse a Kubernetes memory quantity (`"256Mi"`, `"1Gi"`, `"512k"`, `"2G"`,
/// `"1000m"`) to a filesize in whole bytes, a sub-byte remainder rounding
/// down; `None` for an empty or unparseable string. A quantity beyond `i64`
/// saturates.
pub fn parse_memory(s: &str) -> Option<Filesize> {
    parse_quantity(s).map(|bytes| Filesize::new(bytes as i64))
}

/// Parse a Kubernetes CPU quantity (`"500m"`, `"2"`, `"0.5"`, `"100u"`,
/// `"1500000n"`) to millicores; `None` for an empty or unparseable string.
///
/// A fraction of a millicore rounds **up**, as `Quantity.MilliValue()` does
/// (and so `kubectl top`): `999999n` is 1 millicore, not 0. The product is
/// rounded to a nanocore first so `1000000n` stays exactly 1.
pub fn parse_cpu(s: &str) -> Option<i64> {
    let millis = parse_quantity(s)? * 1_000.0;
    Some(((millis * 1e6).round() / 1e6).ceil() as i64)
}

/// Parse a Kubernetes / Go duration string ("8760h", "15m", "1h30m");
/// `None` for an empty or unparseable string.
///
/// Supports the Go `time.Duration` format used by cert-manager and others:
/// hours (`h`), minutes (`m`), seconds (`s`), milliseconds (`ms`),
/// microseconds (`us`/`µs`), nanoseconds (`ns`), plus Prometheus' days (`d`),
/// weeks (`w`) and years (`y`, 365 days).
pub fn parse_duration(s: &str) -> Option<Duration> {
    duration_to_nanos(s).map(Duration)
}

/// The timestamp at `path`, or `None` when absent or unparseable.
pub fn json_date<const N: usize>(root: &Json, path: &[&str; N]) -> Option<Date> {
    json_str(root, path).and_then(parse_date)
}

/// A quantity node as text: the string as stored, or a bare JSON number
/// rendered (CRDs typed int-or-string keep a number a number).
fn quantity_text(node: Option<&Json>) -> String {
    match node {
        Some(Json::String(s)) => s.clone(),
        Some(Json::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// [`quantity_text`] of the quantity at `path`.
fn json_quantity_str<const N: usize>(root: &Json, path: &[&str; N]) -> Option<String> {
    json_at(root, path).map(|node| quantity_text(Some(node)))
}

/// The memory quantity at `path`, or `None`.
pub fn json_memory<const N: usize>(root: &Json, path: &[&str; N]) -> Option<Filesize> {
    parse_memory(&json_quantity_str(root, path)?)
}

/// The CPU quantity at `path` in millicores, or `None`.
pub fn json_cpu<const N: usize>(root: &Json, path: &[&str; N]) -> Option<i64> {
    parse_cpu(&json_quantity_str(root, path)?)
}

/// Parse a Kubernetes `Quantity` string to its value in base units
/// (bytes, cores, or a plain count).
///
/// Handles every suffix of the quantity grammar — binary (`Ki`..`Ei`),
/// decimal SI (`n`, `u`, `m`, `k`, `M`, `G`, `T`, `P`, `E`) and a decimal
/// exponent (`1e3`) — and fractional numbers (`"1.5Gi"`). Returns `None` for
/// an empty, negative or unparseable string.
fn parse_quantity(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    for (suffix, shift) in [
        ("Ki", 10u32),
        ("Mi", 20),
        ("Gi", 30),
        ("Ti", 40),
        ("Pi", 50),
        ("Ei", 60),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            return number(n).map(|v| v * (1u64 << shift) as f64);
        }
    }
    for (suffix, factor) in [
        ("n", 1e-9),
        ("u", 1e-6),
        ("m", 1e-3),
        ("k", 1e3),
        ("M", 1e6),
        ("G", 1e9),
        ("T", 1e12),
        ("P", 1e15),
        ("E", 1e18),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            return number(n).map(|v| v * factor);
        }
    }
    // A plain number, with or without an exponent.
    number(s)
}

/// The numeric part of a quantity: a non-negative decimal, possibly with an
/// exponent (`"1e3"`), as Kubernetes' `Quantity` grammar allows.
fn number(s: &str) -> Option<f64> {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
}

/// Parse a Go `time.Duration` string into nanoseconds.
///
/// The format is a sequence of decimal numbers each with a unit suffix:
/// `"1h30m"`, `"500ms"`, `"2h0m0s"`, `"8760h"`.
///
/// Recognised units: `ns`, `us`/`µs`, `ms`, `s`, `m`, `h`, plus the `d`,
/// `w` and `y` (365 days) of Prometheus durations — a strict superset of
/// Go's; a leading `-` negates the whole duration and a leading `+` is
/// allowed, as in Go.
fn duration_to_nanos(s: &str) -> Option<i64> {
    // At most one sign, as in Go: `--1h` is no duration.
    let s = s.trim();
    match s.strip_prefix('-') {
        Some(magnitude) => unsigned_nanos(magnitude)?.checked_neg(),
        None => unsigned_nanos(s.strip_prefix('+').unwrap_or(s)),
    }
}

/// [`duration_to_nanos`] without its sign; `None` when the total does not
/// fit an `i64`, as Go rejects an overflowing duration.
fn unsigned_nanos(s: &str) -> Option<i64> {
    // Go accepts a bare zero without a unit.
    if s == "0" {
        return Some(0);
    }
    if s.is_empty() {
        return None;
    }
    let mut total: i64 = 0;
    let mut remaining = s;

    while !remaining.is_empty() {
        // Parse the numeric part (integer or float).
        let num_end = remaining
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(remaining.len());
        if num_end == 0 {
            return None; // no digits before unit
        }
        let num: f64 = remaining[..num_end].parse().ok()?;
        remaining = &remaining[num_end..];

        // Parse the unit suffix.
        let (nanos_per_unit, unit_len) = if remaining.starts_with("ns") {
            (1i64, 2)
        } else if remaining.starts_with("us") || remaining.starts_with("µs") {
            (1_000i64, if remaining.starts_with("µ") { 3 } else { 2 })
        } else if remaining.starts_with("ms") {
            (1_000_000i64, 2)
        } else if remaining.starts_with('s') {
            (1_000_000_000i64, 1)
        } else if remaining.starts_with('m') {
            (60_000_000_000i64, 1)
        } else if remaining.starts_with('h') {
            (3_600_000_000_000i64, 1)
        } else if remaining.starts_with('d') {
            (86_400_000_000_000i64, 1)
        } else if remaining.starts_with('w') {
            (604_800_000_000_000i64, 1)
        } else if remaining.starts_with('y') {
            (31_536_000_000_000_000i64, 1)
        } else {
            return None; // unknown unit
        };

        let nanos = num * nanos_per_unit as f64;
        if nanos >= i64::MAX as f64 {
            return None;
        }
        total = total.checked_add(nanos as i64)?;
        remaining = &remaining[unit_len..];
    }

    Some(total)
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

/// `metadata.name`, `""` when the object has none.
pub fn meta_name(object: &Json) -> &str {
    json_str(object, &["metadata", "name"]).unwrap_or("")
}

/// `metadata.namespace`, or `None` for a cluster-scoped object.
pub fn meta_namespace(object: &Json) -> Option<&str> {
    json_str_nonempty(object, &["metadata", "namespace"])
}

/// `metadata.creationTimestamp`.
pub fn meta_created(object: &Json) -> Option<Date> {
    json_date(object, &["metadata", "creationTimestamp"])
}

/// `metadata.labels`, one string column per label; empty when none.
pub fn meta_labels(object: &Json) -> StrMap<'_> {
    str_map(json_at(object, &["metadata", "labels"]))
}

/// `metadata.annotations`, one string column per annotation; empty when
/// none.
pub fn meta_annotations(object: &Json) -> StrMap<'_> {
    str_map(json_at(object, &["metadata", "annotations"]))
}

fn str_map(node: Option<&Json>) -> StrMap<'_> {
    StrMap(
        node.and_then(|v| v.as_object())
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| Some((k.as_str(), v.as_str()?)))
            .collect(),
    )
}

/// The controlling `ownerReference` (the `metadata.ownerReferences[]`
/// entry with `controller: true`, of which the API allows one) as
/// `"<lowercase-kind>/<name>"` (`"replicaset/my-rs-xz4f"`), or `None` when
/// there is no controller owner.
pub fn meta_owner(object: &Json) -> Option<String> {
    json_array(object, &["metadata", "ownerReferences"])
        .iter()
        .find(|r| json_bool(r, &["controller"]).unwrap_or(false))
        .map(|r| {
            format!(
                "{}/{}",
                json_str(r, &["kind"]).unwrap_or("").to_lowercase(),
                json_str(r, &["name"]).unwrap_or("")
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(duration_to_nanos("0"), Some(0));
        assert_eq!(duration_to_nanos("1h30m"), Some(5_400_000_000_000));
        assert_eq!(duration_to_nanos("500ms"), Some(500_000_000));
        assert_eq!(duration_to_nanos("1.5s"), Some(1_500_000_000));
        assert_eq!(duration_to_nanos("2µs"), Some(2_000));
        assert_eq!(duration_to_nanos("-1h"), Some(-3_600_000_000_000));
        assert_eq!(duration_to_nanos("+1s"), Some(1_000_000_000));
        assert_eq!(duration_to_nanos("1d"), Some(86_400_000_000_000));
        assert_eq!(duration_to_nanos("2w"), Some(1_209_600_000_000_000));
        assert_eq!(duration_to_nanos("1y"), Some(31_536_000_000_000_000));
        assert_eq!(duration_to_nanos("10"), None);
        assert_eq!(duration_to_nanos("h"), None);
        // One sign only, and a value that does not fit is not a duration.
        assert_eq!(duration_to_nanos("--1h"), None);
        assert_eq!(duration_to_nanos("+-1h"), None);
        assert_eq!(duration_to_nanos("-+1h"), None);
        assert_eq!(duration_to_nanos("300y"), None);
        assert_eq!(duration_to_nanos("9999999999999h"), None);
        assert_eq!(duration_to_nanos("292y"), Some(9_208_512_000_000_000_000));
        let span = Span::test_data();
        assert_eq!(
            parse_duration("-1s").into_value(span),
            Value::duration(-1_000_000_000, span)
        );
        assert_eq!(parse_duration("").into_value(span), Value::nothing(span));
    }

    #[test]
    fn quantities() {
        let span = Span::test_data();
        assert_eq!(parse_memory("1Ki").map(|f| f.get()), Some(1_024));
        assert_eq!(parse_memory("1.5Gi").map(|f| f.get()), Some(1_610_612_736));
        assert_eq!(parse_memory("1k").map(|f| f.get()), Some(1_000));
        assert_eq!(parse_memory(""), None);
        assert_eq!(parse_memory("x"), None);
        assert_eq!(parse_cpu("999999n"), Some(1));
        assert_eq!(parse_cpu("1000000n"), Some(1));
        assert_eq!(parse_cpu("0.5"), Some(500));
        // A bare JSON number is a quantity too.
        let j = serde_json::json!({"storage": 10737418240u64, "pods": 110});
        assert_eq!(
            json_memory(&j, &["storage"]).into_value(span),
            Value::filesize(10_737_418_240i64, span)
        );
        assert_eq!(json_memory(&j, &["absent"]), None);
    }

    #[test]
    fn metadata() {
        let o = serde_json::json!({"metadata": {
            "name": "web-1", "namespace": "default",
            "creationTimestamp": "2026-01-02T03:04:05Z",
            "labels": {"app": "web", "n": 1},
            "ownerReferences": [
                {"kind": "Node", "name": "n1"},
                {"kind": "ReplicaSet", "name": "web", "controller": true}
            ]
        }});
        assert_eq!(meta_name(&o), "web-1");
        assert_eq!(meta_namespace(&o), Some("default"));
        assert!(meta_created(&o).is_some());
        // An offset names the same instant, shown in UTC as the apiserver
        // writes it.
        let span = Span::test_data();
        let utc = parse_date("2026-01-02T03:04:05Z");
        assert_eq!(parse_date("2026-01-02T05:04:05+02:00"), utc);
        assert_eq!(
            utc.into_value(span).as_date().unwrap().to_rfc3339(),
            "2026-01-02T03:04:05+00:00"
        );
        assert_eq!(parse_date("yesterday"), None);
        assert_eq!(meta_labels(&o), StrMap(vec![("app", "web")]));
        assert_eq!(meta_owner(&o).as_deref(), Some("replicaset/web"));
        assert_eq!(meta_namespace(&serde_json::json!({})), None);
    }
}
