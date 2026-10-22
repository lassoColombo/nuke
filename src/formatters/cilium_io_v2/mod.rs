//! Formatters for the `cilium.io` API group.
//!
//! Cilium serves most of its CRDs under `v2`; the ones it promoted from
//! `v2alpha1` (BGP, CIDR groups, LB IP pools, node configs, pod IP pools)
//! are registered under both versions, and a few are still `v2alpha1` only
//! (`CiliumL2AnnouncementPolicy`, `CiliumGatewayClassConfig`). All formatters
//! live here regardless of version because their `spec`/`status` shapes are
//! version-stable; the registry maps each GVR to the right one.
pub mod cilium_helpers;
pub mod ciliumbgpadvertisements;
pub mod ciliumbgpclusterconfigs;
pub mod ciliumbgpnodeconfigoverrides;
pub mod ciliumbgpnodeconfigs;
pub mod ciliumbgppeerconfigs;
pub mod ciliumcidrgroups;
pub mod ciliumclusterwideenvoyconfigs;
pub mod ciliumclusterwidenetworkpolicies;
pub mod ciliumegressgatewaypolicies;
pub mod ciliumendpoints;
pub mod ciliumendpointslices;
pub mod ciliumenvoyconfigs;
pub mod ciliumexternalworkloads;
pub mod ciliumgatewayclassconfigs;
pub mod ciliumidentities;
pub mod ciliuml2announcementpolicies;
pub mod ciliumloadbalancerippools;
pub mod ciliumlocalredirectpolicies;
pub mod ciliumnetworkpolicies;
pub mod ciliumnodeconfigs;
pub mod ciliumnodes;
pub mod ciliumpodippools;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "cilium.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    ("v2", "ciliumnodes", &ciliumnodes::CiliumNodeFormatter),
    (
        "v2",
        "ciliumendpoints",
        &ciliumendpoints::CiliumEndpointFormatter,
    ),
    (
        "v2",
        "ciliumidentities",
        &ciliumidentities::CiliumIdentityFormatter,
    ),
    (
        "v2",
        "ciliumexternalworkloads",
        &ciliumexternalworkloads::CiliumExternalWorkloadFormatter,
    ),
    (
        "v2",
        "ciliumnetworkpolicies",
        &ciliumnetworkpolicies::CiliumNetworkPolicyFormatter,
    ),
    (
        "v2",
        "ciliumclusterwidenetworkpolicies",
        &ciliumclusterwidenetworkpolicies::CiliumClusterwideNetworkPolicyFormatter,
    ),
    (
        "v2",
        "ciliumcidrgroups",
        &ciliumcidrgroups::CiliumCIDRGroupFormatter,
    ),
    (
        "v2alpha1",
        "ciliumcidrgroups",
        &ciliumcidrgroups::CiliumCIDRGroupFormatter,
    ),
    (
        "v2",
        "ciliumloadbalancerippools",
        &ciliumloadbalancerippools::CiliumLoadBalancerIPPoolFormatter,
    ),
    (
        "v2alpha1",
        "ciliumloadbalancerippools",
        &ciliumloadbalancerippools::CiliumLoadBalancerIPPoolFormatter,
    ),
    (
        "v2",
        "ciliumnodeconfigs",
        &ciliumnodeconfigs::CiliumNodeConfigFormatter,
    ),
    (
        "v2alpha1",
        "ciliumnodeconfigs",
        &ciliumnodeconfigs::CiliumNodeConfigFormatter,
    ),
    (
        "v2alpha1",
        "ciliuml2announcementpolicies",
        &ciliuml2announcementpolicies::CiliumL2AnnouncementPolicyFormatter,
    ),
    (
        "v2alpha1",
        "ciliumpodippools",
        &ciliumpodippools::CiliumPodIPPoolFormatter,
    ),
    // Storage version since Cilium 1.21.
    (
        "v2",
        "ciliumpodippools",
        &ciliumpodippools::CiliumPodIPPoolFormatter,
    ),
    (
        "v2",
        "ciliumbgpclusterconfigs",
        &ciliumbgpclusterconfigs::CiliumBGPClusterConfigFormatter,
    ),
    (
        "v2alpha1",
        "ciliumbgpclusterconfigs",
        &ciliumbgpclusterconfigs::CiliumBGPClusterConfigFormatter,
    ),
    (
        "v2",
        "ciliumbgppeerconfigs",
        &ciliumbgppeerconfigs::CiliumBGPPeerConfigFormatter,
    ),
    (
        "v2alpha1",
        "ciliumbgppeerconfigs",
        &ciliumbgppeerconfigs::CiliumBGPPeerConfigFormatter,
    ),
    (
        "v2",
        "ciliumbgpadvertisements",
        &ciliumbgpadvertisements::CiliumBGPAdvertisementFormatter,
    ),
    (
        "v2alpha1",
        "ciliumbgpadvertisements",
        &ciliumbgpadvertisements::CiliumBGPAdvertisementFormatter,
    ),
    (
        "v2",
        "ciliumbgpnodeconfigs",
        &ciliumbgpnodeconfigs::CiliumBGPNodeConfigFormatter,
    ),
    (
        "v2alpha1",
        "ciliumbgpnodeconfigs",
        &ciliumbgpnodeconfigs::CiliumBGPNodeConfigFormatter,
    ),
    (
        "v2",
        "ciliumbgpnodeconfigoverrides",
        &ciliumbgpnodeconfigoverrides::CiliumBGPNodeConfigOverrideFormatter,
    ),
    (
        "v2alpha1",
        "ciliumbgpnodeconfigoverrides",
        &ciliumbgpnodeconfigoverrides::CiliumBGPNodeConfigOverrideFormatter,
    ),
    (
        "v2",
        "ciliumenvoyconfigs",
        &ciliumenvoyconfigs::CiliumEnvoyConfigFormatter,
    ),
    (
        "v2",
        "ciliumclusterwideenvoyconfigs",
        &ciliumclusterwideenvoyconfigs::CiliumClusterwideEnvoyConfigFormatter,
    ),
    (
        "v2",
        "ciliumegressgatewaypolicies",
        &ciliumegressgatewaypolicies::CiliumEgressGatewayPolicyFormatter,
    ),
    (
        "v2",
        "ciliumlocalredirectpolicies",
        &ciliumlocalredirectpolicies::CiliumLocalRedirectPolicyFormatter,
    ),
    (
        "v2alpha1",
        "ciliumendpointslices",
        &ciliumendpointslices::CiliumEndpointSliceFormatter,
    ),
    (
        "v2alpha1",
        "ciliumgatewayclassconfigs",
        &ciliumgatewayclassconfigs::CiliumGatewayClassConfigFormatter,
    ),
];
