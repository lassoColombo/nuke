use "../api"
use "../http-get"
use "../config"
use "../config/config-completers.nu"
use "../fmt"
use "../fmt/decorators.nu"
use "../fmt/fmt-completers.nu"
use ./get-resource.nu
use ./resource-formatters
use ./show-completers.nu
# use ./watch-resource.nu

# Display one or many resources
export def main [
  resource: string@"api get-resource-completer" # The kind of resource to get.
  resourcename?: string@"show-completers resourcename" # The name of the resource you want to get.
  --namespace(-n): string@"show-completers namespace" # The namespace you want to get your resource(s) from.
  --all-namespaces(-A) # Get resources from all namespaces.
  --selector(-l): string # Filter resources by label.

  --output(-o): string@"fmt-completers output" # The format of the output (compact, wide, full).
  --show-annotations # Decorate the output with annotations.
  --show-labels # Decorate the output with labels.
  --show-conditions # Decorate the output with conditions.

  --kubeconf(-K): record # The configuration to use (defaults to kubeconfig).
  --kubeconfpath(-k): path # The path to the kubeconfig (defaults to $env.KUBECONFIG or ~/.kube/config).
  --context(-c): string@"config-completers context" # The context to use in the configuration (defaults to current).
  --cluster(-C): string@"config-completers cluster" # The cluster to use in the configuration (defaults to current).
] {
  if ($output | is-not-empty) and not ($output in (fmt-completers output)) {
    error make --unspanned { msg: $'Supported outputs are (fmt-completers output)' }
  }
  let kubeconf = if ($kubeconf | is-not-empty) { $kubeconf } else {
    config --kubeconfpath $kubeconfpath
  } 
  let resources = if $resource != all {
    [(api resolve-resource $resource)]
  } else {
    [
      {group: api version: v1 name: pods namespaced: true}
      {group: api version: v1 name: services namespaced: true}
      {group: api version: v1 name: replicationcontrollers namespaced: true}
      {group: apps version: v1 name: deployments namespaced: true}
      {group: apps version: v1 name: replicasets namespaced: true}
      {group: apps version: v1 name: statefulsets namespaced: true}
      {group: apps version: v1 name: daemonsets namespaced: true}
    ]
  }

  let decs = (decorators 
    --namespace=$all_namespaces
    --labels=$show_labels
    --annotations=$show_annotations
    --conditions=$show_conditions
  )


  # if $watch {
  #   let res = $resources | first
  #   return (watch-resource $res $resourcename
  #     -n $namespace 
  #     -g $res.group 
  #     -v $res.version 
  #     -l $labels
  #     -K $kubeconf
  #     -c $context
  #     -C $cluster
  #     -o $output
  #     --all=$all
  #   ) 
  # }

  mut res = $resources | reduce --fold {} {|resource, acc|
    let r = (get-resource $resource $resourcename 
      -n $namespace 
      -l $selector
      -K $kubeconf
      -c $context
      -C $cluster
      --all-namespaces=$all_namespaces
    )

    let formatters = resource-formatters
    | merge deep ($env.NUKE_RESOURCE_FORMATTERS? | default {})

    if ($r | is-not-empty) {
      $acc | merge { $resource.name: ($r | fmt $resource $formatters -d $decs -o $output) }
    } else {
      $acc
    }
  }

  if ($resources | length) == 1 {
    $res = $res 
    | transpose resource items 
    | get items 
    | first
  }

  $res
}
