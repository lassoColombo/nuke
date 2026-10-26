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
    ("v1", "roles", &RoleFormatter { namespaced: true }),
    ("v1", "clusterroles", &RoleFormatter { namespaced: false }),
    (
        "v1",
        "rolebindings",
        &RoleBindingFormatter { namespaced: true },
    ),
    (
        "v1",
        "clusterrolebindings",
        &RoleBindingFormatter { namespaced: false },
    ),
];
