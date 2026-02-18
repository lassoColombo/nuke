export def "poddisruptionbudgets v1" [output?: string = compact] {
  let pdb = $in

  let cond = (
    $pdb.status.conditions?
    | default []
    | where type == DisruptionAllowed
    | first
    | default {}
  )

  let policy = (
    if ($pdb.spec.maxUnavailable? != null) {
      { maxUnavailable: $pdb.spec.maxUnavailable }
    } else {
      { minAvailable: $pdb.spec.minAvailable }
    }
  )

  let res = {
    name: $pdb.metadata.name
    allowed: ($cond.status? | default "Unknown")
    disruptionsAllowed: ($pdb.status.disruptionsAllowed? | default 0)
    healthy: ($pdb.status.currentHealthy? | default 0)
    expected: ($pdb.status.expectedPods? | default 0)
    age: ($pdb.metadata.creationTimestamp? | helpers fmtage)
  } | merge $policy

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert selector ($pdb.spec.selector?.matchLabels?)
  | upsert desiredHealthy ($pdb.status.desiredHealthy?)
  | upsert observedGeneration ($pdb.status.observedGeneration?)
  | upsert condition (
    {
      status: $cond.status?
      reason: $cond.reason?
      message: $cond.message?
      lastTransitionTime: $cond.lastTransitionTime?
    }
  )
  | upsert status ($pdb.status? | reject -o conditions)
}
