export def "clusterroles v1" [output?: string = compact] {
  let r = $in
  let res = {
    name: $r.metadata.name
    created: $r.metadata.creationTimestamp
    rules: ($r.rules? | default [] | length)
  }
  if ($output | is-empty) or $output == compact {
    $res
  } else {
    $res
    | upsert aggregationRule ($r.aggregationRule?.clusterRoleSelectors?)
    | update rules ($r.rules)
  }
}

export def "clusterrolebindings v1" [output?: string = compact] {
  let r = $in
  let subjects = ($r.subjects? | default [])
  let users = ($subjects | where kind == User | get name | default [])
  let groups = ($subjects | where kind == Group | get name | default [])
  let serviceaccounts = ($subjects | where kind == ServiceAccount | each {|s| 
    $'($s.namespace | default '')/($s.name)'
  } | default [])

  let res = {
    name: $r.metadata.name
    role: ($r.roleRef.name)
    users: ($users | length)
    groups: ($groups | length)
    serviceaccounts: ($serviceaccounts | length)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert subjects $subjects
  | update role ($r.roleRef | select -o kind name)
  | update users $users
  | update groups $groups
  | update serviceaccounts $serviceaccounts
  | insert created $r.metadata.creationTimestamp
}

export def "roles v1" [output?: string = compact] {
  let r = $in
  let res = {
    name: $r.metadata.name
    created: ($r.metadata.creationTimestamp? | into datetime)
    rules: ($r.rules? | default [] | length)
  }
  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | update rules ($r.rules)
}

export def "rolebindings v1" [output?: string = compact] {
  let r = $in
  let subjects = ($r.subjects? | default [])
  let users = ($subjects | where kind == User | get name | default [])
  let groups = ($subjects | where kind == Group | get name | default [])
  let serviceaccounts = ($subjects | where kind == ServiceAccount | each {|s| 
    $'($s.namespace | default $r.metadata.namespace)/($s.name)'
  } | default [])

  let res = {
    name: $r.metadata.name
    namespace: ($r.metadata.namespace?)
    role: ($r.roleRef.name)
    users: ($users | length)
    groups: ($groups | length)
    serviceaccounts: ($serviceaccounts | length)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert subjects $subjects
  | update role ($r.roleRef | select -o kind name)
  | update users $users
  | update groups $groups
  | update serviceaccounts $serviceaccounts
  | insert created $r.metadata.creationTimestamp
}
