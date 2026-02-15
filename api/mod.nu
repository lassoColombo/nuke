use "../config"
use "../cache"
use "../http-get"
use "../fmt"
use "../fmt/fmt-completers.nu"
use "../config/config-completers.nu"

def get-api-resources [conf] {
  let core = http-get {path: api/v1} $conf 
  | get resources
  | upsert group api
  | upsert version v1

  let noncore = http-get {path: apis} $conf 
  | get groups
  | select name versions
  | reduce --fold [] {|group acc|
    let re = $group.versions | reduce --fold [] {|version acc|
      $acc | append (
        (http-get {path: $'apis/($group.name)/($version.version)'} $conf).resources
        | upsert group $group.name 
        | upsert version $version.version
      )
    }
    $acc | append $re
  }

  $core | append $noncore 
  | where {not ($in.name | str contains /)}
  | each {
    $in | upsert names {|res|
      $res.shortNames?
      | default []
      | append $res.name
      | append $res.singularName?
      | where {$in | is-not-empty}
    }
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
  mut res = $content
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
  return ($res | select ...[
    name
    version
    namespaced
    kind
    group
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
  mut conf = config
  if ($context | is-not-empty) { $conf.current-context = $context }

  let cache_file = $'($conf.current-context).api-resources'

  let cached = cache read $cache_file -c 7day
  if ($cached | is-not-empty) {
    return (fmt-api-resources $cached -o $output -v $verbs -g $group --namespaced=$namespaced)
  }

  let res = get-api-resources $conf

  cache write $cache_file $res
  fmt-api-resources $res -o $output -v $verbs -g $group --namespaced=$namespaced
}

# ----------------
#  api versions   
# ----------------

def fmt-api-versions [content: any] {
  $content 
  | select group version 
  | uniq-by group version
  | each {|v|
    $v | insert name $"($v.group)/($v.version)"
  }
}

# lists all API versions available in the cluster.
export def versions [
  --context(-c): string@"config-completers context"
] {
  mut conf = config
  if ($context | is-not-empty) { $conf.current-context = $context }
  let cache_file = $'($conf.current-context).api-resources'

  let cached = cache read $cache_file -c 7day
  if ($cached | is-not-empty) {
    return (fmt-api-versions $cached)
  }
  let res = get-api-resources $conf
  cache write $cache_file $res
  fmt-api-versions $res
}
