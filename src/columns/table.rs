//! The Table representation of a list or an object, as kubectl asks for it:
//! `Accept: application/json;as=Table;v=v1;g=meta.k8s.io` with
//! `includeObject=Object`, so one response carries the apiserver's cells
//! for every object and the object itself. A server that cannot serve a
//! Table for a resource answers the plain list or object instead, which
//! reads as a Table without columns.

use http::header::ACCEPT;
use kube::core::Request;
use kube::core::params::{GetParams, ListParams};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

/// kubectl's Accept header: a Table, or the plain representation when the
/// resource has no Table.
pub const ACCEPT_TABLE: &str = "application/json;as=Table;v=v1;g=meta.k8s.io, application/json";

/// One column of a Table, as `columnDefinitions` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TableColumn {
    pub name: String,
    #[serde(rename = "type", default)]
    pub r#type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub format: String,
    #[serde(default, skip_serializing_if = "super::is_zero")]
    pub priority: i64,
}

/// One row: the cells, aligned with the columns, and the object.
#[derive(Debug, Clone, Deserialize)]
pub struct Row {
    #[serde(default)]
    pub cells: Vec<Json>,
    pub object: Json,
}

#[derive(Debug, Clone, Default)]
pub struct Table {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<Row>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TableWire {
    #[serde(default)]
    column_definitions: Vec<TableColumn>,
    #[serde(default)]
    rows: Vec<Row>,
}

impl Table {
    /// A response as a Table: a `Table` as it is; a plain list as rows
    /// without cells; a single plain object as one row. The row objects
    /// of a list carry no `apiVersion`/`kind` of their own; a Table's
    /// objects do.
    ///
    /// A list is a kind ending in `List` without a name of its own: a
    /// custom resource may be called `AllowList`, but an object always has
    /// `metadata.name` and a list never does.
    pub fn from_response(mut json: Json) -> Result<Self, serde_json::Error> {
        let is_list = json
            .get("kind")
            .and_then(Json::as_str)
            .is_some_and(|kind| kind.ends_with("List"))
            && json.pointer("/metadata/name").is_none();
        match json.get("kind").and_then(Json::as_str) {
            Some("Table") => {
                let wire: TableWire = serde_json::from_value(json)?;
                Ok(Table {
                    columns: wire.column_definitions,
                    rows: wire.rows,
                })
            }
            _ if is_list => Ok(Table {
                columns: Vec::new(),
                rows: crate::values::take_items(&mut json)
                    .into_iter()
                    .map(|object| Row {
                        cells: Vec::new(),
                        object,
                    })
                    .collect(),
            }),
            _ => Ok(Table {
                columns: Vec::new(),
                rows: vec![Row {
                    cells: Vec::new(),
                    object: json,
                }],
            }),
        }
    }
}

/// The list request for `url_path` (`/api/v1/namespaces/x/pods`) as a
/// Table with objects.
pub fn list_request(url_path: &str, params: &ListParams) -> kube::Result<http::Request<Vec<u8>>> {
    let request = Request::new(url_path)
        .list(params)
        .map_err(kube::Error::BuildRequest)?;
    Ok(as_table(request))
}

/// The get request for `name` under `url_path` as a Table with the object.
pub fn get_request(url_path: &str, name: &str) -> kube::Result<http::Request<Vec<u8>>> {
    let request = Request::new(url_path)
        .get(name, &GetParams::default())
        .map_err(kube::Error::BuildRequest)?;
    Ok(as_table(request))
}

fn as_table(request: http::Request<Vec<u8>>) -> http::Request<Vec<u8>> {
    let (mut parts, body) = request.into_parts();
    let uri = parts.uri.to_string();
    let joiner = match uri.contains('?') {
        true => '&',
        false => '?',
    };
    parts.uri = format!("{uri}{joiner}includeObject=Object")
        .parse()
        .unwrap_or(parts.uri);
    parts
        .headers
        .insert(ACCEPT, ACCEPT_TABLE.parse().expect("a static header value"));
    http::Request::from_parts(parts, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_a_table_a_list_and_an_object() {
        let table = Table::from_response(json!({
            "kind": "Table",
            "columnDefinitions": [{"name": "Name", "type": "string"}, {"name": "Ready", "type": "string", "priority": 1}],
            "rows": [{"cells": ["a", "1/1"], "object": {"metadata": {"name": "a"}}}]
        }))
        .unwrap();
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.columns[1].priority, 1);
        assert_eq!(table.rows[0].cells[1], json!("1/1"));

        let list = Table::from_response(
            json!({"kind": "PodList", "items": [{"metadata": {"name": "a"}}]}),
        )
        .unwrap();
        assert!(list.columns.is_empty());
        assert_eq!(list.rows.len(), 1);
        assert!(list.rows[0].cells.is_empty());

        let one = Table::from_response(json!({"kind": "Pod", "metadata": {"name": "a"}})).unwrap();
        assert_eq!(one.rows.len(), 1);
        assert_eq!(one.rows[0].object["kind"], json!("Pod"));

        // A custom resource whose kind ends in List is still one object.
        let allow =
            Table::from_response(json!({"kind": "AllowList", "metadata": {"name": "a"}})).unwrap();
        assert_eq!(allow.rows.len(), 1);
        assert_eq!(allow.rows[0].object["metadata"]["name"], json!("a"));

        // An empty list may carry `items: null`.
        let empty = Table::from_response(json!({"kind": "PodList", "metadata": {}, "items": null}))
            .unwrap();
        assert!(empty.rows.is_empty());
    }

    #[test]
    fn requests_ask_for_a_table_with_objects() {
        let req = list_request(
            "/api/v1/namespaces/x/pods",
            &ListParams::default().labels("a=b"),
        )
        .unwrap();
        let uri = req.uri().to_string();
        assert!(uri.starts_with("/api/v1/namespaces/x/pods?"), "{uri}");
        assert!(uri.contains("labelSelector=a%3Db"), "{uri}");
        assert!(uri.ends_with("&includeObject=Object"), "{uri}");
        assert_eq!(req.headers()[ACCEPT], ACCEPT_TABLE);
        let req = get_request("/apis/apps/v1/namespaces/x/deployments", "web").unwrap();
        assert_eq!(
            req.uri().to_string(),
            "/apis/apps/v1/namespaces/x/deployments/web?includeObject=Object"
        );
    }
}
