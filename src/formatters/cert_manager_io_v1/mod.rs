pub mod cert_manager_helpers;
pub mod certificaterequests;
pub mod certificates;
pub mod issuers;

use super::ResourceFormatter;
use issuers::IssuerFormatter;

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
    ("v1", "issuers", &IssuerFormatter { namespaced: true }),
    ("v1", "clusterissuers", &IssuerFormatter { namespaced: false }),
];
