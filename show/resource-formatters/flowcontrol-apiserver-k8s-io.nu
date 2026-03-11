use "../../fmt/helpers.nu"

# -----------------------
# FlowSchemas
# -----------------------
export def "flowschemas v1" [output?: string = compact] {
  let fs = $in

  let pl = $fs.spec.priorityLevelConfiguration?.name?

  let distinguisher = (
    $fs.spec.distinguisherMethod?.type?
  )

  let dangling = (
    $fs.status.conditions?
    | default []
    | where type == "Dangling"
    | where status == "True"
    | length
  )

  let missingpl = ($dangling > 0)

  let base = (
    $fs
    | helpers meta base
    | merge {
        precedence: ($fs.spec.matchingPrecedence? | default 0)
        priorityLevel: $pl
        distinguisher: $distinguisher
        missingPL: $missingpl
      }
  )

  if $output == "compact" {
    return $base
  }

  let rules = (
    $fs.spec.rules?
    | default []
    | each {|r|
        {
          subjects: ($r.subjects? | default [])
          resourceRules: ($r.resourceRules? | default [])
          nonResourceRules: ($r.nonResourceRules? | default [])
        }
      }
  )

  let conditions = (
    $fs.status.conditions?
    | default []
    | each {|c|
      {
        type: $c.type
        status: $c.status
        reason: $c.reason?
        message: $c.message?
        updated: ($c.lastTransitionTime? | helpers fmt-time)
      }
    }
  )

  $base | merge {
    owner: ($fs | helpers meta owner)
    rules: $rules
    conditions: $conditions
  }
}

# -----------------------
# PriorityLevelConfigurations
# -----------------------

export def "prioritylevelconfigurations v1" [output?: string = compact] {
  let pl = $in

  let spec = ($pl.spec? | default {})

  let limited = ($spec.limited? | default {})
  let queuing = ($limited.limitResponse?.queuing? | default {})

  let base = (
    $pl
    | helpers meta base
    | merge {
        type: ($spec.type? | default "Limited")

        shares: ($limited.nominalConcurrencyShares?)
        queues: ($queuing.queues?)
      }
  )

  if $output == "compact" {
    return $base
  }

  let conditions = (
    $pl.status.conditions?
    | default []
    | each {|c|
      {
        type: $c.type
        status: $c.status
        reason: $c.reason?
        message: $c.message?
        updated: ($c.lastTransitionTime? | helpers fmt-time)
      }
    }
  )

  $base | merge {
    owner: ($pl | helpers meta owner)

    lendablePercent: ($limited.lendablePercent?)
    borrowingLimitPercent: ($limited.borrowingLimitPercent?)

    limitResponse: ($limited.limitResponse?)

    queuing: (
      if ($queuing | is-empty) {
        null
      } else {
        {
          queues: $queuing.queues?
          handSize: $queuing.handSize?
          queueLengthLimit: $queuing.queueLengthLimit?
        }
      }
    )

    conditions: $conditions
  }
}
