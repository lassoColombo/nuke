
# ------
#  v1   
# ------

export def "horizontalpodautoscalers v1" [output: string = compact] {
  let hpa = $in

  let conditions = ($hpa.status.conditions? | default [])

  let able_cond = (
    $conditions
    | where type == AbleToScale
    | first
    | default {}
  )

  let active_cond = (
    $conditions
    | where type == ScalingActive
    | first
    | default {}
  )

  let target = (
    $hpa.spec.scaleTargetRef?
    | default {}
    | select -o kind name
  )

  let res = {
    name: $hpa.metadata.name
    target: $target
    min: ($hpa.spec.minReplicas? | default null)
    max: ($hpa.spec.maxReplicas)
    current: ($hpa.status.currentReplicas? | default 0)
    desired: ($hpa.status.desiredReplicas? | default 0)
    ableToScale: ($able_cond.status?)
    scalingActive: ($active_cond.status?)
    age: ($hpa.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert namespace ($hpa.metadata.namespace?)
  | upsert generation ($hpa.metadata.generation?)
  | upsert metrics (
    $hpa.spec.metrics?
    | default []
    | each {|m|
      if ($m.resource? != null) {
        {
          type: "Resource"
          name: $m.resource.name
          targetType: $m.resource.target.type
          averageUtilization: ($m.resource.target.averageUtilization? | default null)
          averageValue: ($m.resource.target.averageValue? | default null)
        }
      } else {
        {
          type: $m.type?
        }
      }
    }
  )
  | upsert currentMetrics (
    $hpa.status.currentMetrics?
    | default []
  )
  | upsert conditions (
    $conditions
    | each {|c|
      {
        type: $c.type
        status: $c.status
        reason: $c.reason?
        message: $c.message?
        lastTransitionTime: (
          if ($c.lastTransitionTime? | is-not-empty) {
            $c.lastTransitionTime | into datetime
          } else {
            null
          }
        )
      }
    }
  )
}
