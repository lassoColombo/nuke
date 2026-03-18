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
//!
//! 4. **Spec/status extractors** (`meta_owner`, `status_condition`,
//!    `spec_selector`, `spec_strategy`) — structured sub-object helpers shared
//!    across multiple resource formatters.
//!
//! 5. **Container helpers** (`fmt_containers`, `fmt_images`) — extract
//!    container metadata from `.spec.template.spec.containers`.

use chrono::DateTime;
use kube::api::DynamicObject;
use kube::ResourceExt;
use nu_protocol::{Record, Span, Value};
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
    let mut rec = Record::new();
    for (k, v) in item.labels() {
        rec.push(k.clone(), Value::string(v.clone(), span));
    }
    Value::record(rec, span)
}

/// `metadata.annotations` → `Value::record` (one string column per annotation).
/// Returns an empty record when there are no annotations.
pub fn meta_annotations(item: &DynamicObject, span: Span) -> Value {
    let mut rec = Record::new();
    for (k, v) in item.annotations() {
        rec.push(k.clone(), Value::string(v.clone(), span));
    }
    Value::record(rec, span)
}

/// Resolve the controlling `ownerReference` → `Value::string` (`"kind/name"`),
/// or `Value::nothing` when there is no controller owner.
///
/// Mirrors the Nushell `meta owner` helper: walks
/// `metadata.ownerReferences[]`, finds the entry where `controller == true`,
/// and formats it as `"<lowercase-kind>/<name>"` (e.g. `"replicaset/my-rs-xz4f"`).
/// Only the first controller reference is returned (there can be at most one
/// per the Kubernetes spec).
pub fn meta_owner(item: &DynamicObject, span: Span) -> Value {
    let refs = match item.owner_references() {
        refs if !refs.is_empty() => refs,
        _ => return Value::nothing(span),
    };

    let controller = refs.iter().find(|r| r.controller.unwrap_or(false));

    match controller {
        Some(r) => Value::string(format!("{}/{}", r.kind.to_lowercase(), r.name), span),
        None => Value::nothing(span),
    }
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

/// Parse a Kubernetes CPU quantity ("500m", "2", "0.5", "100u", "800n")
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
// 4. Spec / status extractors
// ---------------------------------------------------------------------------

/// Find the first `.status.conditions[]` entry whose `type` field equals
/// `cond_type` → `Value::record` with all condition fields as string columns.
///
/// Returns an empty record (not `Value::nothing`) when no match is found,
/// mirroring the Nushell `status condition` helper behaviour so callers can
/// always do `$r.conditions.Available.status` without a null-check.
///
/// Typical usage:
/// ```text
/// status_condition(&item.data, "Available", span)
/// status_condition(&item.data, "Ready",     span)
/// ```
pub fn status_condition(data: &Json, cond_type: &str, span: Span) -> Value {
    let condition = json_array(data, "status.conditions")
        .iter()
        .find(|c| json_str(c, "type") == cond_type);

    let obj = match condition {
        Some(c) => c,
        None => return Value::record(Record::new(), span),
    };

    // Materialise the condition object as a flat string record.
    // We handle the common fields explicitly so the column order is stable.
    let mut rec = Record::new();
    for key in &["type", "status", "reason", "message", "lastTransitionTime"] {
        let val = json_str(obj, key);
        rec.push(*key, Value::string(val, span));
    }
    Value::record(rec, span)
}

/// Extract `.spec.selector` → `Value::record`.
///
/// For label-selector based resources (Deployments, Services, DaemonSets …)
/// this returns the raw selector map as a flat string record so callers can
/// filter or display it.  Returns an empty record when the field is absent.
pub fn spec_selector(data: &Json, span: Span) -> Value {
    let selector = json_at(data, "spec.selector").and_then(|v| v.as_object());

    let mut rec = Record::new();
    if let Some(map) = selector {
        for (k, v) in map {
            let s = v.as_str().unwrap_or("");
            rec.push(k.clone(), Value::string(s, span));
        }
    }
    Value::record(rec, span)
}

/// Extract `.spec.strategy` → `Value::record { type, maxUnavailable, maxSurge }`.
///
/// Mirrors the Nushell `spec strategy` helper.  Fields that are absent in
/// the spec are stored as `Value::nothing` so wide formatters can display
/// them conditionally.
pub fn spec_strategy(data: &Json, span: Span) -> Value {
    let strategy_type = {
        let t = json_str(data, "spec.strategy.type");
        if t.is_empty() {
            "RollingUpdate"
        } else {
            t
        }
    };

    let max_unavailable = {
        let s = json_str(data, "spec.strategy.rollingUpdate.maxUnavailable");
        if s.is_empty() {
            Value::nothing(span)
        } else {
            Value::string(s, span)
        }
    };

    let max_surge = {
        let s = json_str(data, "spec.strategy.rollingUpdate.maxSurge");
        if s.is_empty() {
            Value::nothing(span)
        } else {
            Value::string(s, span)
        }
    };

    let mut rec = Record::new();
    rec.push("type", Value::string(strategy_type, span));
    rec.push("maxUnavailable", max_unavailable);
    rec.push("maxSurge", max_surge);
    Value::record(rec, span)
}

// ---------------------------------------------------------------------------
// 5. Container helpers
// ---------------------------------------------------------------------------

/// Build a `Value::record` for a single container JSON object.
///
/// ```text
/// { name, image, requests?: { cpu, memory }, limits?: { cpu, memory } }
/// ```
///
/// `requests` and `limits` are omitted entirely (not `nothing`) when absent,
/// matching the Nushell `container base` helper.
pub fn container_base(c: &Json, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push("name", Value::string(json_str(c, "name"), span));
    rec.push("image", Value::string(json_str(c, "image"), span));

    // resources — only include sub-record when the field is present
    if let Some(resources) = json_at(c, "resources") {
        if let Some(requests) = json_at(resources, "requests") {
            rec.push("requests", resources_record(requests, span));
        }
        if let Some(limits) = json_at(resources, "limits") {
            rec.push("limits", resources_record(limits, span));
        }
    }

    Value::record(rec, span)
}

/// Build a typed `{ cpu, memory }` record from a resources map.
fn resources_record(r: &Json, span: Span) -> Value {
    let mut rec = Record::new();
    rec.push("cpu", parse_cpu(json_str(r, "cpu"), span));
    rec.push("memory", parse_memory(json_str(r, "memory"), span));
    Value::record(rec, span)
}

/// Format a slice of container JSON objects → `Value::list` of
/// `container_base` records.
///
/// The caller is responsible for locating the right array with `json_array`:
///
/// ```rust
/// // Standard workload (Deployment, DaemonSet, StatefulSet, ReplicaSet, Job)
/// fmt_containers(json_array(data, "spec.template.spec.containers"), span)
///
/// // CronJob — template nested one level deeper
/// fmt_containers(json_array(data, "spec.jobTemplate.spec.template.spec.containers"), span)
/// ```
pub fn fmt_containers(containers: &[Json], span: Span) -> Value {
    Value::list(
        containers.iter().map(|c| container_base(c, span)).collect(),
        span,
    )
}

/// Format a slice of container JSON objects → `Value::list` of image strings.
///
/// Same calling convention as `fmt_containers`:
///
/// ```rust
/// fmt_images(json_array(data, "spec.template.spec.containers"), span)
/// ```
pub fn fmt_images(containers: &[Json], span: Span) -> Value {
    Value::list(
        containers
            .iter()
            .map(|c| Value::string(json_str(c, "image"), span))
            .collect(),
        span,
    )
}

// ---------------------------------------------------------------------------
// Internal quantity parsers (consumed by the public wrappers above)
// ---------------------------------------------------------------------------

/// Parse a Kubernetes memory quantity string to bytes.
pub(crate) fn memory_to_bytes(s: &str) -> u64 {
    // Binary suffixes (IEC)
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
    // Decimal suffixes (SI)
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
    // Plain integer bytes
    s.trim().parse::<u64>().unwrap_or(0)
}

/// Parse a Kubernetes CPU quantity string to millicores.
///
/// Handles all suffixes defined in the Kubernetes quantity spec:
///
/// | suffix | unit       | conversion         |
/// |--------|------------|--------------------|
/// | `m`    | millicores | value as-is        |
/// | *(none)* | cores    | × 1 000            |
/// | `u`    | microcores | ÷ 1 000 (round)    |
/// | `n`    | nanocores  | ÷ 1 000 000 (round)|
///
/// The Nushell `cvt-cpu` helper also handles `u` and `n`; the previous Rust
/// version silently returned 0 for those suffixes.
pub(crate) fn cpu_to_millicores(s: &str) -> u64 {
    if let Some(n) = s.strip_suffix('m') {
        // millicores — use directly
        n.trim().parse::<u64>().unwrap_or(0)
    } else if let Some(n) = s.strip_suffix('u') {
        // microcores → millicores  (1 mc = 1 000 uc, so ÷ 1 000)
        n.trim().parse::<u64>().map(|v| v / 1_000).unwrap_or(0)
    } else if let Some(n) = s.strip_suffix('n') {
        // nanocores → millicores  (1 mc = 1 000 000 nc, so ÷ 1 000 000)
        n.trim().parse::<u64>().map(|v| v / 1_000_000).unwrap_or(0)
    } else {
        // bare float/int — whole cores, multiply by 1 000
        s.trim()
            .parse::<f64>()
            .map(|f| (f * 1_000.0) as u64)
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- cpu_to_millicores ---

    #[test]
    fn cpu_millicore_suffix() {
        assert_eq!(cpu_to_millicores("500m"), 500);
        assert_eq!(cpu_to_millicores("1000m"), 1000);
    }

    #[test]
    fn cpu_whole_cores() {
        assert_eq!(cpu_to_millicores("2"), 2000);
        assert_eq!(cpu_to_millicores("0.5"), 500);
    }

    #[test]
    fn cpu_microcores() {
        // 1 000 000 u = 1 core = 1 000 mc
        assert_eq!(cpu_to_millicores("1000000u"), 1000);
        // 500 000 u = 0.5 core = 500 mc
        assert_eq!(cpu_to_millicores("500000u"), 500);
        // sub-millicore: 100u = 0.1 mc → rounds down to 0
        assert_eq!(cpu_to_millicores("100u"), 0);
    }

    #[test]
    fn cpu_nanocores() {
        // 1 000 000 000 n = 1 core = 1 000 mc
        assert_eq!(cpu_to_millicores("1000000000n"), 1000);
        // 500 000 000 n = 500 mc
        assert_eq!(cpu_to_millicores("500000000n"), 500);
        // sub-millicore: 1n → 0
        assert_eq!(cpu_to_millicores("1n"), 0);
    }

    #[test]
    fn cpu_empty() {
        // empty handled by parse_cpu wrapper, but cpu_to_millicores should not panic
        assert_eq!(cpu_to_millicores(""), 0);
    }

    // --- memory_to_bytes ---

    #[test]
    fn memory_mebibytes() {
        assert_eq!(memory_to_bytes("128Mi"), 128 * 1024 * 1024);
    }

    #[test]
    fn memory_gibibytes() {
        assert_eq!(memory_to_bytes("2Gi"), 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn memory_kilobytes_decimal() {
        assert_eq!(memory_to_bytes("1k"), 1_000);
    }

    #[test]
    fn memory_plain_bytes() {
        assert_eq!(memory_to_bytes("4096"), 4096);
    }
}
