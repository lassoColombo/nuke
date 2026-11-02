//! The capture tool: writes `tests/fixtures/<group>/<version>/<plural>.json`
//! for every resource a cluster serves that nuke has definitions for (a
//! built-in in `builtin.rs`) or whose CRD it can read.
//!
//! ```text
//! NUKE_CAPTURE_CONTEXT=kind-bench-console cargo test capture -- --ignored
//! NUKE_CAPTURE_CONTEXT=kind-bench-console NUKE_CAPTURE_ONLY=apps/v1/deployments cargo test capture -- --ignored
//! ```
//!
//! Each resource is listed across all namespaces as a Table with
//! `includeObject=Object`, the representation kubectl asks for, so one
//! response carries both the objects and the cells the apiserver's printer
//! produced for them; a custom resource's fixture also records the CRD's
//! printer columns the apiserver printed from. A fixture file holds one
//! cluster's objects, since its cells align with that apiserver's columns:
//! an existing file is kept unless `NUKE_CAPTURE_OVERWRITE=1`, so capturing
//! from a second cluster only fills the gaps. Resources the cluster serves
//! empty are reported and left alone. The kubeconfig and context come from
//! the test process's own environment: a kind cluster, never a production
//! context.
//!
//! Objects are sanitized before writing: `managedFields`, `resourceVersion`
//! and the last-applied annotation go, every Secret value becomes a
//! placeholder, a CRD loses its schemas, and items are sorted by namespace
//! and name and capped, objects whose cells differ in something other than
//! name and age going first so the cap keeps variety (a crashing pod, a
//! pending claim) rather than the alphabet.

use kube::api::ListParams;
use kube::config::KubeConfigOptions;
use kube::{Client, Config};
use serde_json::Value as Json;

use super::{Fixture, Gvr, Item};
use crate::columns::table::{self, Table};
use crate::columns::{builtin, crd};

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

        let (mut written, mut kept, mut empty, mut skipped) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for entry in discovery.entries() {
            let gvr = Gvr::new(&entry.group, &entry.version, &entry.plural);
            let key = gvr.key();
            if only.as_ref().is_some_and(|o| !key.contains(o.as_str())) {
                continue;
            }
            if !entry.verbs.iter().any(|v| v == "list") {
                continue;
            }
            let printer_columns = match builtin::find(&gvr.group, &gvr.version, &gvr.plural) {
                Some(_) => None,
                None => match crd_columns(&client, &gvr).await {
                    Some(columns) => Some(columns),
                    // Neither a built-in nor a custom resource: nothing to define.
                    None => {
                        skipped.push(key);
                        continue;
                    }
                },
            };
            let out = gvr.fixture_path(".json");
            if out.exists() && !overwrite {
                kept.push(key);
                continue;
            }
            let table = list_table(&client, &entry.url_path(None)).await;
            let fixture = Fixture {
                apiserver: Some(apiserver.clone()),
                namespaced: entry.namespaced,
                columns: table.columns.clone(),
                printer_columns,
                items: items(&gvr, table),
            };
            if fixture.items.is_empty() {
                empty.push(key);
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
        eprintln!(
            "neither built-in nor a readable CRD:\n  {}",
            skipped.join("\n  ")
        );
    });
}

/// The CRD's printer columns for this version, `None` when there is no
/// CRD for the resource.
async fn crd_columns(client: &Client, gvr: &Gvr) -> Option<Vec<crate::columns::PrinterColumn>> {
    let url = format!(
        "/apis/apiextensions.k8s.io/v1/customresourcedefinitions/{}.{}",
        gvr.plural, gvr.group
    );
    let request = http::Request::get(url).body(Vec::new()).unwrap();
    let crd: Json = client.request(request).await.ok()?;
    Some(
        crd::columns_of(&crd)
            .remove(&gvr.version)
            .unwrap_or_default(),
    )
}

/// The Table of `url_path`, listed across all namespaces with the objects
/// included.
async fn list_table(client: &Client, url_path: &str) -> Table {
    let request = table::list_request(url_path, &ListParams::default().limit(500)).unwrap();
    let json: Json = client
        .request(request)
        .await
        .unwrap_or_else(|e| panic!("{url_path}: {e}"));
    Table::from_response(json).unwrap()
}

/// The items of a Table, sanitized, sorted and capped.
fn items(gvr: &Gvr, table: Table) -> Vec<Item> {
    let columns = table.columns;
    let mut items: Vec<Item> = table
        .rows
        .into_iter()
        .map(|row| Item {
            object: sanitize(gvr, row.object),
            cells: (!row.cells.is_empty()).then_some(row.cells),
        })
        .collect();
    items.sort_by_key(Item::id);
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
