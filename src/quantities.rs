//! Parsing and typed-value helpers for Kubernetes resource quantity strings.
//!
//! These are used by both `kube top` (metrics quantities) and anywhere else
//! the API returns CPU / memory / duration strings.

use chrono::DateTime;
use nu_protocol::{Span, Value};

// ---------------------------------------------------------------------------
// CPU
// ---------------------------------------------------------------------------

/// Parse a Kubernetes CPU quantity string into millicores.
///
/// Kubernetes emits three possible formats:
///   "2041180n" → nanocores  (ceiling-divided to millicores)
///   "250m"     → millicores (used as-is)
///   "1.5"      → cores      (multiplied by 1000)
///
/// Ceiling division is used for nanocores so that a container using e.g.
/// 1500n reports 1m rather than 0m — consistent with kubectl top behaviour.
pub fn parse_cpu_to_millicores(s: &str) -> u64 {
    if let Some(nc) = s.strip_suffix('n') {
        let nanocores: u64 = nc.parse().unwrap_or(0);
        (nanocores + 999_999) / 1_000_000
    } else if let Some(mc) = s.strip_suffix('m') {
        mc.parse().unwrap_or(0)
    } else {
        (s.parse::<f64>().unwrap_or(0.0) * 1000.0) as u64
    }
}

// ---------------------------------------------------------------------------
// Memory
// ---------------------------------------------------------------------------

/// Parse a Kubernetes memory quantity string into bytes.
///
/// Supports binary (Ki, Mi, Gi, Ti) and SI (K, M, G, T) suffixes.
/// Binary suffixes are listed first so that "Ki" is never mis-stripped as "K".
pub fn parse_memory_to_bytes(s: &str) -> u64 {
    const UNITS: &[(&str, u64)] = &[
        ("Ti", 1024u64 * 1024 * 1024 * 1024),
        ("Gi", 1024u64 * 1024 * 1024),
        ("Mi", 1024u64 * 1024),
        ("Ki", 1024u64),
        ("T", 1000u64 * 1000 * 1000 * 1000),
        ("G", 1000u64 * 1000 * 1000),
        ("M", 1000u64 * 1000),
        ("K", 1000u64),
    ];

    for &(suffix, factor) in UNITS {
        if let Some(num) = s.strip_suffix(suffix) {
            return num.parse::<u64>().unwrap_or(0) * factor;
        }
    }

    s.parse().unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Duration / window
// ---------------------------------------------------------------------------

/// Parse a Kubernetes metrics window string into nanoseconds (Nu duration unit).
///
/// Handles:
///   "28.727s"  → pure-seconds fast path
///   "1m30.5s"  → minutes + seconds
///   "2m"       → minutes only
pub fn parse_window_to_ns(s: &str) -> i64 {
    // Fast path: pure seconds with optional decimal.
    if let Some(secs_str) = s.strip_suffix('s') {
        if !secs_str.contains('m') {
            if let Ok(secs) = secs_str.parse::<f64>() {
                return (secs * 1_000_000_000.0) as i64;
            }
        }
    }

    // General "XmY.Zs" or "Xm" parser.
    let mut remaining = s;
    let mut total_ns: i64 = 0;

    if let Some(pos) = remaining.find('m') {
        let mins: i64 = remaining[..pos].parse().unwrap_or(0);
        total_ns += mins * 60 * 1_000_000_000;
        remaining = &remaining[pos + 1..];
    }

    if let Some(secs_str) = remaining.strip_suffix('s') {
        if let Ok(secs) = secs_str.parse::<f64>() {
            total_ns += (secs * 1_000_000_000.0) as i64;
        }
    }

    total_ns
}

// ---------------------------------------------------------------------------
// Timestamp
// ---------------------------------------------------------------------------

/// Parse an RFC 3339 timestamp string into a Nu date Value, or `nothing` on failure.
pub fn parse_timestamp(s: &str, span: Span) -> Value {
    match DateTime::parse_from_rfc3339(s) {
        Ok(dt) => Value::date(dt.fixed_offset(), span),
        Err(_) => Value::nothing(span),
    }
}

// ---------------------------------------------------------------------------
// Percentage
// ---------------------------------------------------------------------------

/// Return `used / total * 100.0` as a float Value, or `nothing` if total == 0.
pub fn pct_value(used: u64, total: u64, span: Span) -> Value {
    if total == 0 {
        Value::nothing(span)
    } else {
        Value::float(used as f64 * 100.0 / total as f64, span)
    }
}
