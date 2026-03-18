//! Shared extraction and conversion helpers for all resource formatters.
//!
//! # Design principles
//!
//! - Every function is pure and infallible (never panics, uses sensible defaults).
//! - Nushell types are used as precisely as possible:
//!     - plain string            → `Value::string`
//!     - RFC 3339 timestamp      → `Value::date`   (chrono `DateTime<FixedOffset>`)
//!     - k8s duration ("15m")    → `Value::duration` (nanoseconds i64)
//!     - memory quantity ("512Mi") → `Value::filesize` (bytes i64)
//!     - CPU quantity ("250m")   → `Value::int`    (millicores i64)
//!     - integer                 → `Value::int`
//!     - boolean                 → `Value::bool`
//!     - absent / unparseable    → `Value::nothing`
//!
//! # Layers
//!
//! 1. **JSON walkers** (`json_str`, `json_i64`, …) — replace the scattered
//!    `item.data["a"]["b"].as_str().unwrap_or_default()` pattern.
//!    The dot-path API (`"status.phase"`) is readable and uniform.
//!
//! 2. **Metadata helpers** (`meta_name`, `meta_namespace`, …) — typed `Value`
//!    wrappers around `kube::ResourceExt` for the fields every resource has.
//!
//! 3. **Quantity converters** (`parse_date`, `parse_duration`, `parse_memory`,
//!    `parse_cpu`, `pct`) — the single authoritative place that decides which
//!    nushell type a Kubernetes quantity becomes.

use chrono::DateTime;
use kube::api::DynamicObject;
use kube::ResourceExt;
use nu_protocol::{Span, Value};
use serde_json::Value as Json;

// ---------------------------------------------------------------------------
// 1. Low-level JSON walkers
// ---------------------------------------------------------------------------

/// Walk a **dot-separated path** into a `serde_json::Value`.
///
/// Returns `None` if any segment is missing or the final node is JSON null.
///
/// ```text
/// json_at(&item.data, "status.phase")
/// json_at(&container, "state.waiting.reason")
/// ```
pub fn json_at<'a>(root: &'a Json, path: &str) -> Option<&'a Json> {
    let mut cur = root;
    for seg in path.split('.') {
        cur = cur.get(seg)?;
    }
    if cur.is_null() {
        None
    } else {
        Some(cur)
    }
}

/// Extract a `&str` from a dot-path, falling back to `""`.
pub fn json_str<'a>(root: &'a Json, path: &str) -> &'a str {
    json_at(root, path).and_then(|v| v.as_str()).unwrap_or("")
}

/// Extract an `i64` from a dot-path, falling back to `0`.
pub fn json_i64(root: &Json, path: &str) -> i64 {
    json_at(root, path).and_then(|v| v.as_i64()).unwrap_or(0)
}

/// Extract a `bool` from a dot-path, falling back to `false`.
pub fn json_bool(root: &Json, path: &str) -> bool {
    json_at(root, path)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Extract a JSON array from a dot-path, falling back to an empty slice.
pub fn json_array<'a>(root: &'a Json, path: &str) -> &'a [Json] {
    json_at(root, path)
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

// ---------------------------------------------------------------------------
// 2. Typed metadata helpers  (DynamicObject → Value)
// ---------------------------------------------------------------------------

/// `metadata.name` → `Value::string`.
pub fn meta_name(item: &DynamicObject, span: Span) -> Value {
    Value::string(item.name_any(), span)
}

/// `metadata.namespace` → `Value::string` (`""` for cluster-scoped resources).
pub fn meta_namespace(item: &DynamicObject, span: Span) -> Value {
    Value::string(item.namespace().unwrap_or_default(), span)
}

/// `metadata.creationTimestamp` → `Value::date`, or `Value::nothing`.
///
/// Storing the absolute timestamp lets nushell users compute the age
/// themselves: `$r.created | into duration` gives a human-readable age.
/// We name the column `created` to make it clear it is not a computed age.
pub fn meta_created(item: &DynamicObject, span: Span) -> Value {
    let Some(ts) = item.creation_timestamp() else {
        return Value::nothing(span);
    };
    let secs = ts.0.as_second();
    let nanos = ts.0.subsec_nanosecond() as u32;
    match DateTime::from_timestamp(secs, nanos) {
        Some(utc) => Value::date(utc.fixed_offset(), span),
        None => Value::nothing(span),
    }
}

/// `metadata.labels` → `Value::record` (one string column per label).
/// Returns an empty record when there are no labels.
pub fn meta_labels(item: &DynamicObject, span: Span) -> Value {
    let mut rec = nu_protocol::Record::new();
    for (k, v) in item.labels() {
        rec.push(k.clone(), Value::string(v.clone(), span));
    }
    Value::record(rec, span)
}

/// `metadata.annotations` → `Value::record` (one string column per annotation).
/// Returns an empty record when there are no annotations.
pub fn meta_annotations(item: &DynamicObject, span: Span) -> Value {
    let mut rec = nu_protocol::Record::new();
    for (k, v) in item.annotations() {
        rec.push(k.clone(), Value::string(v.clone(), span));
    }
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// 3. Quantity converters  (string → typed Value)
// ---------------------------------------------------------------------------

/// Parse an RFC 3339 / ISO 8601 timestamp string → `Value::date`.
/// Returns `Value::nothing` on failure.
pub fn parse_date(s: &str, span: Span) -> Value {
    match DateTime::parse_from_rfc3339(s) {
        Ok(dt) => Value::date(dt, span),
        Err(_) => Value::nothing(span),
    }
}

/// Parse a Kubernetes / Go duration string ("15m", "2h30m", "300s", "1h2m3s")
/// → `Value::duration` (nanoseconds i64).
/// Returns `Value::nothing` on failure.
pub fn parse_duration(s: &str, span: Span) -> Value {
    match parse_duration_to_ns(s) {
        Some(ns) => Value::duration(ns, span),
        None => Value::nothing(span),
    }
}

/// Parse a Kubernetes memory quantity ("256Mi", "1Gi", "512k", "2G")
/// → `Value::filesize` (bytes i64).
/// Returns `Value::nothing` for an empty string; `Value::filesize(0)` for
/// a valid zero quantity.
pub fn parse_memory(s: &str, span: Span) -> Value {
    if s.is_empty() {
        return Value::nothing(span);
    }
    Value::filesize(memory_to_bytes(s) as i64, span)
}

/// Parse a Kubernetes CPU quantity ("500m", "2", "0.5")
/// → `Value::int` (millicores i64).
/// Returns `Value::nothing` for an empty string.
pub fn parse_cpu(s: &str, span: Span) -> Value {
    if s.is_empty() {
        return Value::nothing(span);
    }
    Value::int(cpu_to_millicores(s) as i64, span)
}

/// Compute `used / total * 100` → `Value::float`.
/// Returns `Value::nothing` when `total` is zero.
pub fn pct(used: u64, total: u64, span: Span) -> Value {
    if total == 0 {
        Value::nothing(span)
    } else {
        Value::float((used as f64 / total as f64) * 100.0, span)
    }
}

// ---------------------------------------------------------------------------
// Internal quantity parsers (consumed by the public wrappers above)
// ---------------------------------------------------------------------------

/// Parse a Kubernetes memory quantity string to bytes.
pub(crate) fn memory_to_bytes(s: &str) -> u64 {
    // Binary suffixes
    for (suffix, shift) in [
        ("Ki", 10u32),
        ("Mi", 20),
        ("Gi", 30),
        ("Ti", 40),
        ("Pi", 50),
        ("Ei", 60),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            if let Ok(v) = n.trim().parse::<u64>() {
                return v << shift;
            }
        }
    }
    // Decimal suffixes
    for (suffix, factor) in [
        ("k", 1_000u64),
        ("M", 1_000_000),
        ("G", 1_000_000_000),
        ("T", 1_000_000_000_000),
        ("P", 1_000_000_000_000_000),
        ("E", 1_000_000_000_000_000_000),
    ] {
        if let Some(n) = s.strip_suffix(suffix) {
            if let Ok(v) = n.trim().parse::<u64>() {
                return v * factor;
            }
        }
    }
    s.trim().parse::<u64>().unwrap_or(0)
}

/// Parse a Kubernetes CPU quantity string to millicores.
pub(crate) fn cpu_to_millicores(s: &str) -> u64 {
    if let Some(n) = s.strip_suffix('m') {
        n.trim().parse::<u64>().unwrap_or(0)
    } else {
        s.trim()
            .parse::<f64>()
            .map(|f| (f * 1000.0) as u64)
            .unwrap_or(0)
    }
}

/// Parse a Go/Kubernetes duration string to nanoseconds.
pub(crate) fn parse_duration_to_ns(s: &str) -> Option<i64> {
    let mut total_ns: i64 = 0;
    let mut rest = s;
    while !rest.is_empty() {
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        if end == 0 {
            return None;
        }
        let n: i64 = rest[..end].parse().ok()?;
        rest = &rest[end..];
        let unit_end = rest
            .find(|c: char| c.is_ascii_digit())
            .unwrap_or(rest.len());
        if unit_end == 0 {
            return None;
        }
        let unit = &rest[..unit_end];
        rest = &rest[unit_end..];
        let ns_per_unit: i64 = match unit {
            "ns" => 1,
            "us" | "µs" => 1_000,
            "ms" => 1_000_000,
            "s" => 1_000_000_000,
            "m" => 60 * 1_000_000_000,
            "h" => 3_600 * 1_000_000_000,
            _ => return None,
        };
        total_ns += n * ns_per_unit;
    }
    Some(total_ns)
}
