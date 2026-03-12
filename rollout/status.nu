use "../config"
use "../api"
use "../fmt"
use "../show/get-resource.nu"
use "../show/watch-resource.nu"
use "../show/show-completers.nu"
use "../fmt/fmt-completers.nu"
use ./rollout-formatters

# ----------
#  status   
# ----------

# Shows the status of the rollout.
export def main [
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
