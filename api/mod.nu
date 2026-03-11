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

export def get-resource-completer [context: string] {
  if ($context | is-empty) {
    return (resources --verbs [get] | get name)
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

  resources --verbs [get] -k $kubeconfpath -c $current_context | get name
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

  mut candidates = (
    $index.flat | where { not ($in.name | str contains /) }
  )
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
  # Canonical name resolution (alias → plural name)
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
  # Build priority pattern list:
  #
  # 1. {group: "", version: "v1"}          ← core always first
  # 2. For each non-core group (discovery order):
  #    {group: G, version: preferredVersion}
  # 3. For each non-core group:
  #    {group: G, version: "*"}             ← any version of that group
  # 4. Catch-all: {group: "*", version: "*"}
  # ---------------------------------------

  # Collect non-core groups in the order they appear among candidates
  # (preserving discovery order as best we can from the index)
  let non_core_groups = (
    $candidates
    | where { $in.group != "api" and ($in.group | is-not-empty) }
    | each { $in.group }
    | uniq
  )

  mut patterns = [
    { group: "",  version: "v1" }
  ]

  for grp in $non_core_groups {
    let preferred = ($idx.preferred | get -o $grp)
    if ($preferred | is-not-empty) {
      $patterns = ($patterns | append { group: $grp, version: $preferred })
    }
  }

  for grp in $non_core_groups {
    $patterns = ($patterns | append { group: $grp, version: "*" })
  }

  $patterns = ($patterns | append { group: "*", version: "*" })

  # ---------------------------------------
  # Apply patterns iteratively (PriorityRESTMapper.ResourceFor logic):
  # - filter current candidates by pattern
  # - 0 matches → skip pattern
  # - 1 match   → return it
  # - N matches  → narrow candidates to these N, continue
  # ---------------------------------------

  mut remaining = $candidates

  for pattern in $patterns {
    let matched = (
      $remaining | where {|r|
        let group_ok = (
          $pattern.group == "*"
          or $pattern.group == $r.group
          or ($pattern.group == "" and $r.group == "api")
        )
        let version_ok = (
          $pattern.version == "*"
          or $pattern.version == $r.version
        )
        $group_ok and $version_ok
      }
    )

    match ($matched | length) {
      0 => { continue }
      1 => { return ($matched | first) }
      _ => { $remaining = $matched }
    }
  }

  # Exhausted all patterns with multiple remaining — return first
  $remaining | first
}
