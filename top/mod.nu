use "../config"
use "../api/resolve-resource.nu"
use "../http-get"
use "../fmt"
use "../api"
use "../show"
use "../show/helpers.nu"
use "../show/get-resource.nu"
use "../show/show-completers.nu"
use "../config/config-completers.nu"
use "../fmt/fmt-completers.nu"

def resource-completer [] { [pods nodes] }

# shows pods and nodes resource usage
export def main [
  resource: string@resource-completer
  resourcename?: string@"show-completers resourcename"
  --output(-o): string@"fmt-completers output"
  --namespace(-n): string@"show-completers namespace"
  --selector(-l): string # filter resources by label
  --context(-C):string@"config-completers context"
  --all-namespaces(-A)
  --show-labels(-l)
] {
  let conf = config

  let namespace = if ($namespace | is-not-empty) {
    $namespace
  } else {
    config get-current-namespace $conf
  }

  let spec = (
    helpers build-path $resource $resourcename
    -n $namespace 
    -p "apis/metrics.k8s.io/v1beta1"
    -c $conf
    -l $selector
    --all-namespaces=$all_namespaces
  )

  let decorators = [
    ...(if ($namespace | is-empty) {['namespace']} else {[]})
    ...(if $show_labels {['labels']} else {[]})
  ]

  let r = (
    resolve-resource $resource 
    | select group version name 
    | first
  )

  http-get $spec $conf -c $context | fmt $r -o $output -d $decorators
}
