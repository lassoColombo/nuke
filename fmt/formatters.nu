use ./resources/apiservice.nu
use ./resources/clusterrole.nu
use ./resources/clusterrolebinding.nu
use ./resources/configmap.nu
use ./resources/controllerrevision.nu
use ./resources/cronjob.nu
use ./resources/csidriver.nu
use ./resources/customresourcedefinition.nu
use ./resources/daemonset.nu
use ./resources/deployment.nu
use ./resources/deviceclass.nu
use ./resources/endpointslice.nu
use ./resources/event.nu
use ./resources/flowschema.nu
use ./resources/horizontalpodautoscaler.nu
use ./resources/ingress.nu
use ./resources/ingressclass.nu
use ./resources/ipaddress.nu
use ./resources/job.nu
use ./resources/limitrange.nu
use ./resources/namespace.nu
use ./resources/networkpolicy.nu
use ./resources/node.nu
use ./resources/persistentvolume.nu
use ./resources/persistentvolumeclaim.nu
use ./resources/pod.nu
use ./resources/poddisruptionbudget.nu
use ./resources/podtemplate.nu
use ./resources/priorityclass.nu
use ./resources/replicaset.nu
use ./resources/resourceclaimtemplate.nu
use ./resources/resourcequota.nu
use ./resources/role.nu
use ./resources/rolebinding.nu
use ./resources/runtimeclass.nu
use ./resources/secret.nu
use ./resources/service.nu
use ./resources/serviceaccount.nu
use ./resources/servicecidr.nu
use ./resources/statefulset.nu
use ./resources/storageclass.nu

export def main [] {
  {
    apiservice: {| output?: string = compact | apiservice $output }
    clusterrole: {| output?: string = compact | clusterrole $output }
    clusterrolebinding: {| output?: string = compact | clusterrolebinding $output }
    configmap: {| output?: string = compact| configmap $output }
    controllerrevision: {| output?: string = compact | controllerrevision $output }
    cronjob: {|output?: string = compact| cronjob $output }
    csidriver: {|output?: string = compact| csidriver $output }
    customresourcedefinition: {|output?: string = compact| customresourcedefinition $output }
    daemonset: {|output?: string = compact| daemonset $output }
    deployment: {|output?: string = compact| deployment $output }
    deviceclass: {|output?: string = compact| deviceclass $output }
    endpointslice: {|output?: string = compact| endpointslice $output }
    event: {|output?: string = compact| event $output }
    flowschema: {|output?: string = compact| flowschema $output }
    horizontalpodautoscaler: {|output?: string = compact| horizontalpodautoscaler $output }
    ingress: {|output?: string = compact| ingress $output }
    ingressclass: {|output?: string = compact| ingressclass $output }
    ipaddress: {|output?: string = compact| ipaddress $output }
    job: {|output?: string = compact| job $output }
    limitrange: {|output?: string = compact| limitrange $output }
    namespace: {|output?: string = compact| namespace $output }
    networkpolicy: {|output?: string = compact| networkpolicy $output }
    node: {|output?: string = compact| node $output }
    persistentvolume: {|output?: string = compact| persistentvolume $output }
    persistentvolumeclaim: {|output?: string = compact| persistentvolumeclaim $output }
    pod: {|output?: string = compact| pod $output }
    poddisruptionbudget: {|output?: string = compact| poddisruptionbudget $output }
    podtemplate: {|output?: string = compact| podtemplate $output }
    priorityclass: {|output?: string = compact| priorityclass $output }
    replicaset: {|output?: string = compact| replicaset $output }
    resourceclaimtemplate: {|output?: string = compact| resourceclaimtemplate $output }
    resourcequota: {|output?: string = compact| resourcequota $output }
    role: {|output?: string = compact| role $output }
    rolebinding: {|output?: string = compact| rolebinding $output }
    runtimeclass: {|output?: string = compact| runtimeclass $output }
    secret: {|output?: string = compact| secret $output }
    service: {|output?: string = compact| service $output }
    serviceaccount: {|output?: string = compact| serviceaccount $output }
    servicecidr: {|output?: string = compact| servicecidr $output }
    statefulset: {|output?: string = compact| statefulset $output }
    storageclass: {|output?: string = compact| storageclass $output }
  }
}
