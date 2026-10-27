pub mod rbac_helpers;
pub mod rolebindings;
pub mod roles;

use super::ResourceFormatter;
use rolebindings::RoleBindingFormatter;
use roles::RoleFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "rbac.authorization.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1",
        "roles",
        &RoleFormatter {
            cluster_role: false,
        },
    ),
    ("v1", "clusterroles", &RoleFormatter { cluster_role: true }),
    ("v1", "rolebindings", &RoleBindingFormatter),
    ("v1", "clusterrolebindings", &RoleBindingFormatter),
];
