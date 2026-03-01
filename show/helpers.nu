use "../config/"
use "../api/"

export def build-path [
  resource: string
  resourcename?: string
  --selector(-l): string # filter resources by label
  --group(-g): string
  --version(-v): string
  --namespace(-n): string
  --kubeconf(-c): any
  --all-namespaces(-A)
  --prefix(-p): string
] {
  let resource = $resource
  | str downcase 
  | str replace --regex 'metrics$' ''
  | str replace --regex 'list$' ''

  let kubeconf = $kubeconf | default (config)
  let namespace = if $all_namespaces {
    '' 
  } else if ($namespace | is-not-empty) {
    $namespace
  } else {
    config get-current-namespace
  }
  let resourcename = if ($resourcename | is-not-empty) {$resourcename} else { '' } 

  let resource = if ($group | is-not-empty) and ($version | is-not-empty) {
    api resources -o wide $resource
    | first
    | upsert group $group
    | upsert version $version
    | select group version name namespaced
  } else if ($group | is-empty) and ($version | is-empty) {
    api resources -o wide $resource
    | first
    | select group version name namespaced
  }
  let prefix = if ($prefix | is-not-empty) {
    $prefix
  } else if $resource.group == "api" and $resource.version == "v1" {
    "api/v1"
  } else {
    $"apis/($resource.group)/($resource.version)"
  }

  let path = if $resource.namespaced {
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
