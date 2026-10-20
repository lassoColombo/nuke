pub mod ingressclasses;
pub mod ingresses;
pub mod ipaddresses;
pub mod networkpolicies;
pub mod servicecidrs;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "networking.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1",
        "ingressclasses",
        &ingressclasses::IngressClassFormatter,
    ),
    ("v1", "ingresses", &ingresses::IngressFormatter),
    ("v1", "ipaddresses", &ipaddresses::IPAddressFormatter),
    (
        "v1",
        "networkpolicies",
        &networkpolicies::NetworkPolicyFormatter,
    ),
    ("v1", "servicecidrs", &servicecidrs::ServiceCIDRFormatter),
];
