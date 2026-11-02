//! A custom resource's printer columns, read from its CRD and kept under
//! nuke's cache directory next to discovery:
//! `<cache root>/columns/<percent-encoded cluster URL>/<plural>.<group>.json`,
//! for [`CRD_TTL`]. The file holds every served version's
//! `additionalPrinterColumns`, or `null` for a resource that has no CRD (a
//! built-in, an aggregated API), so the miss is not repeated per call. A CRD
//! nuke is not allowed to read (403) is not stored: the next call tries again,
//! and the row falls back to the apiserver's cells meanwhile.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::Result;
use http::Uri;
use kube::Client;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value as Json;

use super::PrinterColumn;
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

/// The printer columns of `version` of the custom resource `plural.group`,
/// `None` when the resource has no CRD or nuke may not read it. The
/// stored answer when it is younger than [`CRD_TTL`], else one GET.
pub async fn printer_columns(
    env: &KubeEnv,
    client: &Client,
    cluster_url: &Uri,
    group: &str,
    version: &str,
    plural: &str,
) -> Result<Option<Vec<PrinterColumn>>> {
    // The core group has no CRDs; the caller spares the built-in groups.
    if group.is_empty() {
        return Ok(None);
    }
    let path = env
        .cache_root()
        .map(|root| file(&root, cluster_url, group, plural));
    if let Some(path) = &path
        && let Some(versions) = read(path)
    {
        return Ok(versions.map(|v| v.get(version).cloned().unwrap_or_default()));
    }
    let versions = fetch(client, group, plural).await?;
    if let (Some(path), Some(versions)) = (&path, &versions) {
        // Not stored on a failed fetch: nothing to remember.
        let _ = write(path, versions.as_ref());
    }
    Ok(versions
        .flatten()
        .map(|v| v.get(version).cloned().unwrap_or_default()))
}

type Versions = BTreeMap<String, Vec<PrinterColumn>>;

/// The CRD's printer columns per version: `Some(None)` for a 404 (no CRD),
/// `None` for a read nuke was refused, which is not worth remembering.
async fn fetch(client: &Client, group: &str, plural: &str) -> Result<Option<Option<Versions>>> {
    let url = format!("/apis/apiextensions.k8s.io/v1/customresourcedefinitions/{plural}.{group}");
    let request = http::Request::get(url).body(Vec::new())?;
    match client.request::<Json>(request).await {
        Ok(crd) => Ok(Some(Some(columns_of(&crd)))),
        Err(kube::Error::Api(status)) if status.code == 404 => Ok(Some(None)),
        Err(kube::Error::Api(status)) if status.code == 403 => Ok(None),
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

/// The stored answer, when there is one younger than [`CRD_TTL`]:
/// `Some(None)` for a stored "no CRD".
fn read(path: &Path) -> Option<Option<Versions>> {
    let modified = path.metadata().and_then(|m| m.modified()).ok()?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default();
    if age >= CRD_TTL {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Store `versions` (or the "no CRD" marker) atomically.
fn write(path: &Path, versions: Option<&Versions>) -> Result<()> {
    let dir = path.parent().expect("a file path");
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    std::fs::write(&tmp, serde_json::to_vec(&versions)?)?;
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
    fn stores_and_expires() {
        let dir = std::env::temp_dir().join(format!("nuke-crd-test-{}", std::process::id()));
        let url: Uri = "https://127.0.0.1:6443".parse().unwrap();
        let path = file(&dir, &url, "cilium.io", "ciliumnodes");
        assert!(
            path.ends_with("columns/https%3A%2F%2F127.0.0.1%3A6443/ciliumnodes.cilium.io.json")
        );
        write(&path, None).unwrap();
        assert_eq!(read(&path), Some(None));
        let versions: Versions = [("v2".to_string(), vec![])].into_iter().collect();
        write(&path, Some(&versions)).unwrap();
        assert_eq!(read(&path), Some(Some(versions)));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
