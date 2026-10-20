pub mod gatewayclasses;
pub mod gateways;
pub mod httproutes;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "gateway.networking.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
///
/// The same three kinds are still served under `v1beta1` with an identical
/// shape, so they get the same formatters.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1",
        "gatewayclasses",
        &gatewayclasses::GatewayClassFormatter,
    ),
    ("v1", "gateways", &gateways::GatewayFormatter),
    ("v1", "httproutes", &httproutes::HTTPRouteFormatter),
    (
        "v1beta1",
        "gatewayclasses",
        &gatewayclasses::GatewayClassFormatter,
    ),
    ("v1beta1", "gateways", &gateways::GatewayFormatter),
    ("v1beta1", "httproutes", &httproutes::HTTPRouteFormatter),
];
