pub mod gatewayclasses;
pub mod gateways;
pub mod httproutes;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "gateway.networking.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1",
        "gatewayclasses",
        &gatewayclasses::GatewayClassFormatter,
    ),
    ("v1", "gateways", &gateways::GatewayFormatter),
    ("v1", "httproutes", &httproutes::HTTPRouteFormatter),
];
