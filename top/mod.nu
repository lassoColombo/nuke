use "../api"
use "../api/resolve-resource.nu"
use "../config"
use "../config/config-completers.nu"
use "../fmt"
use "../fmt/decorators.nu"
use "../fmt/fmt-completers.nu"
use "../http-get"
use "../show"
use "../show/get-resource.nu"
use "../show/helpers.nu"
use "../show/show-completers.nu"
use ./metric-formatters

def resource-completer [] { [pods nodes] }

# Shows pods and nodes resource usage.
export def main [
  resource: string@resource-completer # The kind of resource to get.
  resourcename?: string@"show-completers resourcename" # The name of the resource you want to get.
  --namespace(-n): string@"show-completers namespace" # The namespace you want to get your resource(s) from.
  --all-namespaces(-A) # Get resources from all namespaces.
  --selector(-l): string # Filter resources by label.

  --output(-o): string@"fmt-completers output" # The format of the output (compact, wide, full).
  --show-labels # Decorate the output with labels.

  --kubeconf(-K): record # The configuration to use (defaults to kubeconfig).
  --kubeconfpath(-k): path # The path to the kubeconfig (defaults to $env.KUBECONFIG or ~/.kube/config).
  --context(-c): string@"config-completers context" # The context to use in the configuration (defaults to current).
  --cluster(-C): string@"config-completers cluster" # The cluster to use in the configuration (defaults to current).

] {
  let kubeconf = if ($kubeconf | is-not-empty) {
    $kubeconf
  } else {
    config --kubeconfpath $kubeconfpath
  } 

  let spec = (
    helpers build-path $resource $resourcename
    -n $namespace 
    -p "apis/metrics.k8s.io/v1beta1"
    -c $kubeconf
    -l $selector
    --all-namespaces=$all_namespaces
  )

  let res = (
    resolve-resource $resource 
    | select group version name 
    | first
  )

  let formatters = metric-formatters
  | merge deep ($env.NUKE_METRIC_FORMATTERS? | default {})

  let decs = (decorators --labels=$show_labels)

  http-get $spec -K $kubeconf -c $context -c $cluster 
  | fmt $res $formatters -d $decs -o $output
}
