//! Formatter for `discovery.k8s.io/v1 EndpointSlice` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    EndpointPort, append, endpoint_ports, json_array, json_at, json_bool, json_i64, json_str_list,
    json_str_nonempty, object_ref,
};

pub struct EndpointSliceFormatter;

/// One `.endpoints[]` entry.
#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Endpoint<'a> {
    addresses: Vec<&'a str>,
    ready: bool,
    serving: bool,
    terminating: bool,
    hostname: Option<&'a str>,
    target_ref: Option<String>,
}

fn endpoints_list(item: &DynamicObject) -> Vec<Endpoint<'_>> {
    json_array(&item.data, &["endpoints"])
        .iter()
        .map(|e| {
            // The API reads an unset `ready` as true, an unset `serving` as
            // whatever `ready` says, an unset `terminating` as false.
            let ready = json_bool(e, &["conditions", "ready"]).unwrap_or(true);
            Endpoint {
                addresses: json_str_list(e, &["addresses"]),
                ready,
                serving: json_bool(e, &["conditions", "serving"]).unwrap_or(ready),
                terminating: json_bool(e, &["conditions", "terminating"]).unwrap_or(false),
                hostname: json_str_nonempty(e, &["hostname"]),
                target_ref: object_ref(json_at(e, &["targetRef"])),
            }
        })
        .collect()
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    address_type: Option<&'a str>,
    /// kubectl's PORTS and ENDPOINTS: the port numbers, and every
    /// endpoint's addresses; `ports`/`endpoints` in wide hold the detail.
    port: Vec<Option<i64>>,
    addresses: Vec<&'a str>,
}

#[derive(IntoValue)]
struct Wide<'a> {
    ports: Vec<EndpointPort<'a>>,
    endpoints: Vec<Endpoint<'a>>,
}

impl ResourceFormatter for EndpointSliceFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let compact = Compact {
            address_type: json_str_nonempty(data, &["addressType"]),
            port: json_array(data, &["ports"])
                .iter()
                .map(|p| json_i64(p, &["port"]))
                .collect(),
            addresses: json_array(data, &["endpoints"])
                .iter()
                .flat_map(|e| json_array(e, &["addresses"]))
                .filter_map(|a| a.as_str())
                .collect(),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            ports: endpoint_ports(json_array(&item.data, &["ports"])),
            endpoints: endpoints_list(item),
        };
        append(rec, wide, span);
    }
}
