use "../../fmt/helpers.nu"
use ./api.nu
use ./apps.nu
use ./rbac-authorization-k8s-io.nu
use ./networking-k8s-io.nu
use ./storage-k8s-io.nu

def default-formatter [output?: string = compact] { $in | helpers meta base }

export def main [] {
  {
    default: {|output?: string = compact| default-formatter $output }
    api: {
      v1: {
        bindings: {|output?: string = compact| api bindings v1 $output }
        componentstatuses: {|output?: string = compact| api componentstatuses v1 $output }
        configmaps: {|output?: string = compact| api configmaps v1 $output }
        endpoints: {|output?: string = compact| api endpoints v1 $output }
        events: {|output?: string = compact| api events v1 $output }
        limitranges: {|output?: string = compact| api limitranges v1 $output }
        namespaces: {|output?: string = compact| api namespaces v1 $output }
        nodes: {|output?: string = compact| api nodes v1 $output }
        persistentvolumeclaims: {|output?: string = compact| api persistentvolumeclaims v1 $output }
        persistentvolumes: {|output?: string = compact| api persistentvolumes v1 $output }
        pods: {|output?: string = compact| api pods v1 $output }
        podtemplates: {|output?: string = compact| api podtemplates v1 $output }
        replicationcontrollers: {|output?: string = compact| api replicationcontrollers v1 $output }
        resourcequotas: {|output?: string = compact| api resourcequotas v1 $output }
        secrets: {|output?: string = compact| api secrets v1 $output }
        serviceaccounts: {|output?: string = compact| api serviceaccounts v1 $output }
        services: {|output?: string = compact| api services v1 $output }
      }
    }
    apps: {
      v1: {
        controllerrevisions: {| output?: string = compact | apps controllerrevisions v1 $output }
        daemonsets: {|output?: string = compact| apps daemonsets v1 $output }
        deployments: {|output?: string = compact| apps deployments v1 $output }
        replicasets: {|output?: string = compact| apps replicasets v1 $output }
        statefulsets: {|output?: string = compact| apps statefulsets v1 $output }
      }
    }
    rbac.authorization.k8s.io: {
      v1: {
        clusterrolebindings: {| output?: string = compact | rbac-authorization-k8s-io clusterrolebindings v1 $output }
        clusterroles: {| output?: string = compact | rbac-authorization-k8s-io clusterroles v1 $output }
        rolebindings: {| output?: string = compact | rbac-authorization-k8s-io rolebindings v1 $output }
        roles: {| output?: string = compact | rbac-authorization-k8s-io roles v1 $output }
      }
    }
    networking.k8s.io: {
      v1: {
        ingressclasses: {|output?: string = compact| networking-k8s-io ingressclasses v1 $output }
        ingresses: {|output?: string = compact| networking-k8s-io ingresses v1 $output }
        ipaddresses: {|output?: string = compact| networking-k8s-io ipaddresses v1 $output }
        networkpolicies: {|output?: string = compact| networking-k8s-io networkpolicies v1 $output }
        servicecidrs: {|output?: string = compact| networking-k8s-io servicecidrs v1 $output }
      }
    }
    storage.k8s.io: {
      v1: {
        csidrivers: {|output?: string = compact| storage-k8s-io csidrivers v1 $output }
        csinodes: {|output?: string = compact| storage-k8s-io csinodes v1 $output }
        csistoragecapacities: {|output?: string = compact| storage-k8s-io csistoragecapacities v1 $output }
        storageclasses: {|output?: string = compact| storage-k8s-io storageclasses v1 $output }
        volumeattachments: {|output?: string = compact| storage-k8s-io volumeattachments v1 $output }
        volumeattributesclasses: {|output?: string = compact| storage-k8s-io volumeattributesclasses v1 $output }
      }
    }
  }
}
