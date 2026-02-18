# resources
use ./api.nu
use ./apiregistration_k8s_io.nu
use ./apps.nu
use ./autoscaling.nu
use ./batch.nu
use ./events_k8s_io.nu
use ./networking_k8s_io.nu
use ./policy.nu
use ./rbac_authorization_k8s_io.nu
use ./storage_k8s_io.nu
use ./apiextensions_k8s_io.nu
use ./scheduling_k8s_io.nu
use ./node_k8s_io.nu
use ./discovery_k8s_io.nu
use ./flowcontrol_k8s_io.nu
use ./resource_k8s_io.nu

export def main [] {
  {
    api: {
      v1: {
        configmaps: {| output?: string = compact| api configmaps v1 $output }
        events: {|output?: string = compact| api events v1 $output }
        limitranges: {|output?: string = compact| api limitranges v1 $output }
        namespaces: {|output?: string = compact| api namespaces v1 $output }
        nodes: {|output?: string = compact| api nodes v1 $output }
        pods: {|output?: string = compact| api pods v1 $output }
        podtemplates: {|output?: string = compact| api podtemplates v1 $output }
        resourcequotas: {|output?: string = compact| api resourcequotas v1 $output }
        secrets: {|output?: string = compact| api secrets v1 $output }
        serviceaccounts: {|output?: string = compact| api serviceaccounts v1 $output }
        services: {|output?: string = compact| api services v1 $output }
      }
    }
    apiregistration.k8s.io: {
      v1: {
        apiservices: {| output?: string = compact | apiregistration_k8s_io apiservices v1 $output }
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
    autoscaling: {
      v1: {
        horizontalpodautoscalers: {|output?: string = compact| autoscaling horizontalpodautoscalers v1 $output }
      }
    }
    batch: {
      v1: {
        cronjobs: {|output?: string = compact| batch cronjobs v1 $output }
        jobs: {|output?: string = compact| batch jobs v1 $output }
      }
    }
    certificates.k8s.io: {
      v1: { }
    }
    events.k8s.io: {
      v1: {
        events: {|output?: string = compact| events_k8s_io events v1 $output }
      }
    }
    networking.k8s.io: {
      v1: {
        ingressclasses: {|output?: string = compact| networking_k8s_io ingressclasses v1 $output }
        ingresses: {|output?: string = compact| networking_k8s_io ingresses v1 $output }
        ipaddresses: {|output?: string = compact| networking_k8s_io ipaddresses v1 $output }
        networkpolicies: {|output?: string = compact| nnetworking_k8s_io etworkpolicies v1 $output }
        servicecidr: {|output?: string = compact| snetworking_k8s_io ervicecidr v1 $output }
      }
    }
    policy: {
      v1: {
        poddisruptionbudgets: {|output?: string = compact| policy poddisruptionbudgets v1 $output }
      }
    }
    rbac.authorization.k8s.io: {
      v1: {
        clusterrolebindings: {| output?: string = compact | rbac_authorization_k8s_io clusterrolebindings v1 $output }
        clusterroles: {| output?: string = compact | rbac_authorization_k8s_io clusterroles v1 $output }
        rolebindings: {|output?: string = compact| rbac_authorization_k8s_io rolebindings v1 $output }
        roles: {|output?: string = compact| rbac_authorization_k8s_io roles v1 $output }
      }
    }
    storage.k8s.io: {
      v1: {
        csidrivers: {|output?: string = compact| storage_k8s_io csidriver v1 $output }
        storageclasses: {|output?: string = compact| storage_k8s_io storageclasses v1 $output }
        volumeattachments: {|output?: string = compact| storage_k8s_io volumeattachment v1 $output }
        volumeattributesclasses: {|output?: string = compact| storage_k8s_io volumeattributesclass v1 $output }
        persistentvolumes: {|output?: string = compact| storage_k8s_io api_v1_persistentvolume v1 $output }
        persistentvolumeclaims: {|output?: string = compact| storage_k8s_io api_v1_persistentvolumeclaim v1 $output }
      }
    }
    admissionregistration.k8s.io: {
      v1: { }
    }
    apiextensions.k8s.io: {
      v1: {
        customresourcedefinitions: {|output?: string = compact| apiextensions_k8s_io customresourcedefinitions v1 $output }
      }
    }
    scheduling.k8s.io: {
      v1: {
        priorityclasses: {|output?: string = compact| scheduling_k8s_io priorityclasses v1 $output }
      }
    }
    coordination.k8s.io: {
      v1: { }
    }
    node.k8s.io: {
      v1: {
        runtimeclasses: {|output?: string = compact| node_k8s_io runtimeclasses v1 $output }
      }
    }
    discovery.k8s.io: {
      v1: {
        endpointslices: {|output?: string = compact| discovery_k8s_io endpointslices v1 $output }
      }
    }
    resource.k8s.io: {
      v1: {
        deviceclasses: {|output?: string = compact| resource_k8s_io deviceclasses v1 $output }
        resourceclaimtemplates: {|output?: string = compact| resource_k8s_io resourceclaimtemplates v1 $output }
      }
    }
    flowcontrol.apiserver.k8s.io: {
      v1: {
        flowschemas: {|output?: string = compact| flowcontrol_k8s_io flowschemas v1 $output }
        prioritylevelconfigurations: {|output?: string = compact| flowcontrol_k8s_io prioritylevelconfigurations v1 $output }
      }
    }
  }
}
