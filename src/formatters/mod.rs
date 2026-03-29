pub mod admissionregistration_k8s_io_v1;
pub mod apiextensions_k8s_io_v1;
pub mod apiregistration_k8s_io_v1;
pub mod apps_v1;
pub mod autoscaling_v2;
pub mod batch_v1;
pub mod certificates_k8s_io_v1;
pub mod coordination_k8s_io_v1;
pub mod core_v1;
pub mod default;
pub mod discovery_k8s_io_v1;
pub mod events_k8s_io_v1;
pub mod flowcontrol_k8s_io_v1;
pub mod helpers;
pub mod networking_k8s_io_v1;
pub mod node_k8s_io_v1;
pub mod policy_v1;
pub mod rbac_k8s_io_v1;
pub mod resource_k8s_io_v1;
pub mod scheduling_k8s_io_v1;
pub mod storage_k8s_io_v1;

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

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "compact" => Some(Self::Compact),
            "wide" => Some(Self::Wide),
            "full" => Some(Self::Full),
            _ => None,
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
    }
}

impl Default for FormatterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
