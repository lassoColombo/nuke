use "../config"
use "../http-get"
use "../show/fmt"
use "../api"
use "../show"
use "../show/helpers.nu"
use "../show/get-resource.nu"

def resource-completer [] {
  [pods nodes]
}

def output-completer [] {
  fmt supported-outputs
}

def namespace-completer [] {
  show ns | get name
}


def resourcename-completer [context: string] {
  if ($context | is-empty) {
    return []
  } 
  mut prev = $context | parse --regex '(?P<word>\S+)' | get word

  let idx = $prev | enumerate | where {$in.item == '-n'} | get index | if ($in | length) != 1 {null} else {$in | first}
  let namespace = if ($idx | is-not-empty) {
    $prev | get ($idx + 1)
  } else {
    ''
  }
  if ($idx | is-not-empty) {
    $prev = $prev | reject $idx (idx + 1)
  }

  let resources = api resources -o wide | get -o names | flatten
  let resource = $prev | get (($prev | enumerate | where {|arg| $arg.item in $resources } | first | get index))

  get-resource $resource -n $namespace 
  | get items 
  | get metadata.name
}


# shows pods and nodes resource usage
export def main [
  resource: string@resource-completer
  resourcename?: string@resourcename-completer
  --output(-o): string@output-completer
  --namespace(-n): string@namespace-completer
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

  let decorators = [
    ...(if ($namespace | is-empty) {['namespace']} else {[]})
    ...(if $show_labels {['labels']} else {[]})
  ]

  http-get $path $conf | fmt resource -o $output -d $decorators
}
