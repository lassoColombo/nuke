use "../config"
use "../cache"
use "../http-get"
use "../fmt/fmt-completers.nu"
use "../config/config-completers.nu"

export def resource-completer [context: string] {
  try {
    resources -o wide | get -o names | flatten
  } catch {|e|
    $e.rendered | save -f e.txt
  }
}

export def api-resources-output-completer [context: string] { fmt-completers output | where {$in != full} }
export def verbs-completer [context: string] { resources -o wide | get verbs | flatten | uniq }
export def group-completer [] { resources | get group | uniq }
export def version-completer [] { [ v1alpha1 v1beta1 v1 ] }

def discover-api [conf, --context(-c): string] {
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
  resourcename?: string
  --group(-g): string
  --version(-v): string
  --verbs(-V): list<string>
  --output(-o): string
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

  if $output != wide {
    $res = ($res | select ...[
      name
      version
      group
      namespaced
      kind
      names
    ])
  }

  if ($group | is-not-empty) {
    $res = $res | where group == $group
  }
  if ($version | is-not-empty) {
    $res = $res | where version == $version
  }
  if $namespaced {
    $res = $res | where namespaced == true
  }
  if ($verbs | is-not-empty) {
    $res = $res | where {|resource|
      $verbs | all {|verb| $verb in ($resource.verbs? | default [])}
    }
  }
  if ($resourcename | is-not-empty) {
    $res = $res | where {|r| $resourcename in $r.names}
    if ($res | length) == 0 { 
      error make --unspanned {msg: $"($resourcename) is not a resource from the cluster. Run 'nuke api-resources | get names | flatten' to get the full list"}
    }
  }
  $res
}

# lists all API resources available in the cluster.
export def resources [
  resourcename?: string@resource-completer
  --output(-o): string@api-resources-output-completer
  --verbs(-v): list<string>@verbs-completer
  --group(-g): string@group-completer
  --version(-v): string@version-completer
  --context(-c): string@"config-completers context"
  --namespaced(-n)
] {
  if ($output | is-not-empty) and not ($output in (fmt-completers output)) {
    error make {
      msg: $'Supported outputs are (fmt-completers output)'
      label: {
        text: $'($output) is not a supported output'
        span: (metadata $output).span
      }
    }
  }
  let conf = config
  let cache_file = $'($context | default $conf.current-context).apis'

  let cached = cache read $cache_file -c 7day
  if ($cached | is-not-empty) {
    return (
      fmt-api-resources $cached $resourcename 
      -g $group 
      -v $version
      -V $verbs 
      -o $output 
      --namespaced=$namespaced
    )
  }

  let res = discover-api $conf -c $context

  cache write $cache_file $res
  (
    fmt-api-resources $res $resourcename 
    -g $group 
    -v $version
    -V $verbs 
    -o $output 
    --namespaced=$namespaced
  )
}

# ----------------
#  api versions   
# ----------------

def fmt-api-versions [
  content: any
  groupname?: string
  --version(-v): string
  --output(-o): string
] {
  mut c = $content
  if ($groupname | is-not-empty) {
    $c = $c | where name == $groupname
  }
  if ($version | is-not-empty) {
    $c = $c | where {|v|
      $version in $v.versions.version
    }
  }
  if $output == wide {
    $c = ($c | reject versions.resources)
  } else {
    $c = ($c.versions | flatten).groupVersion
  }
  $c
}

# lists all API versions available in the cluster.
export def versions [
  groupname?: string@group-completer
  --version(-v): string@version-completer
  --output(-o): string@api-resources-output-completer = compact
  --context(-c): string@"config-completers context"
] {
  let conf = config
  let cache_file = $'($context | default $conf.current-context).apis'

  let cached = cache read $cache_file -c 7day
  if ($cached | is-not-empty) {
    return (fmt-api-versions $cached $groupname -v $version -o $output)
  }
  let res = discover-api $conf -c $context
  cache write $cache_file $res
  fmt-api-versions $res $groupname -v $version -o $output
}
