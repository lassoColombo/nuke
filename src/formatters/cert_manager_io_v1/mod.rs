pub mod cert_manager_helpers;
pub mod certificaterequests;
pub mod certificates;
pub mod clusterissuers;
pub mod issuers;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "cert-manager.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    ("v1", "certificates", &certificates::CertificateFormatter),
    (
        "v1",
        "certificaterequests",
        &certificaterequests::CertificateRequestFormatter,
    ),
    (
        "v1",
        "clusterissuers",
        &clusterissuers::ClusterIssuerFormatter,
    ),
    ("v1", "issuers", &issuers::IssuerFormatter),
];
