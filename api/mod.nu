use "../config"
use "../cache"
use "../http-get"
use ./discovery.nu
use "../fmt/fmt-completers.nu"
use "../config/config-completers.nu"

export def resource-completer [context: string] {
  if ($context | is-empty) {
    return (resources | get name)
  }

  mut prev = $context | parse --regex '(?P<word>\S+)' | get word

  let idx = $prev | enumerate | where {$in.item in ['-k', '--kubeconfpath']} | get index
  let kubeconfpath = if ($idx | is-empty) { null } else {
    $prev | get (($idx | first) + 1)
  }

  let idx = $prev | enumerate | where {$in.item in ['-c', '--context', '-C', '--cluster']} | get index
  let current_context = if ($idx | is-empty) { null } else {
    $prev | get (($idx | first) + 1)
  }

  resources -k $kubeconfpath -c $current_context | get name
}

export def verbs-completer [context: string] { resources -o wide | get verbs | flatten | uniq }
export def group-completer [] { resources | get group | uniq }
export def version-completer [] { [ v1alpha1 v1beta1 v1 ] }

# -----------------
#  api resources   
# -----------------

export def resources [
  resourcename?: string@resource-completer
  --verbs(-v): list<string>@verbs-completer
  --group(-g): string@group-completer
  --version(-v): string@version-completer
  --namespaced(-n)
  --output(-o): string@"fmt-completers output-no-full"
  --kubeconf(-K): record
  --kubeconfpath(-k): path
  --context(-c): string@"config-completers context"
  --cluster(-C): string@"config-completers cluster"
] {

  let index = (
    discovery resource-index
    -K $kubeconf
    -k $kubeconfpath
    -c $context
    -C $cluster
  )

  mut candidates = $index.flat
  if ($resourcename | is-not-empty) {
    let needle = ($resourcename | str downcase)
    let canonical = (
      if ($needle in ($index.by_name | columns)) {
        $needle
      } else if ($needle in ($index.by_alias | columns)) {
        $index.by_alias | get $needle
      } else {
        error make --unspanned {
          msg: $"($resourcename) is not a resource from the cluster."
        }
      }
    )

    $candidates = $index.by_name | get $canonical
  }

  # ---------------------------------------
  # Apply filters to small list only
  # ---------------------------------------

  if ($group | is-not-empty) {
    $candidates = $candidates | where group == $group
  }

  if ($version | is-not-empty) {
    $candidates = $candidates | where version == $version
  }

  if $namespaced {
    $candidates = $candidates | where namespaced == true
  }

  if ($verbs | is-not-empty) {
    $candidates = $candidates | where {|r|
      $verbs | all {|verb| $verb in ($r.verbs? | default [])}
    }
  }

  if $output != wide {
    $candidates = (
      $candidates | select -o name version group namespaced kind
    )
  }

  $candidates
}

# ----------------
#  api versions   
# ----------------

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
  let kubeconf = if ($kubeconf | is-not-empty) { $kubeconf } else {
    config -k $kubeconfpath
  } 

  mut content = discovery load -K $kubeconf -c $context -C $cluster
  if ($groupname | is-not-empty) {
    $content = $content | where name == $groupname
  }
  if ($version | is-not-empty) {
    $content = $content | where {|v|
      $version in $v.versions.version
    }
  }
  if $output == wide {
    $content = ($content | reject versions.resources)
  } else {
    $content = ($content.versions | flatten).groupVersion
  }
  $content
}

# ------------
#  resolver   
# ------------

export def resolve-resource [
  needle: string
  --kubeconf(-K): record
  --kubeconfpath(-k): path
  --context(-c): string
  --cluster(-C): string
] {

  let idx = (
    discovery resource-index
    -K $kubeconf
    -k $kubeconfpath
    -c $context
    -C $cluster
  )
  let needle = ($needle | str downcase)

  # ---------------------------------------
  # Canonical name resolution
  # ---------------------------------------

  let canonical = (
    if ($needle in ($idx.by_name | columns)) {
      $needle
    } else if ($needle in ($idx.by_alias | columns)) {
      $idx.by_alias | get $needle
    } else {
      error make --unspanned {
        msg: $"($needle) is not a resource from the cluster."
      }
    }
  )

  let candidates = $idx.by_name | get $canonical

  if ($candidates | length) == 1 {
    return ($candidates | first)
  }

  # ---------------------------------------
  # Preferred version resolution
  # ---------------------------------------

  let annotated = (
    $candidates | each {|r|

      let preferred_version = ($idx.preferred | get -o $r.group?)

      let is_preferred = (
        $preferred_version != null
        and $r.version == $preferred_version
      )

      let is_stable = (
        not ($r.version | str contains "alpha")
        and not ($r.version | str contains "beta")
      )

      $r
      | insert is_preferred $is_preferred
      | insert is_stable $is_stable
    }
  )

  # preferred wins
  let preferred = $annotated | where {$in.is_preferred}
  if ($preferred | length) == 1 {
    return ($preferred | first)
  }

  let current = if ($preferred | length) > 0 { $preferred } else { $annotated }

  # prefer non-core over legacy "api"
  let core = $current | where group == "api"
  if ($core | length) == 1 {
    return ($core | first)
  }

  let current = if ($core | length) > 0 { $core } else { $current }

  # prefer stable over alpha/beta
  let stable = $current | where is_stable
  if ($stable | length) == 1 {
    return ($stable | first)
  }

  let current = if ($stable | length) > 0 { $stable } else { $current }

  $current
  | sort-by group version
  | first
}
