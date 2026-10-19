pub mod clusterrolebindings;
pub mod clusterroles;
pub mod rbac_helpers;
pub mod rolebindings;
pub mod roles;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "rbac.authorization.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    ("v1", "roles", &roles::RoleFormatter),
    (
        "v1",
        "clusterrolebindings",
        &clusterrolebindings::ClusterRoleBindingFormatter,
    ),
    ("v1", "clusterroles", &clusterroles::ClusterRoleFormatter),
    ("v1", "rolebindings", &rolebindings::RoleBindingFormatter),
];
