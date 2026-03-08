use "../../fmt/helpers.nu"

def "rbac subjects" [] {
  let subs = ($in | default [])

  {
    users: ($subs | where kind == User | get name | default [])
    groups: ($subs | where kind == Group | get name | default [])
    serviceaccounts: (
      $subs
      | where kind == ServiceAccount
      | each {|s| $"($s.namespace | default '')/($s.name)" }
      | default []
    )
  }
}

def "rbac roleref" [] {
  let r = $in.roleRef?
  if ($r | is-empty) {
    null
  } else {
    $"($r.kind | str downcase)/($r.name)"
  }
}

def "rbac rules-count" [] {
  $in.rules? | default [] | length
}

export def "roles v1" [output?: string = compact] {
  let r = $in

  let rules = ($r | rbac rules-count)

  let base = (
    $r
    | helpers meta base
    | merge {
      rules: $rules
    }
  )

  if $output == "compact" {
    return $base
  }

  let rulesSpec = (
    $r.rules?
    | default []
    | each {|rule|
      {
        apiGroups: ($rule.apiGroups? | default [])
        resources: ($rule.resources? | default [])
        verbs: ($rule.verbs? | default [])
        resourceNames: ($rule.resourceNames? | default [])
        nonResourceURLs: ($rule.nonResourceURLs? | default [])
      }
    }
  )

  $base | merge {
    owner: ($r | helpers meta owner)
    rulesSpec: $rulesSpec
  }
}

export def "clusterroles v1" [output?: string = compact] {
  let cr = $in

  let rules = ($cr | rbac rules-count)

  let base = (
    $cr
    | helpers meta base
    | merge {
      rules: $rules
    }
  )

  if $output == "compact" {
    return $base
  }

  let aggregation = (
    $cr.aggregationRule?.clusterRoleSelectors?
    | default []
    | each {|s| $s.matchLabels? | default {} }
  )

  let rulesSpec = (
    $cr.rules?
    | default []
    | each {|rule|
      {
        apiGroups: ($rule.apiGroups? | default [])
        resources: ($rule.resources? | default [])
        verbs: ($rule.verbs? | default [])
        resourceNames: ($rule.resourceNames? | default [])
        nonResourceURLs: ($rule.nonResourceURLs? | default [])
      }
    }
  )

  $base | merge {
    owner: ($cr | helpers meta owner)
    aggregationSelectors: $aggregation
    rulesSpec: $rulesSpec
  }
}

export def "rolebindings v1" [output?: string = compact] {
  let rb = $in

  let role = ($rb | rbac roleref)

  let base = (
    $rb
    | helpers meta base
    | merge {
      role: $role
    }
  )

  if $output == "compact" {
    return $base
  }

  let subjects = ($rb.subjects? | rbac subjects)

  $base | merge {
    owner: ($rb | helpers meta owner)
    subjects: $subjects
    roleRef: $rb.roleRef?
  }
}

export def "clusterrolebindings v1" [output?: string = compact] {
  let crb = $in

  let role = ($crb | rbac roleref)

  let base = (
    $crb
    | helpers meta base
    | merge {
      role: $role
    }
  )

  if $output == "compact" {
    return $base
  }

  let subjects = ($crb.subjects? | rbac subjects)

  $base | merge {
    subjects: $subjects
    roleRef: $crb.roleRef?
  }
}
