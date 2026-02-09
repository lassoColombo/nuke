export def main [output: string = compact] {
  let fs = $in

  let rules = ($fs.spec.rules? | default [])

  let dangling_cond = (
    $fs.status.conditions?
    | default []
    | where type == Dangling
    | first
    | default {}
  )

  let res = {
    name: $fs.metadata.name
    priority: ($fs.spec.priorityLevelConfiguration.name?)
    precedence: ($fs.spec.matchingPrecedence?)
    rules: ($rules | length)
    dangling: ($dangling_cond.status? | default "Unknown")
    age: ($fs.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert generation ($fs.metadata.generation?)
  | upsert subjects (
    $rules
    | each {|r|
      $r.subjects?
      | default []
      | each {|s|
        if ($s.user? != null) {
          { kind: "User", name: $s.user.name }
        } else if ($s.group? != null) {
          { kind: "Group", name: $s.group.name }
        } else if ($s.serviceAccount? != null) {
          {
            kind: "ServiceAccount"
            name: $s.serviceAccount.name
            namespace: $s.serviceAccount.namespace?
          }
        } else {
          null
        }
      }
    }
    | flatten
    | where $it != null
  )
  | upsert rules (
    $rules
    | each {|r|
      {
        resourceRules: ($r.resourceRules? | default [])
        nonResourceRules: ($r.nonResourceRules? | default [])
      }
    }
  )
  | upsert conditions ($fs.status.conditions?)
}
