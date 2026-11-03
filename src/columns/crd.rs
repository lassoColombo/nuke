//! A custom resource's printer columns, read from its CRD and kept under
//! nuke's cache directory next to discovery:
//! `<cache root>/columns/<percent-encoded cluster URL>/<plural>.<group>.json`,
//! for [`CRD_TTL`]. The file holds every served version's
//! `additionalPrinterColumns`, or `null` for a resource that has no CRD (a
//! built-in, an aggregated API), so the miss is not repeated per call. A CRD
//! nuke is not allowed to read (403) is not stored: the next call tries again,
//! and the row falls back to the apiserver's cells meanwhile.
//!
//! A CRD can change within the TTL (an operator upgrade). The Table of every
//! `nuke get` names the columns the apiserver printed, so a stored answer
//! that disagrees with it ([`stale`]), or does not serve the version asked
//! for, is read anew at once.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::Result;
use http::Uri;
use kube::Client;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value as Json;

use super::PrinterColumn;
use super::table::TableColumn;
use crate::kube_env::KubeEnv;
use crate::values::json_array;

/// kubectl's discovery TTL, so a CRD's columns go stale with its resource.
pub const CRD_TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// `<root>/columns/<cluster>/<plural>.<group>.json`.
pub fn file(root: &Path, cluster_url: &Uri, group: &str, plural: &str) -> PathBuf {
    // Encoded as discovery encodes it, so one cluster has one directory name.
    const ESCAPE: &AsciiSet = &NON_ALPHANUMERIC.remove(b'.').remove(b'-').remove(b'_');
    let url = cluster_url.to_string();
    let cluster = utf8_percent_encode(url.trim_end_matches('/'), ESCAPE).to_string();
    root.join("columns")
        .join(cluster)
        .join(format!("{plural}.{group}.json"))
}

/// What the cluster answered about a resource's CRD.
#[derive(Debug, Clone, PartialEq)]
enum Answer {
    /// No such CRD (a 404): a built-in or an aggregated resource. Stored,
    /// so the miss is paid once.
    NoCrd,
    /// nuke may not read it (a 403). Not stored: the next call tries again.
    Forbidden,
    /// Every served version's printer columns.
    Versions(Versions),
}

type Versions = BTreeMap<String, Vec<PrinterColumn>>;

/// The printer columns of `version` of the custom resource `plural.group`,
/// given the columns of the Table the apiserver just printed for it.
/// `None` when the resource has no CRD, nuke may not read it, or the CRD
/// still disagrees with the Table once read anew: the row then reads the
/// apiserver's cells.
///
/// The stored answer serves while it is younger than [`CRD_TTL`], knows
/// `version`, and agrees with the Table ([`stale`]); otherwise the CRD is
/// read and stored again, once.
pub async fn printer_columns(
    env: &KubeEnv,
    client: &Client,
    cluster_url: &Uri,
    group: &str,
    version: &str,
    plural: &str,
    table: &[TableColumn],
) -> Result<Option<Vec<PrinterColumn>>> {
    // The core group has no CRDs; the caller spares the built-in groups.
    if group.is_empty() {
        return Ok(None);
    }
    let path = env
        .cache_root()
        .map(|root| file(&root, cluster_url, group, plural));
    if let Lookup::Serve(columns) = lookup(path.as_deref().and_then(read), version, table) {
        return Ok(columns);
    }
    let answer = fetch(client, group, plural).await?;
    if let Some(path) = &path {
        // A failed write only costs the next call a fetch.
        let _ = write(path, &answer);
    }
    Ok(match answer {
        Answer::Versions(versions) => Some(versions.get(version).cloned().unwrap_or_default())
            .filter(|columns| !stale(columns, table)),
        Answer::NoCrd | Answer::Forbidden => None,
    })
}

/// What a stored answer is worth for `version` against `table`.
#[derive(Debug, PartialEq)]
enum Lookup {
    /// These columns, or none for a resource without a CRD.
    Serve(Option<Vec<PrinterColumn>>),
    /// Nothing usable is stored: read the CRD.
    Refetch,
}

/// A stored "no CRD" serves as is; stored versions serve when they know
/// `version` and agree with the Table; anything else is read anew.
fn lookup(stored: Option<Answer>, version: &str, table: &[TableColumn]) -> Lookup {
    match stored {
        Some(Answer::NoCrd) => Lookup::Serve(None),
        Some(Answer::Versions(mut versions)) => match versions.remove(version) {
            Some(columns) if !stale(&columns, table) => Lookup::Serve(Some(columns)),
            _ => Lookup::Refetch,
        },
        Some(Answer::Forbidden) | None => Lookup::Refetch,
    }
}

async fn fetch(client: &Client, group: &str, plural: &str) -> Result<Answer> {
    let url = format!("/apis/apiextensions.k8s.io/v1/customresourcedefinitions/{plural}.{group}");
    let request = http::Request::get(url).body(Vec::new())?;
    match client.request::<Json>(request).await {
        Ok(crd) => Ok(Answer::Versions(columns_of(&crd))),
        Err(kube::Error::Api(status)) if status.code == 404 => Ok(Answer::NoCrd),
        Err(kube::Error::Api(status)) if status.code == 403 => Ok(Answer::Forbidden),
        Err(e) => Err(anyhow::Error::new(e).context(format!("reading the CRD {plural}.{group}"))),
    }
}

/// Every served version's `additionalPrinterColumns`, as the CRD declares
/// them; a version without any has the empty list.
pub fn columns_of(crd: &Json) -> Versions {
    json_array(crd, &["spec", "versions"])
        .iter()
        .filter_map(|v| {
            let name = v.get("name")?.as_str()?.to_string();
            let columns = json_array(v, &["additionalPrinterColumns"])
                .iter()
                .filter_map(|c| serde_json::from_value(c.clone()).ok())
                .collect();
            Some((name, columns))
        })
        .collect()
}

/// Whether the Table the apiserver printed disagrees with `printer`, which
/// means the CRD changed since `printer` was stored. The apiserver prints
/// Name, then the printer columns (Age when there are none), so the two
/// agree on every other name, in order. A Table without columns (a plain
/// list) says nothing.
pub fn stale(printer: &[PrinterColumn], table: &[TableColumn]) -> bool {
    fn significant<'a>(names: impl Iterator<Item = &'a str>) -> Vec<String> {
        names
            .filter(|n| !n.eq_ignore_ascii_case("name") && !n.eq_ignore_ascii_case("age"))
            .map(str::to_lowercase)
            .collect()
    }
    !table.is_empty()
        && significant(table.iter().map(|c| c.name.as_str()))
            != significant(printer.iter().map(|c| c.name.as_str()))
}

/// The stored answer, when there is one younger than [`CRD_TTL`]. The
/// file holds the versions, or `null` for "no CRD".
fn read(path: &Path) -> Option<Answer> {
    let modified = path.metadata().and_then(|m| m.modified()).ok()?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default();
    if age >= CRD_TTL {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Option<Versions>>(&text).ok()? {
        Some(versions) => Some(Answer::Versions(versions)),
        None => Some(Answer::NoCrd),
    }
}

/// Store `answer` atomically; a forbidden read is not stored.
fn write(path: &Path, answer: &Answer) -> Result<()> {
    let stored = match answer {
        Answer::Versions(versions) => Some(versions),
        Answer::NoCrd => None,
        Answer::Forbidden => return Ok(()),
    };
    let dir = path.parent().expect("a file path");
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    std::fs::write(&tmp, serde_json::to_vec(&stored)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_the_columns_of_every_version() {
        let crd = json!({"spec": {"versions": [
            {"name": "v1", "additionalPrinterColumns": [
                {"name": "Ready", "type": "string", "jsonPath": ".status.ready"},
                {"name": "Age", "type": "date", "jsonPath": ".metadata.creationTimestamp", "priority": 1}
            ]},
            {"name": "v1beta1"}
        ]}});
        let versions = columns_of(&crd);
        assert_eq!(versions["v1"].len(), 2);
        assert_eq!(versions["v1"][1].priority, 1);
        assert!(versions["v1beta1"].is_empty());
    }

    #[test]
    fn a_table_that_disagrees_is_stale() {
        let printer = |names: &[&str]| -> Vec<PrinterColumn> {
            names
                .iter()
                .map(|n| PrinterColumn {
                    name: n.to_string(),
                    r#type: "string".into(),
                    json_path: ".x".into(),
                    priority: 0,
                })
                .collect()
        };
        let table = |names: &[&str]| -> Vec<TableColumn> {
            names
                .iter()
                .map(|n| TableColumn {
                    name: n.to_string(),
                    ..Default::default()
                })
                .collect()
        };
        let ready = printer(&["Ready", "Age"]);
        assert!(!stale(&ready, &table(&["Name", "Ready", "Age"])));
        assert!(!stale(&printer(&[]), &table(&["Name", "Age"])));
        assert!(!stale(&ready, &[]));
        assert!(stale(&ready, &table(&["Name", "Ready", "Status", "Age"])));
        assert!(stale(&ready, &table(&["Name", "Synced", "Age"])));
        assert!(stale(&printer(&["A", "B"]), &table(&["Name", "B", "A"])));
    }

    #[test]
    fn a_stored_answer_serves_only_what_the_table_agrees_with() {
        let ready = vec![PrinterColumn {
            name: "Ready".into(),
            r#type: "string".into(),
            json_path: ".status.ready".into(),
            priority: 0,
        }];
        let table = |names: &[&str]| -> Vec<TableColumn> {
            names
                .iter()
                .map(|n| TableColumn {
                    name: n.to_string(),
                    ..Default::default()
                })
                .collect()
        };
        let stored = || {
            Some(Answer::Versions(
                [("v1".to_string(), ready.clone())].into_iter().collect(),
            ))
        };
        let agreeing = table(&["Name", "Ready", "Age"]);
        assert_eq!(
            lookup(stored(), "v1", &agreeing),
            Lookup::Serve(Some(ready.clone()))
        );
        // The CRD changed since: a new column.
        assert_eq!(
            lookup(stored(), "v1", &table(&["Name", "Ready", "Synced", "Age"])),
            Lookup::Refetch
        );
        // A version the stored CRD did not serve.
        assert_eq!(lookup(stored(), "v2", &agreeing), Lookup::Refetch);
        assert_eq!(
            lookup(Some(Answer::NoCrd), "v1", &agreeing),
            Lookup::Serve(None)
        );
        assert_eq!(lookup(None, "v1", &agreeing), Lookup::Refetch);
    }

    #[test]
    fn stores_and_expires() {
        let dir = std::env::temp_dir().join(format!("nuke-crd-test-{}", std::process::id()));
        let url: Uri = "https://127.0.0.1:6443".parse().unwrap();
        let path = file(&dir, &url, "cilium.io", "ciliumnodes");
        assert!(
            path.ends_with("columns/https%3A%2F%2F127.0.0.1%3A6443/ciliumnodes.cilium.io.json")
        );
        write(&path, &Answer::NoCrd).unwrap();
        assert_eq!(read(&path), Some(Answer::NoCrd));
        let versions: Versions = [("v2".to_string(), vec![])].into_iter().collect();
        write(&path, &Answer::Versions(versions.clone())).unwrap();
        assert_eq!(read(&path), Some(Answer::Versions(versions.clone())));
        // A forbidden read leaves what is stored alone.
        write(&path, &Answer::Forbidden).unwrap();
        assert_eq!(read(&path), Some(Answer::Versions(versions)));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
