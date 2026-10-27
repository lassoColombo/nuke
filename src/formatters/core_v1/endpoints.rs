//! Formatter for `core/v1 Endpoints` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    append, json_array, json_at, json_i64, json_str, json_str_nonempty, object_ref,
};

pub struct EndpointsFormatter;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Sum the lengths of a per-subset address array field
/// (`addresses` or `notReadyAddresses`).
fn address_count(item: &DynamicObject, field: &str) -> i64 {
    json_array(&item.data, &["subsets"])
        .iter()
        .map(|s| json_array(s, &[field]).len())
        .sum::<usize>() as i64
}

/// One address of a subset, `target` being the `targetRef` as
/// `"kind/name"`.
#[derive(IntoValue)]
struct Address<'a> {
    ip: Option<&'a str>,
    node: Option<&'a str>,
    target: Option<String>,
}

/// A subset's `addresses` or `notReadyAddresses`.
fn address_list<'a>(subset: &'a Json, field: &str) -> Vec<Address<'a>> {
    json_array(subset, &[field])
        .iter()
        .map(|a| Address {
            ip: json_str_nonempty(a, &["ip"]),
            node: json_str_nonempty(a, &["nodeName"]),
            target: object_ref(json_at(a, &["targetRef"])),
        })
        .collect()
}

/// One port of a subset; `protocol` defaults to TCP as the API does.
#[derive(IntoValue)]
struct EndpointPort<'a> {
    name: Option<&'a str>,
    port: Option<i64>,
    protocol: &'a str,
}

/// One `subsets[]` entry, keeping its own ports so kubectl's `ip:port`
/// pairs can be rebuilt.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Subset<'a> {
    addresses: Vec<Address<'a>>,
    not_ready_addresses: Vec<Address<'a>>,
    ports: Vec<EndpointPort<'a>>,
}

fn subsets(item: &DynamicObject) -> Vec<Subset<'_>> {
    json_array(&item.data, &["subsets"])
        .iter()
        .map(|s| Subset {
            addresses: address_list(s, "addresses"),
            not_ready_addresses: address_list(s, "notReadyAddresses"),
            ports: json_array(s, &["ports"])
                .iter()
                .map(|p| EndpointPort {
                    name: json_str_nonempty(p, &["name"]),
                    port: json_i64(p, &["port"]),
                    protocol: json_str(p, &["protocol"]).unwrap_or("TCP"),
                })
                .collect(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact {
    ready_count: i64,
    not_ready_count: i64,
}

#[derive(IntoValue)]
struct Wide<'a> {
    subsets: Vec<Subset<'a>>,
}

impl ResourceFormatter for EndpointsFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            ready_count: address_count(item, "addresses"),
            not_ready_count: address_count(item, "notReadyAddresses"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            subsets: subsets(item),
        };
        append(rec, wide, span);
    }
}
