//! What discovery and the CRD printer columns share on disk: the files of
//! one cluster live under a name made from its URL, are written atomically,
//! and serve for kubectl's discovery TTL.

use std::path::Path;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use http::Uri;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

/// How long a stored answer serves before it is fetched anew: kubectl's
/// discovery TTL, so nuke and kubectl go stale together, and a CRD's
/// columns with its resource.
pub const TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// The cluster at `url` as one path segment: the URL percent-encoded, so
/// that it is reversible whatever the URL holds (a port, a proxy path).
pub fn cluster_name(url: &Uri) -> String {
    const ESCAPE: &AsciiSet = &NON_ALPHANUMERIC.remove(b'.').remove(b'-').remove(b'_');
    let url = url.to_string();
    utf8_percent_encode(url.trim_end_matches('/'), ESCAPE).to_string()
}

/// How long ago `modified` was; zero for an mtime in the future, so a
/// skewed clock does not fetch on every command.
pub fn age(modified: SystemTime) -> Duration {
    SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default()
}

/// Store `bytes` at `path`, atomically: a sibling temporary file renamed
/// into place, so a reader (another nuke process, a completion
/// mid-keystroke) never sees half a file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        anyhow::bail!("not a file path: {}", path.display());
    };
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    std::fs::write(&tmp, bytes)
        .and_then(|()| std::fs::rename(&tmp, path))
        .with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One cluster has one name, whatever is stored under it.
    #[test]
    fn discovery_and_columns_share_the_cluster_name() {
        let root = Path::new("/c");
        let url: Uri = "https://10.0.0.1:6443/k8s/clusters/c-1/".parse().unwrap();
        let name = cluster_name(&url);
        assert_eq!(name, "https%3A%2F%2F10.0.0.1%3A6443%2Fk8s%2Fclusters%2Fc-1");
        assert_eq!(
            crate::discovery::file(root, &url),
            root.join("discovery").join(format!("{name}.json"))
        );
        assert_eq!(
            crate::columns::crd::file(root, &url, "cilium.io", "ciliumnodes"),
            root.join("columns")
                .join(&name)
                .join("ciliumnodes.cilium.io.json")
        );
    }

    #[test]
    fn a_future_mtime_is_no_age() {
        let future = SystemTime::now() + Duration::from_secs(60);
        assert_eq!(age(future), Duration::ZERO);
    }
}
