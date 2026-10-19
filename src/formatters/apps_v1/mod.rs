pub mod controllerrevisions;
pub mod daemonsets;
pub mod deployments;
pub mod replicasets;
pub mod statefulsets;
pub mod workload_helpers;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "apps";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    ("v1", "statefulsets", &statefulsets::StatefulSetFormatter),
    ("v1", "replicasets", &replicasets::ReplicaSetFormatter),
    ("v1", "deployments", &deployments::DeploymentFormatter),
    ("v1", "daemonsets", &daemonsets::DaemonSetFormatter),
    (
        "v1",
        "controllerrevisions",
        &controllerrevisions::ControllerRevisionFormatter,
    ),
];
