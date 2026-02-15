# Coverage

### api

```mermaid
pie title Resource Coverage
    "supported" : 13
    "unsupported" : 3
    "unwilling to support" : 1

```

| Resource       | Status |
|-----------------|--------|
| componentstatuses     | ⚪     |
| configmaps     | 🟢     |
| endpoints     | 🔴    |
| events     | 🟢     |
| limitranges     | 🟢     |
| namespaces     | 🟢     |
| nodes     | 🟢     |
| persistentvolumeclaims     | 🟢     |
| persistentvolumes     | 🟢     |
| pods     | 🟢     |
| podtemplates     | 🟢     |
| replicationcontrollers     | ⚪     |
| resourcequotas     | 🟢     |
| secrets     | 🟢     |
| serviceaccounts     | 🟢     |
| services     | 🟢     |
### apiregistration.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 1
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| apiservices     | 🟢     |
### apps

```mermaid
pie title Resource Coverage
    "supported" : 5
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| controllerrevisions     | 🟢     |
| daemonsets     | 🟢     |
| deployments     | 🟢     |
| replicasets     | 🟢     |
| statefulsets     | 🟢     |
### autoscaling

```mermaid
pie title Resource Coverage
    "supported" : 1
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| horizontalpodautoscalers     | 🟢     |
### batch

```mermaid
pie title Resource Coverage
    "supported" : 2
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| cronjobs     | 🟢     |
| jobs     | 🟢     |
### certificates.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 0
    "unsupported" : 1
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| certificatesigningrequests     | ⚪     |
### networking.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 5
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| ingressclasses     | 🟢     |
| ingresses     | 🟢     |
| ipaddresses     | 🟢     |
| networkpolicies     | 🟢     |
| servicecidrs     | 🟢     |
### policy

```mermaid
pie title Resource Coverage
    "supported" : 1
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| poddisruptionbudgets     | 🟢     |
### rbac.authorization.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 4
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| clusterrolebindings     | 🟢     |
| clusterroles     | 🟢     |
| rolebindings     | 🟢     |
| roles     | 🟢     |
### storage.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 4
    "unsupported" : 2
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| csidrivers     | 🟢     |
| csinodes     | ⚪     |
| csistoragecapacities     | ⚪     |
| storageclasses     | 🟢     |
| volumeattachments     | 🟢     |
| volumeattributesclasses     | 🟢     |
### admissionregistration.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 0
    "unsupported" : 4
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| mutatingwebhookconfigurations     | ⚪     |
| validatingadmissionpolicies     | ⚪     |
| validatingadmissionpolicybindings     | ⚪     |
| validatingwebhookconfigurations     | ⚪     |
### apiextensions.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 1
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| customresourcedefinitions     | 🟢     |
### scheduling.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 1
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| priorityclasses     | 🟢     |
### coordination.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 0
    "unsupported" : 1
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| leases     | ⚪     |
### node.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 1
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| runtimeclasses     | 🟢     |
### discovery.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 1
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| endpointslices     | 🟢     |
### resource.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 2
    "unsupported" : 2
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| deviceclasses     | 🟢     |
| resourceclaims     | ⚪     |
| resourceclaimtemplates     | 🟢     |
| resourceslices     | ⚪     |
### flowcontrol.apiserver.k8s.io

```mermaid
pie title Resource Coverage
    "supported" : 2
    "unsupported" : 0
    "unwilling to support" : 0

```

| Resource       | Status |
|-----------------|--------|
| flowschemas     | 🟢     |
| prioritylevelconfigurations     | 🟢     |