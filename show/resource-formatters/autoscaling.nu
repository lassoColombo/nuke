use "../../fmt/helpers.nu"

# -----------------------
# helpers
# -----------------------

def "hpa target" [] {
  let ref = $in.spec.scaleTargetRef?
  if ($ref | is-empty) {
    null
  } else {
    $"($ref.kind | str downcase)/($ref.name)"
  }
}

def "hpa replicas" [] {
  {
    min: ($in.spec.minReplicas? | default 1)
    max: ($in.spec.maxReplicas? | default 1)
    current: ($in.status.currentReplicas? | default 0)
    desired: ($in.status.desiredReplicas? | default 0)
  }
}

# -----------------------
# HPA v1
# -----------------------

export def "horizontalpodautoscalers v1" [output?: string = compact] {
  let h = $in

  let target = ($h | hpa target)
  let r = ($h | hpa replicas)

  let cpu = (
    if ($h.spec.targetCPUUtilizationPercentage? | is-not-empty) {
      {
        metric: "cpu"
        target: $h.spec.targetCPUUtilizationPercentage
        current: $h.status.currentCPUUtilizationPercentage?
      }
    } else {
      null
    }
  )

  let base = (
    $h
    | helpers meta base
    | merge {
        target: $target
        cpu: $cpu
        ...$r
      }
  )

  if $output == "compact" {
    return $base
  }

  let conditions = (
    $h.status.conditions?
    | default []
    | each {|c|
      {
        type: $c.type
        status: $c.status
        reason: $c.reason?
        message: $c.message?
        updated: ($c.lastTransitionTime? | helpers cvt-time)
      }
    }
  )

  $base | merge {
    owner: ($h | helpers meta owner)
    conditions: $conditions
  }
}


# -----------------------
# HPA v2
# -----------------------

export def "horizontalpodautoscalers v2" [output?: string = compact] {
  let h = $in

  let target = ($h | hpa target)
  let r = ($h | hpa replicas)

  let metrics = ($h.status.currentMetrics? | default [])

  let base = (
    $h
    | helpers meta base
    | merge {
        target: $target
        metrics: ($metrics | length)
        ...$r
      }
  )

  if $output == "compact" {
    return $base
  }

  let metricsSpec = (
    $h.spec.metrics?
    | default []
    | each {|m|
        {
          type: $m.type
          resource: $m.resource?
          pods: $m.pods?
          object: $m.object?
          external: $m.external?
        }
      }
  )

  let metricsStatus = (
    $metrics
    | each {|m|
        {
          type: $m.type
          resource: $m.resource?
          pods: $m.pods?
          object: $m.object?
          external: $m.external?
        }
      }
  )

  let conditions = (
    $h.status.conditions?
    | default []
    | each {|c|
      {
        type: $c.type
        status: $c.status
        reason: $c.reason?
        message: $c.message?
        updated: ($c.lastTransitionTime? | helpers cvt-time)
      }
    }
  )

  $base | merge {
    owner: ($h | helpers meta owner)
    metricsSpec: $metricsSpec
    metricsStatus: $metricsStatus
    behavior: ($h.spec.behavior? | default {})
    conditions: $conditions
  }
}
