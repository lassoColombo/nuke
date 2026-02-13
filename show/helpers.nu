use "../config/"
use "../api/"

export def build-path [
  resource: string
  resourcename?: string
  --group(-g): string
  --version(-v): string
  --namespace(-n): string
  --conf(-c): any
  --all(-A)
  --prefix(-p): string
] {
  let conf = $conf | default (config)
  let namespace = if $all {
    '' 
  } else if ($namespace | is-not-empty) {
    $namespace
  } else {
    config get-current-namespace $conf 
  }
  let resourcename = if ($resourcename | is-not-empty) {$resourcename} else { '' } 

  let resource = if ($group | is-not-empty) and ($version | is-not-empty) {
    api resources -o wide
    | where {$resource in $in.names} 
    | first
    | upsert group $group
    | upsert version $version
    | select group version name namespaced
  } else if ($group | is-empty) and ($version | is-empty) {
    api resources -o wide
    | where {$resource in $in.names?} 
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
    if $all {
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

  $path
}
