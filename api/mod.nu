use "../config"
use "../cache"
use "../http-get"
use "../fmt/fmt-completers.nu"
use "../config/config-completers.nu"

export def resource-completer [context: string] {
  if ($context | is-empty) {
    resources -o wide | get -o names | flatten
  } 

  mut prev = $context | parse --regex '(?P<word>\S+)' | get word

  let idx = $prev | enumerate | where {$in.item in ['-k', '--kubeconfpath']} | get index
  let kubeconfpath = if ($idx | is-not-empty) {
    $prev | get (($idx | first) + 1)
  } else {
    ''
  }

  let idx = $prev | enumerate | where {$in.item in ['-c', '--context', '-C', '--cluster']} | get index
  let current_context = if ($idx | is-not-empty) {
    $prev | get (($idx | first) + 1)
  } else {
    ''
  }

  resources -k $kubeconfpath -c $current_context -o wide | get -o names | flatten
}

export def verbs-completer [context: string] { resources -o wide | get verbs | flatten | uniq }
export def group-completer [] { resources | get group | uniq }
export def version-completer [] { [ v1alpha1 v1beta1 v1 ] }

def discover-api [
  --kubeconf(-K): record,
  --context(-c): string
  --cluster(-C): string
] {
  print $"(ansi cyan)discovering api resources...(ansi reset)"
  let c = http-get {path: api} -K $kubeconf -c $context -C $cluster
  let core = $c
  | update versions {
    $c.versions | each {|version|
      {version: $version, groupVersion: api/v1} 
      | insert resources (http-get {path: $'api/($version)'} -K $kubeconf -c $context -C $cluster).resources 
    }
  }
  | upsert name api
  | reject serverAddressByClientCIDRs
  | reject kind

  let noncore = (http-get {path: apis} -K $kubeconf -c $context -C $cluster).groups
  | each {|group|
    $group | update versions (
      $group.versions | each {|version|
        $version | insert resources (http-get {path: $'apis/($version.groupVersion)'} -K $kubeconf -c $context -C $cluster).resources 
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

  if ($resourcename | is-not-empty) {
    $res = $res | where {|r| $resourcename in $r.names}
    if ($res | length) == 0 {
      error make --unspanned {msg: $"($resourcename) is not a resource from the cluster. Run 'nuke api-resources | get name' to get the full list"}
    }
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

  $res
}

# Lists all API resources available in the cluster.
export def resources [
  resourcename?: string@resource-completer # Optional resource to get (defaults to all).
  --verbs(-v): list<string>@verbs-completer # Filter by list of verbs.
  --group(-g): string@group-completer # Filter by group.
  --version(-v): string@version-completer # Filter by version.
  --namespaced(-n) # Get only namespaced resources.

  --output(-o): string@"fmt-completers output-no-full" # The format of the output (compact wide full).

  --kubeconf(-K): record # The configuration to use (defaults to kubeconfig).
  --kubeconfpath(-k): path # The path to the kubeconfig (defaults to $env.KUBECONFIG or ~/.kube/config).
  --context(-c): string@"config-completers context" # The context to use in the configuration (defaults to current).
  --cluster(-C): string@"config-completers cluster" # The cluster to use in the configuration (defaults to current).
] {
  if ($output | is-not-empty) and not ($output in (fmt-completers output)) {
    error make --unspanned { msg: $'Supported outputs are (fmt-completers output)' }
  }
  let kubeconf = if ($kubeconf | is-not-empty) {
    $kubeconf
  } else if ($kubeconfpath | is-not-empty) {
    open -r $kubeconfpath | from yaml
  } else {
    config
  }
  let cache_file = $'($context | default $kubeconf.current-context).apis'

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

  let res = discover-api -K $kubeconf -c $context -C $cluster

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

# Lists all API versions available in the cluster.
export def versions [
  groupname?: string@group-completer # Group to get.
  --version(-v): string@version-completer # Filter by version.

  --output(-o): string@"fmt-completers output-no-full" = compact # The format of the output (compact wide full).

  --kubeconf(-K): record # The configuration to use (defaults to kubeconfig).
  --kubeconfpath(-k): path # The path to the kubeconfig (defaults to $env.KUBECONFIG or ~/.kube/config).
  --context(-c): string@"config-completers context" # The context to use in the configuration (defaults to current).
  --cluster(-C): string@"config-completers cluster" # The cluster to use in the configuration (defaults to current).
] {
  let kubeconf = if ($kubeconf | is-not-empty) {
    $kubeconf
  } else {
    config -k $kubeconfpath
  } 

  let cache_file = $'($context | default $kubeconf.current-context).apis'
  let cached = cache read $cache_file -c 7day
  if ($cached | is-not-empty) {
    return (fmt-api-versions $cached $groupname -v $version -o $output)
  }

  let res = discover-api -K $kubeconf -c $context -C $cluster
  cache write $cache_file $res
  fmt-api-versions $res $groupname -v $version -o $output
}
