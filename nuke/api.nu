use ./cfg.nu
use ./cache.nu
use ./call.nu

def get-api-resources [conf] {
  let core = call $conf api/v1
  | get resources
  | upsert group api
  | upsert version v1

  let noncore = call $conf 'apis'
  | get groups
  | select name versions
  | reduce --fold [] {|group acc|
    let re = $group.versions | reduce --fold [] {|version acc|
      $acc | append (
        (call $conf $'apis/($group.name)/($version.version)').resources
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

#################
# api resources #
#################

def fmt-api-resources [
  content: any, 
  --output(-o): string
  --verbs(-v): list<string>
] {
  let res = if ($verbs | is-empty) { $content } else {
    $content | where {|resource| $verbs | all {|verb| $verb in ($resource.verbs? | default [])}}
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

def api-resources-output-completer [context: string] { [wide compact] }

def verbs-completer [context: string] {
  resources -o wide | get verbs | flatten | uniq
}

export def resources [
  --output(-o): string@api-resources-output-completer
  --verbs(-v): list<string>@verbs-completer
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
  let conf = cfg show
  let cache_file = $'($conf.current-context).api-resources'

  let cached = cache read $cache_file -c 7day
  if ($cached | is-not-empty) {
    return (fmt-api-resources $cached -o $output -v $verbs)
  }

  let res = get-api-resources $conf

  cache write $cache_file $res
  fmt-api-resources $res -o $output -v $verbs
}

################
# api versions #
################

def fmt-api-versions [content: any] {
  $content 
  | select group version 
  | uniq-by group version
  | each {|v|
    $v | insert name $"($v.group)/($v.version)"
  }
}

export def versions [] {
  let conf = cfg show
  let cache_file = $'($conf.current-context).api-resources'

  let cached = cache read $cache_file -c 7day
  if ($cached | is-not-empty) {
    return (fmt-api-versions $cached)
  }
  let res = get-api-resources $conf
  cache write $cache_file $res
  fmt-api-versions $res
}
