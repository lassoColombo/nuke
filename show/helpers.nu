use "../config/"
use "../api/"

export def build-path [
  resource: record
  resourcename?: string
  --selector(-l): string # filter resources by label
  --namespace(-n): string
  --kubeconf(-c): any
  --all-namespaces(-A)
  --prefix(-p): string
] {
  let kubeconf = $kubeconf | default (config)
  let namespace = if $all_namespaces {
    '' 
  } else if ($namespace | is-not-empty) {
    $namespace
  } else {
    config get-current-namespace
  }
  let resourcename = if ($resourcename | is-not-empty) {$resourcename} else { '' } 

  let prefix = if ($prefix | is-not-empty) {
    $prefix
  } else if $resource.group == "api" and $resource.version == "v1" {
    "api/v1"
  } else {
    $"apis/($resource.group)/($resource.version)"
  }

  let path = if ($resource.namespaced) {
    if $all_namespaces {
      $"($prefix)/($resource.name)"
    } else if ($resourcename | is-empty) {
      $"($prefix)/namespaces/($namespace)/($resource.name)"
    } else {
      $"($prefix)/namespaces/($namespace)/($resource.name)/($resourcename)"
    }
  } else {
    if ($resourcename | is-empty) {
      $"($prefix)/($resource.name)"
    } else {
      $"($prefix)/($resource.name)/($resourcename)"
    }
  }

  mut spec = {path: $path}
  if ($selector | is-not-empty) {
    $spec.params = [
      {key: labelSelector, value: $selector}
    ]
  }

  $spec
}
