use "../config"
use "../api"
use "../fmt"
use "../show/get-resource.nu"
use "../show/watch-resource.nu"
use "../show/show-completers.nu"
use "../fmt/fmt-completers.nu"
use ./rollout-formatters

# Shows the status of the rollout.
export def status [
  resource: string@"api get-resource-completer" # The kind of resource to get.
  resourcename?: string@"show-completers resourcename" # The name of the resource you want to get.
  --namespace(-n): string@"show-completers namespace" # The namespace you want to get your resource(s) from.
  --selector(-l): string # Filter resources by label.

  --output(-o): string@"fmt-completers output" # The format of the output (compact, full).

  --kubeconf(-K): record # The configuration to use (defaults to kubeconfig).
  --kubeconfpath(-k): path # The path to the kubeconfig (defaults to $env.KUBECONFIG or ~/.kube/config).
  --context(-c): string@"config-completers context" # The context to use in the configuration (defaults to current).
  --cluster(-C): string@"config-completers cluster" # The cluster to use in the configuration (defaults to current).
] {
  let kubeconf = if ($kubeconf | is-not-empty) { $kubeconf } else {
    config -k $kubeconfpath
  } 

  let resource = api resolve-resource $resource
  mut res = (get-resource $resource $resourcename 
    -n $namespace 
    -l $selector
    -K $kubeconf
    -c $context
    -C $cluster
  )

  let formatters = rollout-formatters
  | merge deep ($env.NUKE_ROLLOUTSTATUS_FORMATTERS? | default {})

  # if $watch {
  #   (
  #     watch-resource $res
  #     -n $namespace 
  #     -g $resource.group 
  #     -v $resource.version 
  #     -c $kubeconf
  #   ) | ignore
  #   return
  # }

  let output = if ($output | is-not-empty) {$output} else {'compact'}
  $res | fmt $resource $formatters -o $output
}

# Views previous rollout revisions and configurations.
export def history [
  resource: string@"api get-resource-completer" # The kind of resource to get.
  resourcename?: string@"show-completers resourcename" # The name of the resource you want to get.
  --namespace(-n): string@"show-completers namespace" # The namespace you want to get your resource(s) from.
  --selector(-l): string # Filter resources by label.

  --output(-o): string@"fmt-completers output-no-wide" # The format of the output (compact, full).

  --kubeconf(-K): record # The configuration to use (defaults to kubeconfig).
  --kubeconfpath(-k): path # The path to the kubeconfig (defaults to $env.KUBECONFIG or ~/.kube/config).
  --context(-c): string@"config-completers context" # The context to use in the configuration (defaults to current).
  --cluster(-C): string@"config-completers cluster" # The cluster to use in the configuration (defaults to current).
] {
  let kubeconf = if ($kubeconf | is-not-empty) { $kubeconf } else {
    config -k $kubeconfpath
  } 

  let resource = api resolve-resource $resource
  let obj = (get-resource $resource $resourcename 
    -n $namespace 
    -l $selector
    -K $kubeconf
    -c $context
    -C $cluster
  )

  if ($output == full) {
    return $output
  }

  mut revisions = []

  if $obj.kind == "Deployment" {
    let rs = (
      get-resource {
        name: "replicasets" 
        group: apps
        version: v1
        namespaced: true
      }
      -n $namespace
      -K $kubeconf
      -c $context
      -C $cluster
    )

    return (
      $rs.items
      | where {|r| 
        $r.metadata.ownerReferences?
        | any {|o| $o.uid == $obj.metadata.uid }
      }
      | each {|r|
        {
          revision: ($r.metadata.annotations."deployment.kubernetes.io/revision"?)
          changeCause: ($r.metadata.annotations."kubernetes.io/change-cause"?)
          created: ( $r.metadata.creationTimestamp | into datetime )
        }
      }
      | sort-by revision
    )
  } 

  let cr = (
    get-resource {
      name: "controllerrevisions"
      group: apps
      version: v1
      namespaced: true
    }
    -n $namespace
    -K $kubeconf
    -c $context
    -C $cluster
  )

  return (
    $cr.items
    | where {|r|
      $r.metadata.ownerReferences?
      | any {|o| $o.uid == $obj.metadata.uid }
    }
    | each {|r|
      {
        revision: $r.revision
        created: ( $r.metadata.creationTimestamp | into datetime )
      }
    }
    | sort-by revision
  )
}
