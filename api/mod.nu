use "../config"
use "../cache"
use "../http-get"
use "../fmt"
use "../fmt/fmt-completers.nu"
use "../config/config-completers.nu"

def get-api-resources [conf, --context(-c): string] {
  let c = http-get {path: api} $conf -c $context
  let core = $c
  | update versions {
    $c.versions | each {|version|
      {version: $version, groupVersion: api/v1} 
      | insert resources (http-get {path: $'api/($version)'} $conf -c $context).resources 
    }
  }
  | upsert name api
  | reject serverAddressByClientCIDRs
  | reject kind

  let noncore = (http-get {path: apis} $conf -c $context).groups
  | each {|group|
    $group | update versions (
      $group.versions | each {|version|
        $version | insert resources (http-get {path: $'apis/($version.groupVersion)'} $conf -c $context).resources 
      }
    )
  }

  $noncore 
  | append $core 
  | each {|group|
    $group | update versions (
      $group.versions | each {|version|
        $version | update resources (
          $version.resources | where {not ($in.name | str contains /)}
          | reject -o storageVersionHash
          | upsert names {|res|
            $res.shortNames?
            | default []
            | append $res.name
            | append $res.singularName?
            | append $"($version.groupVersion)/($res.name)"
            | where {$in | is-not-empty}
          }
        )
      }
    )
  }
}

# -----------------
#  api resources   
# -----------------

def fmt-api-resources [
  content: any, 
  --output(-o): string
  --verbs(-v): list<string>
  --group(-g): string
  --namespaced(-n)
] {
  mut res = $content | each {|group|
    $group.versions 
    | each {|version|
      $version.resources | each {|resource|
        $resource 
        | insert group $group.name
        | insert version $version.version
      }
    }
    | flatten
  }
  | flatten

  if ($group | is-not-empty) {
    $res = $res | where group == $group
  }
  if $namespaced {
    $res = $res | where namespaced == true
  }
  if ($verbs | is-not-empty) {
    $res | where {|resource|
      $verbs | all {|verb| $verb in ($resource.verbs? | default [])}
    }
  }
  if $output == wide {
    return $res
  }
  # $res
  return ($res | select ...[
    name
    version
    group
    namespaced
    kind
    names
  ])
}

def api-resources-output-completer [context: string] { fmt-completers output | where {$in != wide} }
def verbs-completer [context: string] { resources -o wide | get verbs | flatten | uniq }
def group-completer [] { resources | get group | uniq }

# lists all API resources available in the cluster.
export def resources [
  --output(-o): string@api-resources-output-completer
  --verbs(-v): list<string>@verbs-completer
  --group(-g): string@group-completer
  --context(-c): string@"config-completers context"
  --namespaced(-n)
] {
  if ($output | is-not-empty) and not ($output in (fmt supported-outputs)) {
    error make {
      msg: $'Supported outputs are (fmt supported-outputs)'
      label: {
        text: $'($output) is not a supported output'
        span: (metadata $output).span
      }
    }
  }
  let conf = config
  let cache_file = $'($context | default $conf.current-context).api-resources'

  let cached = cache read $cache_file -c 7day
  if ($cached | is-not-empty) {
    return (fmt-api-resources $cached -o $output -v $verbs -g $group --namespaced=$namespaced)
  }

  let res = get-api-resources $conf -c $context

  cache write $cache_file $res
  fmt-api-resources $res -o $output -v $verbs -g $group --namespaced=$namespaced
}

# ----------------
#  api versions   
# ----------------

def fmt-api-versions [content: any] {
  ($content.versions | flatten).groupVersion
}

# lists all API versions available in the cluster.
export def versions [
  --context(-c): string@"config-completers context"
] {
  let conf = config
  let cache_file = $'($context | default $conf.current-context).api-resources'

  let cached = cache read $cache_file -c 7day
  if ($cached | is-not-empty) {
    return (fmt-api-versions $cached)
  }
  let res = get-api-resources $conf -c $context
  cache write $cache_file $res
  fmt-api-versions $res
}
