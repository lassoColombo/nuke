//! Nuke's column definitions for the built-in resources, in the shape a
//! CRD declares its printer columns: kubectl's column name, a JSONPath
//! over the object, a type, a priority. kubectl computes these cells in Go
//! (`printers.go`); here each is the field it prints, read natively, so a
//! list is a list and a count an int. The oracle test checks every
//! definition against the cell the apiserver printed for the same object,
//! so a definition that drifts from kubectl's printer fails the build.
//!
//! Compact rows are flat: a scalar or a list of scalars per column, so a
//! list renders one line per object. A record or a list of records (a
//! Service's `ports`, an Endpoints' `subsets`) is wide, where the compact
//! row carries its scalars (`port`, `protocol`, `addresses`). A cell kubectl
//! derives from more than one field with no single source (a pod's STATUS,
//! a node's ROLES), or prints as one `kind/name` or `k=v` text from a
//! sub-object (an Event's OBJECT, a selector), reads the apiserver's cell
//! through a declared grammar, and says so with [`cell`]; the sub-object
//! itself is wide under its API name.

use super::{Agg, Column, Dflt, Source, Ty};

/// One definition, as written in [`BUILTIN`].
#[derive(Debug, Clone, Copy)]
pub struct Def {
    name: &'static str,
    source: &'static str,
    /// Paths tried after `source`, in order.
    fallback: &'static [&'static str],
    from_cell: bool,
    ty: Ty,
    agg: Agg,
    default: Dflt,
    priority: i64,
}

/// A column read from the object by `path`, kubectl's `name`.
pub const fn path(name: &'static str, path: &'static str) -> Def {
    Def {
        name,
        source: path,
        fallback: &[],
        from_cell: false,
        ty: Ty::Native,
        agg: Agg::First,
        default: Dflt::None,
        priority: 0,
    }
}

/// A column read from the apiserver's cell for kubectl's column `name`.
pub const fn cell(name: &'static str) -> Def {
    Def {
        name,
        source: name,
        fallback: &[],
        from_cell: true,
        ty: Ty::Text,
        agg: Agg::First,
        default: Dflt::None,
        priority: 0,
    }
}

impl Def {
    /// The column under nuke's own `name`, when kubectl's does not fit (a
    /// second column read from the same cell).
    pub const fn named(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    /// Paths tried in order when the first reaches nothing (or only null),
    /// as kubectl falls back from one field to the next.
    pub const fn or(mut self, paths: &'static [&'static str]) -> Self {
        self.fallback = paths;
        self
    }

    /// A wide column (kubectl's `-o wide`).
    pub const fn wide(mut self) -> Self {
        self.priority = 1;
        self
    }

    pub const fn ty(mut self, ty: Ty) -> Self {
        self.ty = ty;
        self
    }

    /// Every node the path reaches, as a list.
    pub const fn all(mut self) -> Self {
        self.agg = Agg::All;
        self
    }

    pub const fn count(mut self) -> Self {
        self.agg = Agg::Count;
        self
    }

    pub const fn max(mut self) -> Self {
        self.agg = Agg::Max;
        self
    }

    /// `0` when the field is absent: a status counter.
    pub const fn zero(mut self) -> Self {
        self.default = Dflt::Zero;
        self
    }

    /// `1` when the field is absent: an Event's count, which the new API
    /// leaves unset on a singleton.
    pub const fn one(mut self) -> Self {
        self.default = Dflt::One;
        self
    }

    /// `false` when the field is absent.
    pub const fn falsy(mut self) -> Self {
        self.default = Dflt::False;
        self
    }

    pub fn column(&self) -> Column {
        Column {
            name: self.name.into(),
            source: match self.from_cell {
                true => Source::Cell(self.source.into()),
                false => Source::Path(
                    std::iter::once(self.source)
                        .chain(self.fallback.iter().copied())
                        .map(String::from)
                        .collect(),
                ),
            },
            ty: self.ty,
            agg: self.agg,
            default: self.default,
            priority: self.priority,
        }
    }
}

/// The definitions of `group/version/plural`, when it is a built-in nuke
/// knows.
pub fn find(group: &str, version: &str, plural: &str) -> Option<&'static [Def]> {
    BUILTIN
        .iter()
        .find(|(g, v, p, _)| *g == group && *v == version && *p == plural)
        .map(|(_, _, _, defs)| *defs)
}

/// Every built-in resource with its definitions, compact ones first.
#[cfg(test)]
pub fn all() -> impl Iterator<Item = (&'static str, &'static str, &'static str, &'static [Def])> {
    BUILTIN.iter().copied()
}

const CONTAINERS: Def = path("Containers", ".spec.template.spec.containers[*].name")
    .all()
    .wide();
const IMAGES: Def = path("Images", ".spec.template.spec.containers[*].image")
    .all()
    .wide();
const SELECTOR: Def = path("Selector", ".spec.selector").wide();

#[rustfmt::skip]
const BUILTIN: &[(&str, &str, &str, &[Def])] = &[
    // -- core ---------------------------------------------------------------
    ("", "v1", "pods", &[
        cell("Ready").ty(Ty::FractionNum),
        cell("Ready").ty(Ty::FractionDen).named("total"),
        cell("Status"),
        // kubectl counts init containers' restarts while the pod initializes,
        // and restartable (sidecar) ones' after: the cell's leading count.
        // The last restart is the latest of every container's, init and
        // ephemeral ones included.
        cell("Restarts").ty(Ty::LeadingInt),
        path("lastRestart", ".status..lastState.terminated.finishedAt").max().ty(Ty::Date),
        path("IP", ".status.podIP").wide(),
        path("Node", ".spec.nodeName").wide(),
        path("Nominated Node", ".status.nominatedNodeName").wide(),
        cell("Readiness Gates").wide(),
    ]),
    ("", "v1", "nodes", &[
        path("ready", ".status.conditions[?(@.type==\"Ready\")].status").ty(Ty::String),
        path("unschedulable", ".spec.unschedulable").falsy(),
        cell("Roles").ty(Ty::List),
        path("Version", ".status.nodeInfo.kubeletVersion"),
        path("Internal-IP", ".status.addresses[?(@.type==\"InternalIP\")].address").all().wide(),
        path("External-IP", ".status.addresses[?(@.type==\"ExternalIP\")].address").all().wide(),
        path("OS-Image", ".status.nodeInfo.osImage").wide(),
        path("Kernel-Version", ".status.nodeInfo.kernelVersion").wide(),
        path("architecture", ".status.nodeInfo.architecture").wide(),
        path("Container-Runtime", ".status.nodeInfo.containerRuntimeVersion").wide(),
    ]),
    ("", "v1", "namespaces", &[
        path("Status", ".status.phase"),
    ]),
    ("", "v1", "services", &[
        path("Type", ".spec.type"),
        path("Cluster-IP", ".spec.clusterIP"),
        cell("External-IP").ty(Ty::List),
        path("port", ".spec.ports[*].port").all(),
        path("nodePort", ".spec.ports[*].nodePort").all(),
        path("protocol", ".spec.ports[*].protocol").all(),
        path("Ports", ".spec.ports").all().wide(),
        path("Selector", ".spec.selector").wide(),
    ]),
    ("", "v1", "endpoints", &[
        path("addresses", ".subsets[*].addresses[*].ip").all(),
        path("ports", ".subsets[*].ports[*].port").all(),
        path("subsets", ".subsets").all().wide(),
    ]),
    ("", "v1", "configmaps", &[
        path("Data", ".data").count(),
        path("binaryData", ".binaryData").count(),
    ]),
    ("", "v1", "secrets", &[
        path("Type", ".type"),
        path("Data", ".data").count(),
    ]),
    ("", "v1", "serviceaccounts", &[]),
    ("", "v1", "persistentvolumeclaims", &[
        cell("Status"),
        path("Volume", ".spec.volumeName"),
        path("Capacity", ".status.capacity.storage").ty(Ty::Memory),
        path("Access Modes", ".status.accessModes").all(),
        path("StorageClass", ".spec.storageClassName"),
        path("VolumeAttributesClass", ".spec.volumeAttributesClassName"),
        path("VolumeMode", ".spec.volumeMode").wide(),
    ]),
    ("", "v1", "persistentvolumes", &[
        path("Capacity", ".spec.capacity.storage").ty(Ty::Memory),
        path("Access Modes", ".spec.accessModes").all(),
        path("Reclaim Policy", ".spec.persistentVolumeReclaimPolicy"),
        // The phase, or Terminating once deleted, as for a claim.
        cell("Status"),
        cell("Claim"),
        path("StorageClass", ".spec.storageClassName"),
        path("VolumeAttributesClass", ".spec.volumeAttributesClassName"),
        path("Reason", ".status.reason"),
        path("VolumeMode", ".spec.volumeMode").wide(),
        path("claimRef", ".spec.claimRef").wide(),
    ]),
    // kubectl's printEvent: a series' last observation, else the last
    // timestamp, else the first, else the new API's eventTime; the count of
    // the series, else the count, else 1 (a new-API singleton). SOURCE is
    // the component and host, else the reporting controller and instance.
    ("", "v1", "events", &[
        path("Last Seen", ".series.lastObservedTime").or(&[".lastTimestamp", ".firstTimestamp", ".eventTime"]).ty(Ty::Date),
        path("Type", ".type"),
        path("Reason", ".reason"),
        cell("Object"),
        path("Subobject", ".involvedObject.fieldPath").wide(),
        cell("Source").wide(),
        path("Message", ".message"),
        path("First Seen", ".firstTimestamp").or(&[".eventTime"]).ty(Ty::Date).wide(),
        path("Count", ".series.count").or(&[".count"]).one().wide(),
        path("involvedObject", ".involvedObject").wide(),
    ]),
    ("", "v1", "limitranges", &[]),
    ("", "v1", "resourcequotas", &[
        cell("Request"),
        cell("Limit"),
        path("hard", ".status.hard").wide(),
        path("used", ".status.used").wide(),
    ]),
    ("", "v1", "replicationcontrollers", &[
        path("Desired", ".spec.replicas"),
        path("Current", ".status.replicas").zero(),
        path("Ready", ".status.readyReplicas").zero(),
        CONTAINERS,
        IMAGES,
        SELECTOR,
    ]),
    ("", "v1", "podtemplates", &[
        path("Containers", ".template.spec.containers[*].name").all(),
        path("Images", ".template.spec.containers[*].image").all(),
        cell("Pod Labels"),
    ]),
    // -- apps ---------------------------------------------------------------
    ("apps", "v1", "deployments", &[
        path("Ready", ".status.readyReplicas").zero(),
        path("desired", ".spec.replicas"),
        path("Up-to-date", ".status.updatedReplicas").zero(),
        path("Available", ".status.availableReplicas").zero(),
        CONTAINERS,
        IMAGES,
        SELECTOR,
    ]),
    ("apps", "v1", "daemonsets", &[
        path("Desired", ".status.desiredNumberScheduled").zero(),
        path("Current", ".status.currentNumberScheduled").zero(),
        path("Ready", ".status.numberReady").zero(),
        path("Up-to-date", ".status.updatedNumberScheduled").zero(),
        path("Available", ".status.numberAvailable").zero(),
        cell("Node Selector"),
        CONTAINERS,
        IMAGES,
        SELECTOR,
    ]),
    ("apps", "v1", "statefulsets", &[
        path("Ready", ".status.readyReplicas").zero(),
        path("desired", ".spec.replicas"),
        CONTAINERS,
        IMAGES,
    ]),
    ("apps", "v1", "replicasets", &[
        path("Desired", ".spec.replicas"),
        path("Current", ".status.replicas").zero(),
        path("Ready", ".status.readyReplicas").zero(),
        CONTAINERS,
        IMAGES,
        SELECTOR,
    ]),
    ("apps", "v1", "controllerrevisions", &[
        cell("Controller"),
        path("Revision", ".revision"),
    ]),
    // -- batch --------------------------------------------------------------
    ("batch", "v1", "jobs", &[
        cell("Status"),
        // A work-queue Job sets no completions: kubectl counts it as 1
        // (`0/1`, `0/1 of 3` with parallelism).
        path("Completions", ".spec.completions").one(),
        path("succeeded", ".status.succeeded").zero(),
        cell("Duration").ty(Ty::GoDuration),
        CONTAINERS,
        IMAGES,
        SELECTOR,
    ]),
    ("batch", "v1", "cronjobs", &[
        path("Schedule", ".spec.schedule"),
        path("Timezone", ".spec.timeZone"),
        path("Suspend", ".spec.suspend").falsy(),
        path("Active", ".status.active").count(),
        path("Last Schedule", ".status.lastScheduleTime").ty(Ty::Date),
        path("Containers", ".spec.jobTemplate.spec.template.spec.containers[*].name").all().wide(),
        path("Images", ".spec.jobTemplate.spec.template.spec.containers[*].image").all().wide(),
        path("Selector", ".spec.jobTemplate.spec.selector").wide(),
    ]),
    // -- autoscaling --------------------------------------------------------
    ("autoscaling", "v1", "horizontalpodautoscalers", &[
        cell("Reference"),
        cell("Targets"),
        path("MinPods", ".spec.minReplicas"),
        path("MaxPods", ".spec.maxReplicas"),
        path("Replicas", ".status.currentReplicas").zero(),
        path("scaleTargetRef", ".spec.scaleTargetRef").wide(),
    ]),
    ("autoscaling", "v2", "horizontalpodautoscalers", &[
        cell("Reference"),
        cell("Targets"),
        path("MinPods", ".spec.minReplicas"),
        path("MaxPods", ".spec.maxReplicas"),
        path("Replicas", ".status.currentReplicas").zero(),
        path("scaleTargetRef", ".spec.scaleTargetRef").wide(),
    ]),
    // -- the apiserver's own groups -----------------------------------------
    ("admissionregistration.k8s.io", "v1", "mutatingwebhookconfigurations", &[
        path("Webhooks", ".webhooks").count(),
    ]),
    ("admissionregistration.k8s.io", "v1", "validatingwebhookconfigurations", &[
        path("Webhooks", ".webhooks").count(),
    ]),
    ("admissionregistration.k8s.io", "v1", "validatingadmissionpolicies", &[
        path("Validations", ".spec.validations").count(),
        cell("ParamKind"),
    ]),
    ("admissionregistration.k8s.io", "v1", "validatingadmissionpolicybindings", &[
        path("PolicyName", ".spec.policyName"),
        cell("ParamRef"),
    ]),
    ("apiextensions.k8s.io", "v1", "customresourcedefinitions", &[
        path("Scope", ".spec.scope"),
        path("Versions", ".spec.versions[*].name").all(),
        path("Group", ".spec.group").wide(),
        path("Kind", ".spec.names.kind").wide(),
        path("ShortNames", ".spec.names.shortNames").wide(),
        path("Established", ".status.conditions[?(@.type==\"Established\")].status").ty(Ty::String).wide(),
    ]),
    ("apiregistration.k8s.io", "v1", "apiservices", &[
        cell("Service"),
        path("Available", ".status.conditions[?(@.type==\"Available\")].status").ty(Ty::String),
    ]),
    ("certificates.k8s.io", "v1", "certificatesigningrequests", &[
        path("SignerName", ".spec.signerName"),
        path("Requestor", ".spec.username"),
        path("RequestedDuration", ".spec.expirationSeconds").ty(Ty::Seconds),
        cell("Condition").ty(Ty::List),
    ]),
    ("coordination.k8s.io", "v1", "leases", &[
        path("Holder", ".spec.holderIdentity"),
    ]),
    ("discovery.k8s.io", "v1", "endpointslices", &[
        path("AddressType", ".addressType"),
        path("Ports", ".ports[*].port").all(),
        path("Endpoints", ".endpoints[*].addresses[*]").all(),
    ]),
    ("events.k8s.io", "v1", "events", &[
        path("Last Seen", ".series.lastObservedTime").or(&[".deprecatedLastTimestamp", ".deprecatedFirstTimestamp", ".eventTime"]).ty(Ty::Date),
        path("Type", ".type"),
        path("Reason", ".reason"),
        cell("Object"),
        path("Subobject", ".regarding.fieldPath").wide(),
        cell("Source").wide(),
        path("Message", ".note"),
        path("First Seen", ".deprecatedFirstTimestamp").or(&[".eventTime"]).ty(Ty::Date).wide(),
        path("Count", ".series.count").or(&[".deprecatedCount"]).one().wide(),
        path("regarding", ".regarding").wide(),
    ]),
    ("flowcontrol.apiserver.k8s.io", "v1", "flowschemas", &[
        path("PriorityLevel", ".spec.priorityLevelConfiguration.name"),
        path("MatchingPrecedence", ".spec.matchingPrecedence"),
        path("DistinguisherMethod", ".spec.distinguisherMethod.type"),
        path("MissingPL", ".status.conditions[?(@.type==\"Dangling\")].status").ty(Ty::String),
    ]),
    ("flowcontrol.apiserver.k8s.io", "v1", "prioritylevelconfigurations", &[
        path("Type", ".spec.type"),
        path("NominalConcurrencyShares", ".spec.limited.nominalConcurrencyShares"),
        path("Queues", ".spec.limited.limitResponse.queuing.queues"),
        path("HandSize", ".spec.limited.limitResponse.queuing.handSize"),
        path("QueueLengthLimit", ".spec.limited.limitResponse.queuing.queueLengthLimit"),
    ]),
    ("networking.k8s.io", "v1", "ingressclasses", &[
        path("Controller", ".spec.controller"),
        cell("Parameters"),
    ]),
    ("networking.k8s.io", "v1", "ingresses", &[
        path("Class", ".spec.ingressClassName"),
        path("Hosts", ".spec.rules[*].host").all(),
        cell("Address").ty(Ty::List),
        cell("Ports").ty(Ty::IntList),
    ]),
    ("networking.k8s.io", "v1", "ipaddresses", &[
        cell("ParentRef"),
    ]),
    ("networking.k8s.io", "v1", "networkpolicies", &[
        cell("Pod-Selector"),
    ]),
    ("networking.k8s.io", "v1", "servicecidrs", &[
        path("CIDRs", ".spec.cidrs").all(),
    ]),
    ("node.k8s.io", "v1", "runtimeclasses", &[
        path("Handler", ".handler"),
    ]),
    ("policy", "v1", "poddisruptionbudgets", &[
        path("Min Available", ".spec.minAvailable"),
        path("Max Unavailable", ".spec.maxUnavailable"),
        path("Allowed Disruptions", ".status.disruptionsAllowed").zero(),
    ]),
    ("rbac.authorization.k8s.io", "v1", "roles", &[]),
    ("rbac.authorization.k8s.io", "v1", "clusterroles", &[]),
    ("rbac.authorization.k8s.io", "v1", "rolebindings", &[
        cell("Role"),
        path("roleRef", ".roleRef").wide(),
        path("Users", ".subjects[?(@.kind==\"User\")].name").all().wide(),
        path("Groups", ".subjects[?(@.kind==\"Group\")].name").all().wide(),
        path("ServiceAccounts", ".subjects[?(@.kind==\"ServiceAccount\")]").all().wide(),
    ]),
    ("rbac.authorization.k8s.io", "v1", "clusterrolebindings", &[
        cell("Role"),
        path("roleRef", ".roleRef").wide(),
        path("Users", ".subjects[?(@.kind==\"User\")].name").all().wide(),
        path("Groups", ".subjects[?(@.kind==\"Group\")].name").all().wide(),
        path("ServiceAccounts", ".subjects[?(@.kind==\"ServiceAccount\")]").all().wide(),
    ]),
    ("resource.k8s.io", "v1", "resourceclaims", &[
        cell("State"),
    ]),
    ("resource.k8s.io", "v1", "resourceclaimtemplates", &[]),
    ("resource.k8s.io", "v1", "deviceclasses", &[]),
    ("resource.k8s.io", "v1", "resourceslices", &[
        path("Node", ".spec.nodeName"),
        path("Driver", ".spec.driver"),
        path("Pool", ".spec.pool.name"),
    ]),
    ("scheduling.k8s.io", "v1", "priorityclasses", &[
        path("Value", ".value"),
        path("Global-Default", ".globalDefault").falsy(),
        path("PreemptionPolicy", ".preemptionPolicy"),
    ]),
    ("storage.k8s.io", "v1", "storageclasses", &[
        path("Provisioner", ".provisioner"),
        path("ReclaimPolicy", ".reclaimPolicy"),
        path("VolumeBindingMode", ".volumeBindingMode"),
        path("AllowVolumeExpansion", ".allowVolumeExpansion").falsy(),
    ]),
    ("storage.k8s.io", "v1", "csidrivers", &[
        path("AttachRequired", ".spec.attachRequired"),
        path("PodInfoOnMount", ".spec.podInfoOnMount"),
        path("StorageCapacity", ".spec.storageCapacity"),
        path("TokenRequests", ".spec.tokenRequests[*].audience").all(),
        path("RequiresRepublish", ".spec.requiresRepublish"),
        path("Modes", ".spec.volumeLifecycleModes").all(),
    ]),
    ("storage.k8s.io", "v1", "csinodes", &[
        path("Drivers", ".spec.drivers").count(),
    ]),
    ("storage.k8s.io", "v1", "csistoragecapacities", &[]),
    ("storage.k8s.io", "v1", "volumeattachments", &[
        path("Attacher", ".spec.attacher"),
        path("PV", ".spec.source.persistentVolumeName"),
        path("Node", ".spec.nodeName"),
        path("Attached", ".status.attached").falsy(),
    ]),
    ("storage.k8s.io", "v1", "volumeattributesclasses", &[
        path("DriverName", ".driverName"),
    ]),
];
