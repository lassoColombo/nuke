pub mod admissionregistration_k8s_io_v1;
pub mod apiextensions_k8s_io_v1;
pub mod apiregistration_k8s_io_v1;
pub mod apps_v1;
pub mod argoproj_io_v1alpha1;
pub mod autoscaling_v2;
pub mod batch_v1;
pub mod cert_manager_io_v1;
pub mod certificates_k8s_io_v1;
pub mod cilium_io_v2;
pub mod coordination_k8s_io_v1;
pub mod core_v1;
pub mod default;
pub mod discovery_k8s_io_v1;
pub mod events_k8s_io_v1;
pub mod flowcontrol_k8s_io_v1;
pub mod helpers;
pub mod hub_traefik_io_v1alpha1;
pub mod monitoring_coreos_com_v1;
pub mod networking_k8s_io_v1;
pub mod node_k8s_io_v1;
pub mod policy_v1;
pub mod rbac_k8s_io_v1;
pub mod resource_k8s_io_v1;
pub mod scheduling_k8s_io_v1;
pub mod storage_k8s_io_v1;
pub mod traefik_io_v1alpha1;

use kube::api::DynamicObject;
use nu_protocol::{Span, Value};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Output format
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Compact,
    Wide,
    Full,
}

impl std::str::FromStr for OutputFormat {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "compact" => Ok(Self::Compact),
            "wide" => Ok(Self::Wide),
            "full" => Ok(Self::Full),
            _ => Err(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Formatter trait
// ---------------------------------------------------------------------------

/// A formatter knows how to turn a `DynamicObject` into a nushell `Value`
/// in two display densities.
pub trait ResourceFormatter: Send + Sync {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value;

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        self.format_compact(item, span)
    }

    fn format(&self, item: &DynamicObject, span: Span, mode: OutputFormat) -> Value {
        match mode {
            OutputFormat::Wide => self.format_wide(item, span),
            OutputFormat::Compact => self.format_compact(item, span),
            OutputFormat::Full => unreachable!("Full must be handled before formatter dispatch"),
        }
    }
}

// ---------------------------------------------------------------------------
// Registry key
// ---------------------------------------------------------------------------

/// Canonical lookup key: (group, version, plural).
/// An empty `group` string means the core API group.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FormatterKey {
    pub group: String,
    pub version: String,
    pub plural: String,
}

impl FormatterKey {
    pub fn new(
        group: impl Into<String>,
        version: impl Into<String>,
        plural: impl Into<String>,
    ) -> Self {
        Self {
            group: group.into(),
            version: version.into(),
            plural: plural.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Holds all registered formatters indexed by `FormatterKey`.
pub struct FormatterRegistry {
    formatters: HashMap<FormatterKey, Box<dyn ResourceFormatter>>,
    default: Box<dyn ResourceFormatter>,
}

impl FormatterRegistry {
    /// Build the global registry with all built-in formatters registered.
    pub fn new() -> Self {
        let mut reg = Self {
            formatters: HashMap::new(),
            default: Box::new(default::DefaultFormatter),
        };
        reg.register_builtins();
        reg
    }

    /// Register a formatter for an exact GVR triple.
    pub fn register(&mut self, key: FormatterKey, formatter: impl ResourceFormatter + 'static) {
        self.formatters.insert(key, Box::new(formatter));
    }

    /// Look up a formatter.  Resolution order:
    ///   1. Exact (group, version, plural)
    ///   2. Wildcard version  (group, "*", plural)
    ///   3. Built-in default
    pub fn get(&self, group: &str, version: &str, plural: &str) -> &dyn ResourceFormatter {
        let exact = FormatterKey::new(group, version, plural);
        if let Some(f) = self.formatters.get(&exact) {
            return f.as_ref();
        }

        let wildcard = FormatterKey::new(group, "*", plural);
        if let Some(f) = self.formatters.get(&wildcard) {
            return f.as_ref();
        }

        self.default.as_ref()
    }

    // -----------------------------------------------------------------------
    // Built-in registrations
    // -----------------------------------------------------------------------

    fn register_builtins(&mut self) {
        // admissionregistration
        use admissionregistration_k8s_io_v1::mutatingwebhookconfigurations::MutatingWebhookConfigurationFormatter;
        use admissionregistration_k8s_io_v1::validatingadmissionpolicies::ValidatingAdmissionPolicyFormatter;
        use admissionregistration_k8s_io_v1::validatingadmissionpolicybindings::ValidatingAdmissionPolicyBindingFormatter;
        use admissionregistration_k8s_io_v1::validatingwebhookconfigurations::ValidatingWebhookConfigurationFormatter;
        self.register(
            FormatterKey::new(
                "admissionregistration.k8s.io",
                "v1",
                "mutatingwebhookconfigurations",
            ),
            MutatingWebhookConfigurationFormatter,
        );
        self.register(
            FormatterKey::new(
                "admissionregistration.k8s.io",
                "v1",
                "validatingwebhookconfigurations",
            ),
            ValidatingWebhookConfigurationFormatter,
        );
        self.register(
            FormatterKey::new(
                "admissionregistration.k8s.io",
                "v1",
                "validatingadmissionpolicies",
            ),
            ValidatingAdmissionPolicyFormatter,
        );
        self.register(
            FormatterKey::new(
                "admissionregistration.k8s.io",
                "v1",
                "validatingadmissionpolicybindings",
            ),
            ValidatingAdmissionPolicyBindingFormatter,
        );

        // apiextensions
        use apiextensions_k8s_io_v1::customresourcedefinitions::CustomResourceDefinitionFormatter;
        self.register(
            FormatterKey::new("apiextensions.k8s.io", "v1", "customresourcedefinitions"),
            CustomResourceDefinitionFormatter,
        );

        // apiregistration
        use apiregistration_k8s_io_v1::apiservices::APIServiceFormatter;
        self.register(
            FormatterKey::new("apiregistration.k8s.io", "v1", "apiservices"),
            APIServiceFormatter,
        );

        // autoscaling
        use autoscaling_v2::horizontalpodautoscalers::HorizontalPodAutoscalerFormatter;
        self.register(
            FormatterKey::new("autoscaling", "v1", "horizontalpodautoscalers"),
            HorizontalPodAutoscalerFormatter,
        );
        self.register(
            FormatterKey::new("autoscaling", "v2", "horizontalpodautoscalers"),
            HorizontalPodAutoscalerFormatter,
        );

        // events.k8s.io
        use events_k8s_io_v1::events::EventV1Formatter;
        self.register(
            FormatterKey::new("events.k8s.io", "v1", "events"),
            EventV1Formatter,
        );

        // certificates
        use certificates_k8s_io_v1::certificatesigningrequests::CertificateSigningRequestFormatter;
        self.register(
            FormatterKey::new(
                "certificates.k8s.io",
                "v1",
                "certificatesigningrequests",
            ),
            CertificateSigningRequestFormatter,
        );

        // coordination
        use coordination_k8s_io_v1::leases::LeaseFormatter;
        self.register(
            FormatterKey::new("coordination.k8s.io", "v1", "leases"),
            LeaseFormatter,
        );

        // discovery
        use discovery_k8s_io_v1::endpointslices::EndpointSliceFormatter;
        self.register(
            FormatterKey::new("discovery.k8s.io", "v1", "endpointslices"),
            EndpointSliceFormatter,
        );

        // node
        use node_k8s_io_v1::runtimeclasses::RuntimeClassFormatter;
        self.register(
            FormatterKey::new("node.k8s.io", "v1", "runtimeclasses"),
            RuntimeClassFormatter,
        );

        // policy
        use policy_v1::poddisruptionbudgets::PodDisruptionBudgetFormatter;
        self.register(
            FormatterKey::new("policy", "v1", "poddisruptionbudgets"),
            PodDisruptionBudgetFormatter,
        );

        // rbac
        use rbac_k8s_io_v1::clusterrolebindings::ClusterRoleBindingFormatter;
        use rbac_k8s_io_v1::clusterroles::ClusterRoleFormatter;
        use rbac_k8s_io_v1::rolebindings::RoleBindingFormatter;
        use rbac_k8s_io_v1::roles::RoleFormatter;
        self.register(
            FormatterKey::new("rbac.authorization.k8s.io", "v1", "roles"),
            RoleFormatter,
        );
        self.register(
            FormatterKey::new("rbac.authorization.k8s.io", "v1", "clusterrolebindings"),
            ClusterRoleBindingFormatter,
        );
        self.register(
            FormatterKey::new("rbac.authorization.k8s.io", "v1", "clusterroles"),
            ClusterRoleFormatter,
        );
        self.register(
            FormatterKey::new("rbac.authorization.k8s.io", "v1", "rolebindings"),
            RoleBindingFormatter,
        );

        // storage
        use storage_k8s_io_v1::csidrivers::CSIDriverFormatter;
        use storage_k8s_io_v1::csinodes::CSINodeFormatter;
        use storage_k8s_io_v1::csistoragecapacities::CSIStorageCapacityFormatter;
        use storage_k8s_io_v1::storageclasses::StorageClassFormatter;
        use storage_k8s_io_v1::volumeattachments::VolumeAttachmentFormatter;
        use storage_k8s_io_v1::volumeattributeclasses::VolumeAttributesClassFormatter;
        self.register(
            FormatterKey::new("storage.k8s.io", "v1", "csidrivers"),
            CSIDriverFormatter,
        );
        self.register(
            FormatterKey::new("storage.k8s.io", "v1", "csinodes"),
            CSINodeFormatter,
        );
        self.register(
            FormatterKey::new("storage.k8s.io", "v1", "csistoragecapacities"),
            CSIStorageCapacityFormatter,
        );
        self.register(
            FormatterKey::new("storage.k8s.io", "v1", "storageclasses"),
            StorageClassFormatter,
        );
        self.register(
            FormatterKey::new("storage.k8s.io", "v1", "volumeattachments"),
            VolumeAttachmentFormatter,
        );
        self.register(
            FormatterKey::new("storage.k8s.io", "v1", "volumeattributeclasses"),
            VolumeAttributesClassFormatter,
        );
        // scheduling
        use scheduling_k8s_io_v1::priorityclasses::PriorityClassFormatter;
        self.register(
            FormatterKey::new("scheduling.k8s.io", "v1", "priorityclasses"),
            PriorityClassFormatter,
        );
        // networking
        use networking_k8s_io_v1::ingressclasses::IngressClassFormatter;
        use networking_k8s_io_v1::ingresses::IngressFormatter;
        use networking_k8s_io_v1::ipaddresses::IPAddressFormatter;
        use networking_k8s_io_v1::networkpolicies::NetworkPolicyFormatter;
        use networking_k8s_io_v1::servicecidr::ServiceCIDRFormatter;
        self.register(
            FormatterKey::new("networking.k8s.io", "v1", "ingressclasses"),
            IngressClassFormatter,
        );
        self.register(
            FormatterKey::new("networking.k8s.io", "v1", "ingresses"),
            IngressFormatter,
        );
        self.register(
            FormatterKey::new("networking.k8s.io", "v1", "ipaddresses"),
            IPAddressFormatter,
        );
        self.register(
            FormatterKey::new("networking.k8s.io", "v1", "networkpolicies"),
            NetworkPolicyFormatter,
        );
        self.register(
            FormatterKey::new("networking.k8s.io", "v1", "servicecidr"),
            ServiceCIDRFormatter,
        );

        // flowcontrol
        use flowcontrol_k8s_io_v1::flowschemas::FlowSchemaFormatter;
        use flowcontrol_k8s_io_v1::prioritylevelconfigurations::PriorityLevelConfigurationFormatter;
        self.register(
            FormatterKey::new("flowcontrol.apiserver.k8s.io", "v1", "flowschemas"),
            FlowSchemaFormatter,
        );
        self.register(
            FormatterKey::new(
                "flowcontrol.apiserver.k8s.io",
                "v1",
                "prioritylevelconfigurations",
            ),
            PriorityLevelConfigurationFormatter,
        );

        // resource (DRA)
        use resource_k8s_io_v1::deviceclasses::DeviceClassFormatter;
        use resource_k8s_io_v1::resourceclaims::ResourceClaimFormatter;
        use resource_k8s_io_v1::resourceclaimtemplates::ResourceClaimTemplateFormatter;
        use resource_k8s_io_v1::resourceslices::ResourceSliceFormatter;
        self.register(
            FormatterKey::new("resource.k8s.io", "v1", "deviceclasses"),
            DeviceClassFormatter,
        );
        self.register(
            FormatterKey::new("resource.k8s.io", "v1", "resourceclaims"),
            ResourceClaimFormatter,
        );
        self.register(
            FormatterKey::new("resource.k8s.io", "v1", "resourceclaimtemplates"),
            ResourceClaimTemplateFormatter,
        );
        self.register(
            FormatterKey::new("resource.k8s.io", "v1", "resourceslices"),
            ResourceSliceFormatter,
        );

        // batch
        use batch_v1::cronjobs::CronJobFormatter;
        use batch_v1::jobs::JobFormatter;
        self.register(FormatterKey::new("batch", "v1", "jobs"), JobFormatter);
        self.register(
            FormatterKey::new("batch", "v1", "cronjobs"),
            CronJobFormatter,
        );
        // apps
        use apps_v1::controllerrevisions::ControllerRevisionFormatter;
        use apps_v1::daemonsets::DaemonSetFormatter;
        use apps_v1::deployments::DeploymentFormatter;
        use apps_v1::replicasets::ReplicaSetFormatter;
        use apps_v1::statefulsets::StatefulSetFormatter;
        self.register(
            FormatterKey::new("apps", "v1", "statefulsets"),
            StatefulSetFormatter,
        );
        self.register(
            FormatterKey::new("apps", "v1", "replicasets"),
            ReplicaSetFormatter,
        );
        self.register(
            FormatterKey::new("apps", "v1", "deployments"),
            DeploymentFormatter,
        );
        self.register(
            FormatterKey::new("apps", "v1", "daemonsets"),
            DaemonSetFormatter,
        );
        self.register(
            FormatterKey::new("apps", "v1", "controllerrevisions"),
            ControllerRevisionFormatter,
        );

        // core
        use core_v1::bindings::BindingFormatter;
        use core_v1::configmaps::ConfigMapFormatter;
        use core_v1::endpoints::EndpointsFormatter;
        use core_v1::events::EventFormatter;
        use core_v1::limitranges::LimitRangeFormatter;
        use core_v1::namespaces::NamespaceFormatter;
        use core_v1::nodes::NodeFormatter;
        use core_v1::persistentvolumeclaims::PersistentVolumeClaimFormatter;
        use core_v1::persistentvolumes::PersistentVolumeFormatter;
        use core_v1::pods::PodFormatter;
        use core_v1::podtemplates::PodTemplateFormatter;
        use core_v1::replicationcontrollers::ReplicationControllerFormatter;
        use core_v1::resourcequotas::ResourceQuotaFormatter;
        use core_v1::secrets::SecretFormatter;
        use core_v1::serviceaccounts::ServiceAccountFormatter;
        use core_v1::services::ServiceFormatter;
        self.register(FormatterKey::new("", "v1", "nodes"), NodeFormatter);
        self.register(FormatterKey::new("", "v1", "pods"), PodFormatter);
        self.register(FormatterKey::new("", "v1", "events"), EventFormatter);
        self.register(FormatterKey::new("", "v1", "secrets"), SecretFormatter);
        self.register(FormatterKey::new("", "v1", "endpoints"), EndpointsFormatter);
        self.register(FormatterKey::new("", "v1", "bindings"), BindingFormatter);
        self.register(FormatterKey::new("", "v1", "services"), ServiceFormatter);
        self.register(
            FormatterKey::new("", "v1", "limitranges"),
            LimitRangeFormatter,
        );
        self.register(
            FormatterKey::new("", "v1", "resourcequotas"),
            ResourceQuotaFormatter,
        );
        self.register(
            FormatterKey::new("", "v1", "replicationcontrollers"),
            ReplicationControllerFormatter,
        );
        self.register(
            FormatterKey::new("", "v1", "persistentvolumes"),
            PersistentVolumeFormatter,
        );
        self.register(
            FormatterKey::new("", "v1", "persistentvolumeclaims"),
            PersistentVolumeClaimFormatter,
        );
        self.register(
            FormatterKey::new("", "v1", "serviceaccounts"),
            ServiceAccountFormatter,
        );
        self.register(
            FormatterKey::new("", "v1", "podtemplates"),
            PodTemplateFormatter,
        );
        self.register(
            FormatterKey::new("", "v1", "namespaces"),
            NamespaceFormatter,
        );
        self.register(
            FormatterKey::new("", "v1", "configmaps"),
            ConfigMapFormatter,
        );

        // argoproj.io
        use argoproj_io_v1alpha1::applications::ApplicationFormatter;
        use argoproj_io_v1alpha1::appprojects::AppProjectFormatter;
        self.register(
            FormatterKey::new("argoproj.io", "v1alpha1", "applications"),
            ApplicationFormatter,
        );
        self.register(
            FormatterKey::new("argoproj.io", "v1alpha1", "appprojects"),
            AppProjectFormatter,
        );

        // cert-manager.io
        use cert_manager_io_v1::certificaterequests::CertificateRequestFormatter;
        use cert_manager_io_v1::certificates::CertificateFormatter;
        use cert_manager_io_v1::clusterissuers::ClusterIssuerFormatter;
        use cert_manager_io_v1::issuers::IssuerFormatter;
        self.register(
            FormatterKey::new("cert-manager.io", "v1", "certificates"),
            CertificateFormatter,
        );
        self.register(
            FormatterKey::new("cert-manager.io", "v1", "certificaterequests"),
            CertificateRequestFormatter,
        );
        self.register(
            FormatterKey::new("cert-manager.io", "v1", "clusterissuers"),
            ClusterIssuerFormatter,
        );
        self.register(
            FormatterKey::new("cert-manager.io", "v1", "issuers"),
            IssuerFormatter,
        );

        // monitoring.coreos.com
        use monitoring_coreos_com_v1::alertmanagers::AlertmanagerFormatter;
        use monitoring_coreos_com_v1::podmonitors::PodMonitorFormatter;
        use monitoring_coreos_com_v1::prometheuses::PrometheusFormatter;
        use monitoring_coreos_com_v1::prometheusrules::PrometheusRuleFormatter;
        use monitoring_coreos_com_v1::servicemonitors::ServiceMonitorFormatter;
        self.register(
            FormatterKey::new("monitoring.coreos.com", "v1", "alertmanagers"),
            AlertmanagerFormatter,
        );
        self.register(
            FormatterKey::new("monitoring.coreos.com", "v1", "podmonitors"),
            PodMonitorFormatter,
        );
        self.register(
            FormatterKey::new("monitoring.coreos.com", "v1", "prometheuses"),
            PrometheusFormatter,
        );
        self.register(
            FormatterKey::new("monitoring.coreos.com", "v1", "prometheusrules"),
            PrometheusRuleFormatter,
        );
        self.register(
            FormatterKey::new("monitoring.coreos.com", "v1", "servicemonitors"),
            ServiceMonitorFormatter,
        );

        // cilium.io
        //
        // Cilium serves most CRDs under v2; a few are served under v2alpha1, and
        // three (CIDRGroup, LoadBalancerIPPool, NodeConfig) are served under both.
        // We register every served version explicitly so lookups resolve
        // regardless of which version discovery selects.
        use cilium_io_v2::ciliumcidrgroups::CiliumCIDRGroupFormatter;
        use cilium_io_v2::ciliumclusterwidenetworkpolicies::CiliumClusterwideNetworkPolicyFormatter;
        use cilium_io_v2::ciliumendpoints::CiliumEndpointFormatter;
        use cilium_io_v2::ciliumexternalworkloads::CiliumExternalWorkloadFormatter;
        use cilium_io_v2::ciliumidentities::CiliumIdentityFormatter;
        use cilium_io_v2::ciliuml2announcementpolicies::CiliumL2AnnouncementPolicyFormatter;
        use cilium_io_v2::ciliumloadbalancerippools::CiliumLoadBalancerIPPoolFormatter;
        use cilium_io_v2::ciliumnetworkpolicies::CiliumNetworkPolicyFormatter;
        use cilium_io_v2::ciliumnodeconfigs::CiliumNodeConfigFormatter;
        use cilium_io_v2::ciliumnodes::CiliumNodeFormatter;
        use cilium_io_v2::ciliumpodippools::CiliumPodIPPoolFormatter;
        // BGP (served under both v2 and v2alpha1)
        use cilium_io_v2::ciliumbgpadvertisements::CiliumBGPAdvertisementFormatter;
        use cilium_io_v2::ciliumbgpclusterconfigs::CiliumBGPClusterConfigFormatter;
        use cilium_io_v2::ciliumbgpnodeconfigoverrides::CiliumBGPNodeConfigOverrideFormatter;
        use cilium_io_v2::ciliumbgpnodeconfigs::CiliumBGPNodeConfigFormatter;
        use cilium_io_v2::ciliumbgppeerconfigs::CiliumBGPPeerConfigFormatter;
        // envoy / egress / local-redirect (v2) and endpoint-slices / gateway-class (v2alpha1)
        use cilium_io_v2::ciliumclusterwideenvoyconfigs::CiliumClusterwideEnvoyConfigFormatter;
        use cilium_io_v2::ciliumegressgatewaypolicies::CiliumEgressGatewayPolicyFormatter;
        use cilium_io_v2::ciliumendpointslices::CiliumEndpointSliceFormatter;
        use cilium_io_v2::ciliumenvoyconfigs::CiliumEnvoyConfigFormatter;
        use cilium_io_v2::ciliumgatewayclassconfigs::CiliumGatewayClassConfigFormatter;
        use cilium_io_v2::ciliumlocalredirectpolicies::CiliumLocalRedirectPolicyFormatter;
        // v2-only resources
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumnodes"),
            CiliumNodeFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumendpoints"),
            CiliumEndpointFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumidentities"),
            CiliumIdentityFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumexternalworkloads"),
            CiliumExternalWorkloadFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumnetworkpolicies"),
            CiliumNetworkPolicyFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumclusterwidenetworkpolicies"),
            CiliumClusterwideNetworkPolicyFormatter,
        );
        // resources served under both v2 and v2alpha1
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumcidrgroups"),
            CiliumCIDRGroupFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2alpha1", "ciliumcidrgroups"),
            CiliumCIDRGroupFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumloadbalancerippools"),
            CiliumLoadBalancerIPPoolFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2alpha1", "ciliumloadbalancerippools"),
            CiliumLoadBalancerIPPoolFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumnodeconfigs"),
            CiliumNodeConfigFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2alpha1", "ciliumnodeconfigs"),
            CiliumNodeConfigFormatter,
        );
        // v2alpha1-only resources
        self.register(
            FormatterKey::new("cilium.io", "v2alpha1", "ciliuml2announcementpolicies"),
            CiliumL2AnnouncementPolicyFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2alpha1", "ciliumpodippools"),
            CiliumPodIPPoolFormatter,
        );
        // BGP control-plane (served under both v2 and v2alpha1)
        for ver in ["v2", "v2alpha1"] {
            self.register(
                FormatterKey::new("cilium.io", ver, "ciliumbgpclusterconfigs"),
                CiliumBGPClusterConfigFormatter,
            );
            self.register(
                FormatterKey::new("cilium.io", ver, "ciliumbgppeerconfigs"),
                CiliumBGPPeerConfigFormatter,
            );
            self.register(
                FormatterKey::new("cilium.io", ver, "ciliumbgpadvertisements"),
                CiliumBGPAdvertisementFormatter,
            );
            self.register(
                FormatterKey::new("cilium.io", ver, "ciliumbgpnodeconfigs"),
                CiliumBGPNodeConfigFormatter,
            );
            self.register(
                FormatterKey::new("cilium.io", ver, "ciliumbgpnodeconfigoverrides"),
                CiliumBGPNodeConfigOverrideFormatter,
            );
        }
        // envoy / egress / local-redirect (v2-only)
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumenvoyconfigs"),
            CiliumEnvoyConfigFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumclusterwideenvoyconfigs"),
            CiliumClusterwideEnvoyConfigFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumegressgatewaypolicies"),
            CiliumEgressGatewayPolicyFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2", "ciliumlocalredirectpolicies"),
            CiliumLocalRedirectPolicyFormatter,
        );
        // endpoint-slices / gateway-class config (v2alpha1-only)
        self.register(
            FormatterKey::new("cilium.io", "v2alpha1", "ciliumendpointslices"),
            CiliumEndpointSliceFormatter,
        );
        self.register(
            FormatterKey::new("cilium.io", "v2alpha1", "ciliumgatewayclassconfigs"),
            CiliumGatewayClassConfigFormatter,
        );

        // traefik.io (Traefik proxy CRDs — all namespaced, v1alpha1)
        use traefik_io_v1alpha1::ingressroutes::IngressRouteFormatter;
        use traefik_io_v1alpha1::ingressroutetcps::IngressRouteTCPFormatter;
        use traefik_io_v1alpha1::ingressrouteudps::IngressRouteUDPFormatter;
        use traefik_io_v1alpha1::middlewares::MiddlewareFormatter;
        use traefik_io_v1alpha1::middlewaretcps::MiddlewareTCPFormatter;
        use traefik_io_v1alpha1::serverstransports::ServersTransportFormatter;
        use traefik_io_v1alpha1::serverstransporttcps::ServersTransportTCPFormatter;
        use traefik_io_v1alpha1::tlsoptions::TLSOptionFormatter;
        use traefik_io_v1alpha1::tlsstores::TLSStoreFormatter;
        use traefik_io_v1alpha1::traefikservices::TraefikServiceFormatter;
        self.register(
            FormatterKey::new("traefik.io", "v1alpha1", "ingressroutes"),
            IngressRouteFormatter,
        );
        self.register(
            FormatterKey::new("traefik.io", "v1alpha1", "ingressroutetcps"),
            IngressRouteTCPFormatter,
        );
        self.register(
            FormatterKey::new("traefik.io", "v1alpha1", "ingressrouteudps"),
            IngressRouteUDPFormatter,
        );
        self.register(
            FormatterKey::new("traefik.io", "v1alpha1", "middlewares"),
            MiddlewareFormatter,
        );
        self.register(
            FormatterKey::new("traefik.io", "v1alpha1", "middlewaretcps"),
            MiddlewareTCPFormatter,
        );
        self.register(
            FormatterKey::new("traefik.io", "v1alpha1", "serverstransports"),
            ServersTransportFormatter,
        );
        self.register(
            FormatterKey::new("traefik.io", "v1alpha1", "serverstransporttcps"),
            ServersTransportTCPFormatter,
        );
        self.register(
            FormatterKey::new("traefik.io", "v1alpha1", "tlsoptions"),
            TLSOptionFormatter,
        );
        self.register(
            FormatterKey::new("traefik.io", "v1alpha1", "tlsstores"),
            TLSStoreFormatter,
        );
        self.register(
            FormatterKey::new("traefik.io", "v1alpha1", "traefikservices"),
            TraefikServiceFormatter,
        );

        // hub.traefik.io (Traefik Hub API-management CRDs — all v1alpha1;
        // namespaced except AccessControlPolicy)
        use hub_traefik_io_v1alpha1::accesscontrolpolicies::AccessControlPolicyFormatter;
        use hub_traefik_io_v1alpha1::aiservices::AIServiceFormatter;
        use hub_traefik_io_v1alpha1::apiaccesses::APIAccessFormatter;
        use hub_traefik_io_v1alpha1::apiauths::APIAuthFormatter;
        use hub_traefik_io_v1alpha1::apibundles::APIBundleFormatter;
        use hub_traefik_io_v1alpha1::apicatalogitems::APICatalogItemFormatter;
        use hub_traefik_io_v1alpha1::apiplans::APIPlanFormatter;
        use hub_traefik_io_v1alpha1::apiportalauths::APIPortalAuthFormatter;
        use hub_traefik_io_v1alpha1::apiportals::APIPortalFormatter;
        use hub_traefik_io_v1alpha1::apiratelimits::APIRateLimitFormatter;
        use hub_traefik_io_v1alpha1::apis::APIFormatter;
        use hub_traefik_io_v1alpha1::apiversions::APIVersionFormatter;
        use hub_traefik_io_v1alpha1::managedapplications::ManagedApplicationFormatter;
        use hub_traefik_io_v1alpha1::managedsubscriptions::ManagedSubscriptionFormatter;
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "accesscontrolpolicies"),
            AccessControlPolicyFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "aiservices"),
            AIServiceFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "apiaccesses"),
            APIAccessFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "apiauths"),
            APIAuthFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "apibundles"),
            APIBundleFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "apicatalogitems"),
            APICatalogItemFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "apiplans"),
            APIPlanFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "apiportalauths"),
            APIPortalAuthFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "apiportals"),
            APIPortalFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "apiratelimits"),
            APIRateLimitFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "apis"),
            APIFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "apiversions"),
            APIVersionFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "managedapplications"),
            ManagedApplicationFormatter,
        );
        self.register(
            FormatterKey::new("hub.traefik.io", "v1alpha1", "managedsubscriptions"),
            ManagedSubscriptionFormatter,
        );
    }
}

impl Default for FormatterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
