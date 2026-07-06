//! Formatters for the `cilium.io` API group.
//!
//! Cilium serves most of its CRDs under `v2`, with a few still only under
//! `v2alpha1` (`CiliumL2AnnouncementPolicy`, `CiliumPodIPPool`) and a few
//! served under both (`CiliumCIDRGroup`, `CiliumLoadBalancerIPPool`,
//! `CiliumNodeConfig`).  All formatters live here regardless of version because
//! their `spec`/`status` shapes are version-stable; the registry maps each GVR
//! to the right one.
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
