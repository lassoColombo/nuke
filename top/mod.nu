use "../config"
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
  --context(-C):string@"config-completers context"
  --show-labels(-l)
] {
  let conf = config

  let namespace = if ($namespace | is-not-empty) {
    $namespace
  } else {
    config get-current-namespace $conf
  }

  let base = "apis/metrics.k8s.io/v1beta1"

  let path = if $resource in [nodes node no] {
    $"($base)/nodes"
  } else if $resource in [pods pod po] {
    if ($namespace | is-empty) {
      $"($base)/pods"
    } else {
      if ($resourcename | is-empty) {
        $"($base)/namespaces/($namespace)/pods"
      } else {
        $"($base)/namespaces/($namespace)/pods/($resourcename)"
      }
    }
  } else {
    error make {msg: $"($resource) is not a supported resource. Supported resources for the top commands are (resource-completer)"}
  }
  let spec = {path: $path}

  let decorators = [
    ...(if ($namespace | is-empty) {['namespace']} else {[]})
    ...(if $show_labels {['labels']} else {[]})
  ]

  http-get $spec $conf -c $context | fmt {} -o $output -d $decorators
}
