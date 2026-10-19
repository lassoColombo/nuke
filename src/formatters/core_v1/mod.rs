pub mod bindings;
pub mod configmaps;
pub mod core_helpers;
pub mod endpoints;
pub mod events;
pub mod limitranges;
pub mod namespaces;
pub mod nodes;
pub mod persistentvolumeclaims;
pub mod persistentvolumes;
pub mod pods;
pub mod podtemplates;
pub mod replicationcontrollers;
pub mod resourcequotas;
pub mod secrets;
pub mod serviceaccounts;
pub mod services;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    ("v1", "nodes", &nodes::NodeFormatter),
    ("v1", "pods", &pods::PodFormatter),
    ("v1", "events", &events::EventFormatter),
    ("v1", "secrets", &secrets::SecretFormatter),
    ("v1", "endpoints", &endpoints::EndpointsFormatter),
    ("v1", "bindings", &bindings::BindingFormatter),
    ("v1", "services", &services::ServiceFormatter),
    ("v1", "limitranges", &limitranges::LimitRangeFormatter),
    (
        "v1",
        "resourcequotas",
        &resourcequotas::ResourceQuotaFormatter,
    ),
    (
        "v1",
        "replicationcontrollers",
        &replicationcontrollers::ReplicationControllerFormatter,
    ),
    (
        "v1",
        "persistentvolumes",
        &persistentvolumes::PersistentVolumeFormatter,
    ),
    (
        "v1",
        "persistentvolumeclaims",
        &persistentvolumeclaims::PersistentVolumeClaimFormatter,
    ),
    (
        "v1",
        "serviceaccounts",
        &serviceaccounts::ServiceAccountFormatter,
    ),
    ("v1", "podtemplates", &podtemplates::PodTemplateFormatter),
    ("v1", "namespaces", &namespaces::NamespaceFormatter),
    ("v1", "configmaps", &configmaps::ConfigMapFormatter),
];
