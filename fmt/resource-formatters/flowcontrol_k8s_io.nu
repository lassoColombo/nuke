
# ------
#  v1   
# ------

export def "flowschemas v1" [output: string = compact] {
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

export def "prioritylevelconfigurations v1" [output?: string = compact] {
  let plc = $in

  let limited = ($plc.spec.limited? | default {})
  let queuing = ($limited.limitResponse?.queuing? | default {})

  let res = {
    name: $plc.metadata.name
    type: ($plc.spec.type?)
    concurrencyShares: ($limited.nominalConcurrencyShares? | default 0)
    handSize: ($queuing.handSize? | default 0)
    queues: ($queuing.queues? | default 0)
    queueLengthLimit: ($queuing.queueLengthLimit? | default 0)
    age: ($plc.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  $res
  | upsert generation ($plc.metadata.generation?)
  | upsert queuing (
      if ($queuing | is-empty) {
        null
      } else {
        {
          handSize: $queuing.handSize?
          queues: $queuing.queues?
          queueLengthLimit: $queuing.queueLengthLimit?
        }
      }
    )
  | upsert lendablePercent ($limited.lendablePercent? | default null)
  | upsert limitResponseType ($limited.limitResponse.type? | default null)
  | upsert created (
      if ($plc.metadata.creationTimestamp? | is-not-empty) {
        $plc.metadata.creationTimestamp | into datetime
      } else {
        null
      }
    )
}
