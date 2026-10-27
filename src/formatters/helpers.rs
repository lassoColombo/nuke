//! Shared extraction and conversion helpers for all resource formatters.
//!
//! # Design principles
//!
//! - Every function is pure and infallible (never panics, uses sensible defaults).
//! - A formatter's columns are the fields of a `#[derive(IntoValue)]` struct
//!   handed to [`append`], so helpers return Rust types and the field's type
//!   decides the nushell type of its column:
//!     - `&str` / `String`              → `string`
//!     - `i64`                           → `int` (counts; CPU in millicores)
//!     - `f64`                           → `float`
//!     - `bool`                          → `bool`
//!     - [`Date`] (RFC 3339 timestamp)   → `date`
//!     - [`Duration`] (Go duration)      → `duration`
//!     - `Filesize` (memory quantity)    → `filesize`
//!     - [`Quantity`] (dimensionless)    → `int` when integral, else `float`
//!     - [`TypedQuantity`]               → by what it measures: one of the three above
//!     - [`IntOrString`]                 → `int` or `string`, as stored
//!     - `Option<T>`                     → `T`, or `nothing` when `None`
//!     - `Vec<T>`                        → `list`
//!     - [`StrMap`]                      → `record` of strings
//!     - a derived struct                → `record`
//!     - `serde_json::Value`             → the JSON's own shape
//!
//! # Layers
//!
//! 0. **Rows** ([`append`], [`Concat`]) — hand a derived row to a record.
//!
//! 1. **JSON walkers** (`json_str`, `json_i64`, …) — replace the scattered
//!    `item.data["a"]["b"].as_str().unwrap_or_default()` pattern with a
//!    path of segments: `json_str(&item.data, &["status", "phase"])`.
//!
//! 2. **Metadata helpers** (`meta_name`, `meta_namespace`, …) — typed
//!    wrappers around `kube::ResourceExt` for the fields every resource has.
//!
//! 3. **Quantity converters** (`parse_duration`, `parse_memory`, `parse_cpu`,
//!    `quantity`, `typed_quantity`) — the single authoritative place
//!    that decides which nushell type a Kubernetes quantity becomes — and
//!    their path-reading forms (`json_date`, `json_memory`, `json_cpu`,
//!    `json_quantity`, `json_duration`, `json_int_or_string`).
//!
//! 4. **Spec/status extractors** (`meta_owner`, `object_ref`,
//!    `condition_bool`, `label_selector`, `spec_selector`, `spec_strategy`,
//!    `status_conditions_list`) — structured sub-objects shared across
//!    multiple resource formatters.
//!
//! 5. **Container helpers** (`fmt_containers`, `fmt_images`, [`Resources`])
//!    — container specs and resource maps as records.
//!
//! 6. **Native passthrough** (`native_list`, `native_or_nothing`) — a JSON
//!    subtree as its own nushell shape.

use k8s_openapi::jiff;
use kube::ResourceExt;
use kube::api::DynamicObject;
use nu_protocol::{Filesize, IntoValue, Record, Span, Value};
use serde_json::Value as Json;

// ---------------------------------------------------------------------------
// 0. Rows
// ---------------------------------------------------------------------------

/// Append the columns of `row`, a `#[derive(IntoValue)]` struct (or a
/// [`Concat`] of them), to `rec` in field order: how `compact` and `wide`
/// hand over their columns.
///
/// The derive turns every field into a column, an `Option` field into
/// nothing when `None`, so a row's shape never depends on the object.
pub fn append(rec: &mut Record, row: impl IntoValue, span: Span) {
    match row.into_value(span).into_record() {
        Ok(cols) => rec.extend(cols),
        Err(_) => debug_assert!(false, "a row must be a struct deriving IntoValue"),
    }
}

/// Two rows as one record: the columns of the first, then the second's.
/// For a shared row a caller extends with its own columns (a Pod's
/// container rows add the runtime state to [`Container`]'s).
pub struct Concat<A, B>(pub A, pub B);

impl<A: IntoValue, B: IntoValue> IntoValue for Concat<A, B> {
    fn into_value(self, span: Span) -> Value {
        let mut rec = Record::new();
        append(&mut rec, self.0, span);
        append(&mut rec, self.1, span);
        Value::record(rec, span)
    }
}

// ---------------------------------------------------------------------------
// 1. Low-level JSON walkers
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

/// The length of the array at `path`, 0 when absent: the `<thing>Count`
/// compact column of an object list.
pub fn json_count<const N: usize>(root: &Json, path: &[&str; N]) -> i64 {
    json_array(root, path).len() as i64
}

/// The `name` of each element of the object array at `path` (a
/// ServiceAccount's `secrets`, a CRD's `spec.versions`).
pub fn json_name_list<'a, const N: usize>(root: &'a Json, path: &[&str; N]) -> Vec<&'a str> {
    json_array(root, path)
        .iter()
        .filter_map(|v| json_str(v, &["name"]))
        .collect()
}

/// The strings of the array at `path`. Non-string and null entries are
/// silently dropped; empty when the field is absent or not an array.
pub fn json_str_list<'a, const N: usize>(root: &'a Json, path: &[&str; N]) -> Vec<&'a str> {
    json_array(root, path)
        .iter()
        .filter_map(|v| v.as_str())
        .collect()
}

/// A map of strings → `record`, one string column per key, in the map's
/// order: labels, annotations, a flat selector, a `matchLabels`. Empty when
/// absent.
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

/// The JSON object of strings at `path` as a [`StrMap`] (a non-string value
/// becomes `""`); empty when the field is absent or not an object.
pub fn json_str_map<'a, const N: usize>(root: &'a Json, path: &[&str; N]) -> StrMap<'a> {
    StrMap(
        json_at(root, path)
            .and_then(|v| v.as_object())
            .into_iter()
            .flatten()
            .map(|(k, v)| (k.as_str(), v.as_str().unwrap_or("")))
            .collect(),
    )
}

/// Count the keys in a JSON object at `path`.
///
/// Returns `0` when the field is absent or not an object.
pub fn json_obj_key_count<const N: usize>(root: &Json, path: &[&str; N]) -> i64 {
    json_at(root, path)
        .and_then(|v| v.as_object())
        .map(|m| m.len() as i64)
        .unwrap_or(0)
}

/// The keys of a JSON object at `path`; empty when the field is absent or
/// not an object.
pub fn json_obj_keys<'a, const N: usize>(root: &'a Json, path: &[&str; N]) -> Vec<&'a str> {
    json_at(root, path)
        .and_then(|v| v.as_object())
        .map(|m| m.keys().map(String::as_str).collect())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// 2. Typed metadata helpers
// ---------------------------------------------------------------------------

/// `metadata.name`.
pub fn meta_name(item: &DynamicObject) -> String {
    item.name_any()
}

/// `metadata.namespace`, or `None` for a cluster-scoped object.
pub fn meta_namespace(item: &DynamicObject) -> Option<String> {
    item.namespace()
}

/// `metadata.creationTimestamp`.
pub fn meta_created(item: &DynamicObject) -> Option<Date> {
    item.creation_timestamp().map(|ts| Date(ts.0))
}

/// `metadata.labels`, one string column per label; empty when none.
pub fn meta_labels(item: &DynamicObject) -> StrMap<'_> {
    StrMap(
        item.labels()
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect(),
    )
}

/// `metadata.annotations`, one string column per annotation; empty when
/// none.
pub fn meta_annotations(item: &DynamicObject) -> StrMap<'_> {
    StrMap(
        item.annotations()
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect(),
    )
}

/// One `metadata.annotations` entry, or `None` when absent.
///
/// Annotations live on `item.metadata`, never in `item.data` — a
/// `json_*(&item.data, &["metadata", ..])` lookup can never match.
pub fn meta_annotation<'a>(item: &'a DynamicObject, key: &str) -> Option<&'a str> {
    item.metadata
        .annotations
        .as_ref()?
        .get(key)
        .map(String::as_str)
}

/// One annotation parsed as an integer, or `None`.
pub fn meta_annotation_int(item: &DynamicObject, key: &str) -> Option<i64> {
    meta_annotation(item, key).and_then(|s| s.parse::<i64>().ok())
}

/// The controlling `ownerReference` (the `metadata.ownerReferences[]`
/// entry with `controller: true`, of which the API allows one) as
/// `"<lowercase-kind>/<name>"` (`"replicaset/my-rs-xz4f"`), or `None` when
/// there is no controller owner.
pub fn meta_owner(item: &DynamicObject) -> Option<String> {
    item.owner_references()
        .iter()
        .find(|r| r.controller.unwrap_or(false))
        .map(|r| format!("{}/{}", r.kind.to_lowercase(), r.name))
}

/// An object reference (`involvedObject`, `scaleTargetRef`, a Binding's
/// `target`, an address's `targetRef`, a `roleRef`) as `"kind/name"` with the
/// kind lowercased, like [`meta_owner`], or `None` when the reference is
/// absent or carries neither.
pub fn object_ref(reference: Option<&Json>) -> Option<String> {
    let field = |key| reference.and_then(|r| json_str(r, &[key])).unwrap_or("");
    match (field("kind"), field("name")) {
        ("", "") => None,
        (kind, name) => Some(format!("{}/{name}", kind.to_lowercase())),
    }
}

/// The addresses of a `status.loadBalancer` (a Service's or an Ingress')
/// as kubectl's `loadBalancerStatusStringer` lists them: each ingress
/// entry's IP, else its hostname, deduplicated and sorted; empty while
/// the load balancer is pending.
pub fn lb_addresses(data: &Json) -> Vec<&str> {
    json_array(data, &["status", "loadBalancer", "ingress"])
        .iter()
        .filter_map(|e| {
            json_str_nonempty(e, &["ip"]).or_else(|| json_str_nonempty(e, &["hostname"]))
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

// ---------------------------------------------------------------------------
// 3. Quantity converters  (string → typed column)
// ---------------------------------------------------------------------------

/// A timestamp column → `date`. A newtype because neither `IntoValue` nor
/// `jiff::Timestamp` is ours. Ordered, so `Option<Date>::max` picks the
/// latest and `a.or(b)` the first present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date(pub jiff::Timestamp);

impl IntoValue for Date {
    fn into_value(self, span: Span) -> Value {
        timestamp_to_value(self.0, span)
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

/// A dimensionless quantity (a count, a ratio) → `int` when integral, else
/// `float`.
///
/// The apiserver canonicalises quantities, so a count of `1000` is read back
/// as `"1k"` and a ratio of `1.5` as `"1500m"`; this turns them back into
/// numbers so a column never mixes ints and strings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity(pub f64);

impl IntoValue for Quantity {
    fn into_value(self, span: Span) -> Value {
        let v = self.0;
        match v.fract() == 0.0 && v.abs() < (1u64 << 53) as f64 {
            true => Value::int(v as i64, span),
            false => Value::float(v, span),
        }
    }
}

/// A Kubernetes `IntOrString` (a Service `targetPort`, a rolling update's
/// `maxUnavailable`, a PDB's `minAvailable`, …) → `int` or `string` as the
/// server stored it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntOrString<'a> {
    Int(i64),
    String(&'a str),
}

impl IntoValue for IntOrString<'_> {
    fn into_value(self, span: Span) -> Value {
        match self {
            IntOrString::Int(i) => Value::int(i, span),
            IntOrString::String(s) => Value::string(s, span),
        }
    }
}

/// Parse an RFC 3339 / ISO 8601 timestamp string, or `None` on failure.
pub fn parse_timestamp(s: &str) -> Option<jiff::Timestamp> {
    s.parse().ok()
}

/// Convert a `jiff::Timestamp` to a `Value::date` (chrono `DateTime<FixedOffset>`).
///
/// Jiff timestamps are always UTC, so we fix the offset to zero.
/// This is the single place that bridges jiff → chrono for nushell.
///
/// Splits the i128 nanosecond count with Euclidean division so pre-1970
/// instants keep a non-negative sub-second part, as chrono requires. (An
/// `i64` nanosecond count would only reach 1677–2262; this covers jiff's
/// whole range.)
fn timestamp_to_value(ts: jiff::Timestamp, span: Span) -> Value {
    const NANOS_PER_SEC: i128 = 1_000_000_000;
    let nanos = ts.as_nanosecond();
    let secs = nanos.div_euclid(NANOS_PER_SEC) as i64;
    let subsec = nanos.rem_euclid(NANOS_PER_SEC) as u32;
    match chrono::DateTime::from_timestamp(secs, subsec) {
        Some(dt) => Value::date(dt.fixed_offset(), span),
        None => Value::nothing(span),
    }
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
/// microseconds (`us`/`µs`), nanoseconds (`ns`).
pub fn parse_duration(s: &str) -> Option<Duration> {
    duration_to_nanos(s).map(Duration)
}

/// Parse a dimensionless quantity (see [`Quantity`]); `None` when
/// unparseable.
pub fn quantity(raw: &str) -> Option<Quantity> {
    parse_quantity(raw).map(Quantity)
}

/// The timestamp at `path`, or `None` when absent or unparseable.
pub fn json_date<const N: usize>(root: &Json, path: &[&str; N]) -> Option<Date> {
    json_str(root, path).and_then(parse_timestamp).map(Date)
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

/// A quantity typed by what it measures (see [`typed_quantity`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypedQuantity {
    /// CPU → `int` millicores.
    Millicores(i64),
    /// Memory or storage → `filesize`.
    Bytes(Filesize),
    /// Anything else → a [`Quantity`].
    Count(Quantity),
}

impl IntoValue for TypedQuantity {
    fn into_value(self, span: Span) -> Value {
        match self {
            TypedQuantity::Millicores(m) => Value::int(m, span),
            TypedQuantity::Bytes(b) => b.into_value(span),
            TypedQuantity::Count(q) => q.into_value(span),
        }
    }
}

/// A quantity typed by the resource it measures: `cpu` → millicores;
/// `memory`, `storage`, `ephemeral-storage`, `hugepages-*` → filesize;
/// anything else (object counts, extended resources such as
/// `nvidia.com/gpu`, ratios) → [`Quantity`]. `None` when `raw` is absent
/// or unparseable.
///
/// The resource name's last path segment decides, so a quota's
/// `gold.storageclass.storage.k8s.io/requests.storage` is a size while
/// `gold.storageclass.storage.k8s.io/persistentvolumeclaims` is a count,
/// and a `requests.`/`limits.` prefix is transparent. This is the one
/// classifier behind [`Resources`], quotas and limit ranges.
pub fn typed_quantity(resource: &str, raw: Option<&Json>) -> Option<TypedQuantity> {
    let leaf = resource.rsplit('/').next().unwrap_or(resource);
    let leaf = leaf
        .strip_prefix("requests.")
        .or_else(|| leaf.strip_prefix("limits."))
        .unwrap_or(leaf);
    let text = quantity_text(raw);
    match leaf {
        "cpu" => parse_cpu(&text).map(TypedQuantity::Millicores),
        "memory" | "storage" | "ephemeral-storage" => parse_memory(&text).map(TypedQuantity::Bytes),
        _ if leaf.starts_with("hugepages-") => parse_memory(&text).map(TypedQuantity::Bytes),
        _ => quantity(&text).map(TypedQuantity::Count),
    }
}

/// The memory quantity at `path`, or `None`.
pub fn json_memory<const N: usize>(root: &Json, path: &[&str; N]) -> Option<Filesize> {
    parse_memory(&json_quantity_str(root, path)?)
}

/// The CPU quantity at `path` in millicores, or `None`.
pub fn json_cpu<const N: usize>(root: &Json, path: &[&str; N]) -> Option<i64> {
    parse_cpu(&json_quantity_str(root, path)?)
}

/// The dimensionless quantity at `path`, or `None`.
pub fn json_quantity<const N: usize>(root: &Json, path: &[&str; N]) -> Option<Quantity> {
    quantity(&json_quantity_str(root, path)?)
}

/// The Go duration at `path`, or `None`.
pub fn json_duration<const N: usize>(root: &Json, path: &[&str; N]) -> Option<Duration> {
    parse_duration(json_str(root, path)?)
}

/// The `IntOrString` at `path`, or `None` when absent (or a non-integral
/// number).
pub fn json_int_or_string<'a, const N: usize>(
    root: &'a Json,
    path: &[&str; N],
) -> Option<IntOrString<'a>> {
    match json_at(root, path)? {
        Json::Number(n) => n.as_i64().map(IntOrString::Int),
        Json::String(s) => Some(IntOrString::String(s)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 4. Spec / status extractors
// ---------------------------------------------------------------------------

/// The first condition of type `cond_type` in `conditions`.
pub fn condition<'a>(conditions: &'a [Json], cond_type: &str) -> Option<&'a Json> {
    conditions
        .iter()
        .find(|c| json_str(c, &["type"]) == Some(cond_type))
}

/// The `status` string (`"True"`, `"False"`, `"Unknown"`) of the first
/// `.status.conditions[]` entry of type `cond_type`, or `None` when absent.
pub fn condition_status<'a>(data: &'a Json, cond_type: &str) -> Option<&'a str> {
    condition_status_in(json_array(data, &["status", "conditions"]), cond_type)
}

/// [`condition_status`] over any conditions array — for objects that keep
/// conditions somewhere else, such as a route's `status.parents[].conditions`.
fn condition_status_in<'a>(conditions: &'a [Json], cond_type: &str) -> Option<&'a str> {
    condition(conditions, cond_type).and_then(|c| json_str(c, &["status"]))
}

/// Whether the condition of type `cond_type` exists with status `"True"`.
pub fn condition_is_true(data: &Json, cond_type: &str) -> bool {
    condition_status(data, cond_type) == Some("True")
}

/// A condition as a tri-state: `true`/`false` for `"True"`/`"False"`, `None`
/// when it is `"Unknown"` or absent (not yet reconciled).
pub fn condition_bool(data: &Json, cond_type: &str) -> Option<bool> {
    condition_bool_in(json_array(data, &["status", "conditions"]), cond_type)
}

/// [`condition_bool`] over any conditions array.
pub fn condition_bool_in(conditions: &[Json], cond_type: &str) -> Option<bool> {
    match condition_status_in(conditions, cond_type) {
        Some("True") => Some(true),
        Some("False") => Some(false),
        _ => None,
    }
}

/// `.spec.selector` as a flat map. For **Services** (and
/// ReplicationControllers), `spec.selector` is a flat `{ label: value }`
/// map; empty when absent.
pub fn spec_selector(data: &Json) -> StrMap<'_> {
    json_str_map(data, &["spec", "selector"])
}

/// A Kubernetes `LabelSelector` (a workload's `spec.selector`, a PDB's, a
/// Cilium `EndpointSelector`, a CSIStorageCapacity's `nodeTopology`) →
/// `{ matchLabels: record, matchExpressions: [ { key, operator, values } ] }`.
///
/// A present selector always has both columns (an empty record / list when
/// unset), so a selector written with `matchExpressions` only is not
/// mistaken for one that matches everything.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
pub struct LabelSelector<'a> {
    pub match_labels: StrMap<'a>,
    pub match_expressions: Vec<SelectorRequirement<'a>>,
}

/// One `matchExpressions` entry of a [`LabelSelector`].
#[derive(IntoValue)]
pub struct SelectorRequirement<'a> {
    pub key: Option<&'a str>,
    pub operator: Option<&'a str>,
    pub values: Vec<&'a str>,
}

/// The [`LabelSelector`] at `node`. An absent selector is `None`: what it
/// means (no objects, every object, not applicable) is the field's
/// business, never an invented `{}`.
pub fn label_selector(node: Option<&Json>) -> Option<LabelSelector<'_>> {
    let node = node?;
    Some(LabelSelector {
        match_labels: json_str_map(node, &["matchLabels"]),
        match_expressions: json_array(node, &["matchExpressions"])
            .iter()
            .map(|e| SelectorRequirement {
                key: json_str_nonempty(e, &["key"]),
                operator: json_str_nonempty(e, &["operator"]),
                values: json_str_list(e, &["values"]),
            })
            .collect(),
    })
}

/// [`label_selector`] over `.spec.selector`, for the workload resources
/// (Deployment, DaemonSet, StatefulSet, ReplicaSet, PVC, PDB) whose selector
/// is a `LabelSelector` rather than the flat map of [`spec_selector`].
pub fn spec_label_selector(data: &Json) -> Option<LabelSelector<'_>> {
    label_selector(json_at(data, &["spec", "selector"]))
}

/// A rolling-update strategy (a Deployment's `spec.strategy`, a DaemonSet's
/// `spec.updateStrategy`) → `{ type, maxUnavailable, maxSurge }`.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
pub struct Strategy<'a> {
    pub r#type: &'a str,
    pub max_unavailable: Option<IntOrString<'a>>,
    pub max_surge: Option<IntOrString<'a>>,
}

/// The [`Strategy`] at `node`. `type` defaults to `RollingUpdate` as the
/// API does; `maxUnavailable` and `maxSurge` are as stored, `None` when
/// absent.
pub fn spec_strategy(node: Option<&Json>) -> Strategy<'_> {
    let field = |key| node.and_then(|n| json_int_or_string(n, &["rollingUpdate", key]));
    Strategy {
        r#type: node
            .and_then(|n| json_str(n, &["type"]))
            .unwrap_or("RollingUpdate"),
        max_unavailable: field("maxUnavailable"),
        max_surge: field("maxSurge"),
    }
}

/// One `.status.conditions[]` entry → `{ type, status, reason, message,
/// updated }`, `updated` being its `lastTransitionTime`.
#[derive(IntoValue)]
pub struct Condition<'a> {
    pub r#type: Option<&'a str>,
    pub status: Option<&'a str>,
    pub reason: Option<&'a str>,
    pub message: Option<&'a str>,
    pub updated: Option<Date>,
}

/// Every `.status.conditions[]` entry as a [`Condition`].
pub fn status_conditions_list(data: &Json) -> Vec<Condition<'_>> {
    json_array(data, &["status", "conditions"])
        .iter()
        .map(|c| Condition {
            r#type: json_str_nonempty(c, &["type"]),
            status: json_str_nonempty(c, &["status"]),
            reason: json_str_nonempty(c, &["reason"]),
            message: json_str_nonempty(c, &["message"]),
            updated: json_date(c, &["lastTransitionTime"]),
        })
        .collect()
}

/// One endpoint port (a core Endpoints' `subsets[].ports[]`, an
/// EndpointSlice's `ports[]`, which share the shape); `protocol` defaults
/// to TCP as the API does.
#[derive(IntoValue)]
pub struct EndpointPort<'a> {
    pub name: Option<&'a str>,
    pub port: Option<i64>,
    pub protocol: &'a str,
}

/// A slice of endpoint port JSON objects → their [`EndpointPort`] rows.
pub fn endpoint_ports(ports: &[Json]) -> Vec<EndpointPort<'_>> {
    ports
        .iter()
        .map(|p| EndpointPort {
            name: json_str_nonempty(p, &["name"]),
            port: json_i64(p, &["port"]),
            protocol: json_str(p, &["protocol"]).unwrap_or("TCP"),
        })
        .collect()
}

/// The first key in `candidates` present under `.spec`, or `None` when none
/// match.
///
/// Surfaces which variant of a union-typed spec is configured (e.g. a Traefik
/// `Middleware` is a `stripPrefix` *or* a `headers` *or* …; a Hub
/// `AccessControlPolicy` is `jwt` *or* `oidc* *or* …) as one queryable column,
/// instead of forcing callers to probe every key.
pub fn detected_type<'c>(data: &Json, candidates: &[&'c str]) -> Option<&'c str> {
    let spec = json_at(data, &["spec"])?;
    candidates
        .iter()
        .copied()
        .find(|key| spec.get(key).is_some())
}

/// Compact columns of a plain replica controller — `apps/v1 ReplicaSet` and
/// its predecessor `core/v1 ReplicationController` count and report replicas
/// identically.
#[derive(IntoValue)]
pub struct ReplicaCounts {
    pub status: &'static str,
    pub desired: i64,
    pub current: i64,
    pub ready: i64,
    pub available: i64,
}

/// The [`ReplicaCounts`] of a ReplicaSet or ReplicationController.
pub fn replica_controller_compact(item: &DynamicObject) -> ReplicaCounts {
    let data = &item.data;
    // spec.replicas defaults to 1 when absent (Kubernetes default)
    let desired = json_i64(data, &["spec", "replicas"]).unwrap_or(1);
    let current = json_i64(data, &["status", "replicas"]).unwrap_or(0);
    let ready = json_i64(data, &["status", "readyReplicas"]).unwrap_or(0);
    let available = json_i64(data, &["status", "availableReplicas"]).unwrap_or(0);

    // Scaling either way: a scale-down is not Ready until the extra pods go.
    let status = if current != desired {
        "Scaling"
    } else if ready < desired {
        "NotReady"
    } else {
        "Ready"
    };

    ReplicaCounts {
        status,
        desired,
        current,
        ready,
        available,
    }
}

// ---------------------------------------------------------------------------
// 5. Container helpers
// ---------------------------------------------------------------------------

/// A resource map (a container's `requests`/`limits`, a Node's `capacity`,
/// a RuntimeClass's `overhead.podFixed`) → `record` with one column per key,
/// each typed by [`typed_quantity`]. `cpu` and `memory` are always present.
#[derive(Debug, Clone, Copy)]
pub struct Resources<'a>(pub &'a Json);

impl IntoValue for Resources<'_> {
    fn into_value(self, span: Span) -> Value {
        let map = self.0.as_object();
        let at = |key: &str| map.and_then(|m| m.get(key));
        let typed = |key: &str, raw| typed_quantity(key, raw).into_value(span);
        // Named by resource, so built key by key rather than derived.
        let mut rec = Record::new();
        rec.push("cpu", typed("cpu", at("cpu")));
        rec.push("memory", typed("memory", at("memory")));
        for (key, raw) in map.into_iter().flatten() {
            if key != "cpu" && key != "memory" {
                rec.push(key, typed(key, Some(raw)));
            }
        }
        Value::record(rec, span)
    }
}

/// The [`Resources`] map at `path`, or `None` when absent.
pub fn json_resources<'a, const N: usize>(
    root: &'a Json,
    path: &[&str; N],
) -> Option<Resources<'a>> {
    json_at(root, path).map(Resources)
}

/// A container spec → `{ name, image, requests, limits }`, which callers may
/// extend with a [`Concat`].
///
/// `requests` and `limits` are always present (`nothing` when unset) so
/// `get containers.requests.cpu` never errors on a container without them.
#[derive(IntoValue)]
pub struct Container<'a> {
    pub name: Option<&'a str>,
    pub image: Option<&'a str>,
    pub requests: Option<Resources<'a>>,
    pub limits: Option<Resources<'a>>,
}

/// The [`Container`] row of one container JSON object.
pub fn container_base(c: &Json) -> Container<'_> {
    Container {
        name: json_str_nonempty(c, &["name"]),
        image: json_str_nonempty(c, &["image"]),
        requests: json_resources(c, &["resources", "requests"]),
        limits: json_resources(c, &["resources", "limits"]),
    }
}

/// A slice of container JSON objects → their [`Container`] rows.
///
/// The caller is responsible for locating the right array with `json_array`:
///
/// ```rust
/// // Standard workload (Deployment, DaemonSet, StatefulSet, ReplicaSet, Job)
/// fmt_containers(json_array(data, &["spec", "template", "spec", "containers"]))
/// ```
pub fn fmt_containers(containers: &[Json]) -> Vec<Container<'_>> {
    containers.iter().map(container_base).collect()
}

/// A slice of container JSON objects → their images, `None` for a container
/// without one. Same calling convention as [`fmt_containers`].
pub fn fmt_images(containers: &[Json]) -> Vec<Option<&str>> {
    containers
        .iter()
        .map(|c| json_str_nonempty(c, &["image"]))
        .collect()
}

// ---------------------------------------------------------------------------
// Internal quantity parsers (consumed by the public wrappers above)
// ---------------------------------------------------------------------------

/// Parse a Kubernetes `Quantity` string to its value in base units
/// (bytes, cores, or a plain count).
///
/// Handles every suffix of the quantity grammar — binary (`Ki`..`Ei`),
/// decimal SI (`n`, `u`, `m`, `k`, `M`, `G`, `T`, `P`, `E`) and a decimal
/// exponent (`1e3`) — and fractional numbers (`"1.5Gi"`). Returns `None` for
/// an empty, negative or unparseable string.
pub(crate) fn parse_quantity(s: &str) -> Option<f64> {
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
    let s = s.trim();
    if let Some(positive) = s.strip_prefix('-') {
        return duration_to_nanos(positive)?.checked_neg();
    }
    let s = s.strip_prefix('+').unwrap_or(s);
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

        total = total.checked_add((num * nanos_per_unit as f64) as i64)?;
        remaining = &remaining[unit_len..];
    }

    Some(total)
}

// ---------------------------------------------------------------------------
// 6. Native JSON passthrough
// ---------------------------------------------------------------------------
//
// Some CRD specs (Cilium BGP instances, Traefik routes/middlewares, envoy
// services, …) are deeply nested and version-churny.  Rather than hand-type
// every field, these helpers pass the whole sub-tree through as a
// `serde_json::Value` field, whose `IntoValue` keeps its native structure
// (records/lists with typed scalars) so it stays fully queryable without
// flattening anything into composite strings.

/// A JSON node, or an empty **list** when absent. Use for array-shaped
/// fields.
pub fn native_list(node: Option<&Json>) -> Json {
    node.cloned().unwrap_or_else(|| Json::Array(Vec::new()))
}

/// A JSON node, or `None` (nothing) when absent. Use for object-shaped
/// fields.
pub fn native_or_nothing(node: Option<&Json>) -> Option<Json> {
    node.cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        let q = |s: &str| quantity(s).into_value(span);
        assert_eq!(q("1k"), Value::int(1_000, span));
        assert_eq!(q("2000"), Value::int(2_000, span));
        assert_eq!(q("1500m"), Value::float(1.5, span));
        assert_eq!(q("4"), Value::int(4, span));
        assert_eq!(q("1Ki"), Value::int(1_024, span));
        assert_eq!(q(""), Value::nothing(span));
        assert_eq!(q("x"), Value::nothing(span));
        // A bare JSON number is a quantity too.
        let j = serde_json::json!({"storage": 10737418240u64, "pods": 110});
        assert_eq!(
            json_memory(&j, &["storage"]).into_value(span),
            Value::filesize(10_737_418_240i64, span)
        );
        assert_eq!(
            json_quantity(&j, &["pods"]).into_value(span),
            Value::int(110, span)
        );
        assert_eq!(json_memory(&j, &["absent"]), None);
    }

    #[test]
    fn resource_maps() {
        let span = Span::test_data();
        let r = serde_json::json!({"cpu": "250m", "pods": "110", "hugepages-2Mi": "0", "ephemeral-storage": "10Gi", "nvidia.com/gpu": "2"});
        let Value::Record { val, .. } = Resources(&r).into_value(span) else {
            panic!("not a record");
        };
        assert_eq!(val.get("cpu"), Some(&Value::int(250, span)));
        assert_eq!(val.get("memory"), Some(&Value::nothing(span)));
        assert_eq!(val.get("pods"), Some(&Value::int(110, span)));
        assert_eq!(
            val.get("ephemeral-storage"),
            Some(&Value::filesize(10i64 << 30, span))
        );
        assert_eq!(val.get("nvidia.com/gpu"), Some(&Value::int(2, span)));
        let bare = container_base(&serde_json::json!({"name": "c"})).into_value(span);
        let bare = bare.as_record().expect("a record");
        assert_eq!(
            bare.columns().collect::<Vec<_>>(),
            ["name", "image", "requests", "limits"]
        );
        assert_eq!(bare.get("requests"), Some(&Value::nothing(span)));
    }

    #[test]
    fn quantities_are_typed_by_resource() {
        let span = Span::test_data();
        let q = |resource: &str, raw: &str| {
            typed_quantity(resource, Some(&json!(raw))).into_value(span)
        };
        assert_eq!(q("requests.cpu", "500m"), Value::int(500, span));
        assert_eq!(q("limits.memory", "1Gi"), Value::filesize(1i64 << 30, span));
        assert_eq!(
            q("gold.storageclass.storage.k8s.io/requests.storage", "50Gi"),
            Value::filesize(50i64 << 30, span)
        );
        assert_eq!(
            q(
                "gold.storageclass.storage.k8s.io/persistentvolumeclaims",
                "5"
            ),
            Value::int(5, span)
        );
        assert_eq!(q("hugepages-2Mi", "1Gi"), Value::filesize(1i64 << 30, span));
        assert_eq!(q("count/deployments.apps", "3"), Value::int(3, span));
        assert_eq!(q("requests.nvidia.com/gpu", "2"), Value::int(2, span));
        assert_eq!(q("pods", ""), Value::nothing(span));
        assert_eq!(q("pods", "1k"), Value::int(1_000, span));
        assert_eq!(typed_quantity("pods", None), None);
        // A bare JSON number is a quantity too.
        assert_eq!(
            typed_quantity("cpu", Some(&json!(2))),
            Some(TypedQuantity::Millicores(2_000))
        );
    }

    #[test]
    fn int_or_string() {
        let span = Span::test_data();
        let v = |j: Json| json_int_or_string(&j, &["x"]).into_value(span);
        assert_eq!(v(json!({"x": 1})), Value::int(1, span));
        assert_eq!(v(json!({"x": "25%"})), Value::string("25%", span));
        assert_eq!(v(json!({"x": 1.5})), Value::nothing(span));
        assert_eq!(v(json!({"x": null})), Value::nothing(span));
        assert_eq!(v(json!({})), Value::nothing(span));
    }

    #[test]
    fn object_refs() {
        let r = |v: Json| object_ref(Some(&v));
        assert_eq!(object_ref(None), None);
        assert_eq!(r(json!({})), None);
        assert_eq!(
            r(json!({"kind": "Pod", "name": "web-1"})).as_deref(),
            Some("pod/web-1")
        );
        // One side alone still renders.
        assert_eq!(r(json!({"kind": "Node"})).as_deref(), Some("node/"));
        assert_eq!(r(json!({"name": "x"})).as_deref(), Some("/x"));
    }

    #[test]
    fn label_selectors() {
        let span = Span::test_data();
        let sel = json!({"matchExpressions": [{"key": "k", "operator": "Exists"}]});
        let Value::Record { val, .. } = label_selector(Some(&sel)).into_value(span) else {
            panic!("not a record");
        };
        assert_eq!(
            val.get("matchLabels")
                .and_then(|v| v.as_record().ok().map(|r| r.len())),
            Some(0)
        );
        assert_eq!(
            val.get("matchExpressions")
                .and_then(|v| v.as_list().ok().map(|l| l.len())),
            Some(1)
        );
        assert!(label_selector(None).is_none());
    }

    #[test]
    fn strategies() {
        let span = Span::test_data();
        let s = json!({"type": "RollingUpdate", "rollingUpdate": {"maxUnavailable": 1, "maxSurge": "25%"}});
        let Value::Record { val, .. } = spec_strategy(Some(&s)).into_value(span) else {
            panic!("not a record");
        };
        assert_eq!(
            val.columns().collect::<Vec<_>>(),
            ["type", "maxUnavailable", "maxSurge"]
        );
        assert_eq!(val.get("maxUnavailable"), Some(&Value::int(1, span)));
        assert_eq!(val.get("maxSurge"), Some(&Value::string("25%", span)));
        let Value::Record { val, .. } = spec_strategy(None).into_value(span) else {
            panic!("not a record");
        };
        assert_eq!(val.get("type"), Some(&Value::string("RollingUpdate", span)));
        assert_eq!(val.get("maxSurge"), Some(&Value::nothing(span)));
    }

    #[test]
    fn dates() {
        let span = Span::test_data();
        let date = |s: &str| parse_timestamp(s).map(Date).into_value(span).as_date().ok();
        assert_eq!(
            date("1969-12-31T23:59:59.5Z"),
            chrono::DateTime::from_timestamp(-1, 500_000_000).map(|d| d.fixed_offset())
        );
        assert_eq!(
            date("2026-01-02T03:04:05.123456789Z"),
            chrono::DateTime::from_timestamp(1_767_323_045, 123_456_789).map(|d| d.fixed_offset())
        );
        // Beyond the i64-nanosecond range (1677–2262).
        assert_eq!(
            date("9000-01-01T00:00:00Z"),
            chrono::DateTime::from_timestamp(221_845_392_000, 0).map(|d| d.fixed_offset())
        );
        assert_eq!(
            date("1500-01-01T00:00:00.25Z"),
            chrono::DateTime::from_timestamp(-14_831_769_600, 250_000_000)
                .map(|d| d.fixed_offset())
        );
        assert_eq!(date("not a date"), None);
    }

    /// A [`Concat`] of rows is one record, the first row's columns first;
    /// [`append`] adds a row's columns after what a record already holds.
    #[test]
    fn rows_concatenate() {
        #[derive(IntoValue)]
        #[nu_value(rename_all = "camelCase")]
        struct Head<'a> {
            first_column: &'a str,
        }
        #[derive(IntoValue)]
        struct Tail {
            n: i64,
            missing: Option<bool>,
        }
        let span = Span::test_data();
        let mut rec = Record::new();
        rec.push("name", Value::string("x", span));
        append(
            &mut rec,
            Concat(
                Head { first_column: "a" },
                Tail {
                    n: 1,
                    missing: None,
                },
            ),
            span,
        );
        assert_eq!(
            rec.columns().collect::<Vec<_>>(),
            ["name", "firstColumn", "n", "missing"]
        );
        assert_eq!(rec.get("missing"), Some(&Value::nothing(span)));
        let map = json!({"m": {"b": "2", "a": 1}});
        let Value::Record { val, .. } = json_str_map(&map, &["m"]).into_value(span) else {
            panic!("not a record");
        };
        assert_eq!(val.get("b"), Some(&Value::string("2", span)));
        assert_eq!(val.get("a"), Some(&Value::string("", span)));
    }

    #[test]
    fn memory_quantities() {
        let bytes = |s: &str| parse_memory(s).map(|f| f.get());
        assert_eq!(bytes("512Mi"), Some(512 << 20));
        assert_eq!(bytes("1.5Gi"), Some(3 << 29));
        assert_eq!(bytes("2G"), Some(2_000_000_000));
        assert_eq!(bytes("1000m"), Some(1));
        assert_eq!(bytes("123"), Some(123));
        assert_eq!(bytes("1e3"), Some(1_000));
        assert_eq!(bytes("0"), Some(0));
        // Beyond i64: saturates rather than wrapping negative.
        assert_eq!(bytes("16Ei"), Some(i64::MAX));
        assert_eq!(bytes(""), None);
        assert_eq!(bytes("garbage"), None);
        assert_eq!(bytes("-1Gi"), None);
    }

    #[test]
    fn cpu_quantities() {
        assert_eq!(parse_cpu("250m"), Some(250));
        assert_eq!(parse_cpu("2"), Some(2_000));
        assert_eq!(parse_cpu("0.5"), Some(500));
        assert_eq!(parse_cpu("1500000n"), Some(2));
        assert_eq!(parse_cpu("999999n"), Some(1));
        assert_eq!(parse_cpu("1000000n"), Some(1));
        assert_eq!(parse_cpu("0.5m"), Some(1));
        assert_eq!(parse_cpu("3k"), Some(3_000_000));
        assert_eq!(parse_cpu("1E"), Some(i64::MAX));
        assert_eq!(parse_cpu(""), None);
        assert_eq!(parse_cpu("abc"), None);
        // A bare JSON number is a quantity too.
        let j = json!({"cpu": 2, "m": "250m"});
        assert_eq!(json_cpu(&j, &["cpu"]), Some(2_000));
        assert_eq!(json_cpu(&j, &["m"]), Some(250));
        assert_eq!(json_cpu(&j, &["absent"]), None);
    }
}
