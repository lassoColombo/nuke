//! The capture tool: writes `tests/fixtures/<group>/<version>/<plural>.json`
//! for every registered resource a cluster serves.
//!
//! ```text
//! NUKE_CAPTURE_CONTEXT=kind-bench-console cargo test capture -- --ignored
//! NUKE_CAPTURE_CONTEXT=kind-bench-console NUKE_CAPTURE_ONLY=apps/v1/deployments cargo test capture -- --ignored
//! ```
//!
//! Each resource is listed across all namespaces as a Table with
//! `includeObject=Object`, the representation kubectl asks for, so one
//! response carries both the objects and the cells the apiserver's printer
//! produced for them. A fixture file holds one cluster's objects, since
//! its cells align with that apiserver's columns: an existing file is kept
//! unless `NUKE_CAPTURE_OVERWRITE=1`, so capturing from a second cluster
//! only fills the gaps. Resources the cluster does not serve, or serves
//! empty, are reported and left alone. The kubeconfig and context come
//! from the test process's own environment: a kind cluster, never a
//! production context.
//!
//! Objects are sanitized before writing: `managedFields`, `resourceVersion`
//! and the last-applied annotation go, every Secret value becomes a
//! placeholder, a CRD loses its schemas, and items are sorted by namespace
//! and name and capped,
//! objects whose cells differ in something other than name and age going
//! first so the cap keeps variety (a crashing pod, a pending claim) rather
//! than the alphabet.

use kube::api::ListParams;
use kube::config::KubeConfigOptions;
use kube::core::Request;
use kube::{Client, Config};
use serde_json::Value as Json;

use super::{Column, Fixture, Gvr, Item};

const TABLE_ACCEPT: &str = "application/json;as=Table;v=v1;g=meta.k8s.io";

/// How many objects to keep per resource, after sorting. Pods vary the
/// most, nodes are few.
fn cap(gvr: &Gvr) -> usize {
    match (gvr.group.as_str(), gvr.plural.as_str()) {
        ("", "pods") => 12,
        ("", "nodes") => usize::MAX,
        _ => 6,
    }
}

#[test]
#[ignore]
fn capture() {
    let context = std::env::var("NUKE_CAPTURE_CONTEXT")
        .expect("set NUKE_CAPTURE_CONTEXT to a kind context name");
    let only = std::env::var("NUKE_CAPTURE_ONLY").ok();
    let overwrite = std::env::var_os("NUKE_CAPTURE_OVERWRITE").is_some();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let options = KubeConfigOptions {
            context: Some(context),
            ..Default::default()
        };
        let config = Config::from_kubeconfig(&options).await.unwrap();
        let client = Client::try_from(config).unwrap();
        let apiserver = client.apiserver_version().await.unwrap().git_version;
        let discovery = crate::discovery::DiscoveryCache::from_aggregated(
            &crate::discovery::fetch(&client).await.unwrap(),
        );

        let mut keys: Vec<&String> = super::REGISTRY.keys().collect();
        keys.sort();
        let (mut written, mut kept, mut empty, mut skipped) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for key in keys {
            if only.as_ref().is_some_and(|o| !key.contains(o.as_str())) {
                continue;
            }
            let [group, version, plural] = key.split('/').collect::<Vec<_>>()[..] else {
                panic!("bad key {key}");
            };
            let gvr = Gvr::new(group, version, plural);
            let Some(entry) = discovery.find_by_gvr(group, version, plural) else {
                skipped.push(key.clone());
                continue;
            };
            let out = gvr.fixture_path(".json");
            if out.exists() && !overwrite {
                kept.push(key.clone());
                continue;
            }
            let path = match group {
                "" => format!("/api/{version}/{plural}"),
                g => format!("/apis/{g}/{version}/{plural}"),
            };
            let table = list_table(&client, &path).await;
            let fixture = Fixture {
                apiserver: Some(apiserver.clone()),
                namespaced: entry.namespaced,
                columns: columns(&table),
                items: items(&gvr, &table),
            };
            if fixture.items.is_empty() {
                empty.push(key.clone());
                continue;
            }
            std::fs::create_dir_all(out.parent().unwrap()).unwrap();
            let mut text = serde_json::to_string_pretty(&fixture).unwrap();
            text.push('\n');
            std::fs::write(&out, text).unwrap();
            written.push(format!("{key}: {} items", fixture.items.len()));
        }
        eprintln!("written:\n  {}", written.join("\n  "));
        eprintln!(
            "kept (NUKE_CAPTURE_OVERWRITE=1 to replace):\n  {}",
            kept.join("\n  ")
        );
        eprintln!(
            "served but empty on this cluster:\n  {}",
            empty.join("\n  ")
        );
        eprintln!("not served by this cluster:\n  {}", skipped.join("\n  "));
    });
}

/// The Table of `path`, listed across all namespaces with the objects
/// included. A server that ignores the Accept parameter answers with a
/// plain list, which comes back as is.
async fn list_table(client: &Client, path: &str) -> Json {
    let request = Request::new(path)
        .list(&ListParams::default().limit(500))
        .unwrap();
    let (mut parts, body) = request.into_parts();
    parts.uri = format!("{}&includeObject=Object", parts.uri)
        .parse()
        .unwrap();
    parts
        .headers
        .insert(http::header::ACCEPT, TABLE_ACCEPT.parse().unwrap());
    client
        .request::<Json>(http::Request::from_parts(parts, body))
        .await
        .unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn columns(table: &Json) -> Vec<Column> {
    table["columnDefinitions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| Column {
            name: c["name"].as_str().unwrap_or("").to_string(),
            r#type: c["type"].as_str().unwrap_or("").to_string(),
            priority: c["priority"].as_i64().unwrap_or(0),
        })
        .collect()
}

/// The items of a Table (`rows[].object` + `cells`) or of a plain list
/// (`items[]`), sanitized, sorted and capped.
fn items(gvr: &Gvr, table: &Json) -> Vec<Item> {
    let mut items: Vec<Item> = match table["kind"].as_str() {
        Some("Table") => table["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|row| Item {
                object: sanitize(gvr, row["object"].clone()),
                cells: row["cells"].as_array().cloned(),
            })
            .collect(),
        _ => table["items"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|object| Item {
                object: sanitize(gvr, object.clone()),
                cells: None,
            })
            .collect(),
    };
    items.sort_by_key(Item::id);
    let columns = columns(table);
    let signature = |item: &Item| -> Vec<Json> {
        item.cells
            .iter()
            .flatten()
            .zip(&columns)
            .filter(|(_, c)| !matches!(c.name.as_str(), "Name" | "Age"))
            .map(|(cell, _)| cell.clone())
            .collect()
    };
    let mut seen = std::collections::HashSet::new();
    let (novel, rest): (Vec<Item>, Vec<Item>) = items
        .into_iter()
        .partition(|item| seen.insert(serde_json::to_string(&signature(item)).unwrap()));
    let mut items: Vec<Item> = novel.into_iter().chain(rest).collect();
    items.truncate(cap(gvr));
    items.sort_by_key(Item::id);
    items
}

fn sanitize(gvr: &Gvr, mut object: Json) -> Json {
    if let Some(meta) = object["metadata"].as_object_mut() {
        meta.remove("managedFields");
        meta.remove("resourceVersion");
        if let Some(ann) = meta.get_mut("annotations").and_then(|a| a.as_object_mut()) {
            ann.remove("kubectl.kubernetes.io/last-applied-configuration");
            if ann.is_empty() {
                meta.remove("annotations");
            }
        }
    }
    if gvr.group.is_empty()
        && gvr.plural == "secrets"
        && let Some(data) = object["data"].as_object_mut()
    {
        for value in data.values_mut() {
            // base64 of "REDACTED", so the value still decodes.
            *value = Json::String("UkVEQUNURUQ=".into());
        }
    }
    // A CRD's schemas run to hundreds of kilobytes and no column reads them.
    if gvr.group == "apiextensions.k8s.io"
        && let Some(versions) = object["spec"]["versions"].as_array_mut()
    {
        for version in versions {
            if let Some(v) = version.as_object_mut() {
                v.remove("schema");
            }
        }
    }
    object
}
