use "../../fmt/helpers.nu"

# -----------------------
# events.k8s.io Events
# -----------------------

export def "events v1" [output?: string = compact] {
  let e = $in

  let regarding = ($e.regarding? | default {})
  let obj = (
    if ($regarding | is-empty) {
      null
    } else {
      $"($regarding.kind | str downcase)/($regarding.name)"
    }
  )

  let series = ($e.series? | default {})

  let count = (
    if ($series.count? | is-not-empty) {
      $series.count
    } else {
      ($e.deprecatedCount? | default 1)
    }
  )

  let base = (
    $e
    | helpers meta base
    | merge {
      type: $e.type?
      reason: $e.reason?
      object: $obj
      count: $count
    }
  )

  if $output == "compact" {
    return $base
  }

  let event_time = ($e.eventTime? | helpers cvt-time)

  let last_observed = (
    if ($series.lastObservedTime? | is-not-empty) {
      ($series.lastObservedTime | helpers cvt-time)
    } else {
      null
    }
  )

  $base | merge {
    message: $e.note?
    reportingController: $e.reportingController?
    reportingInstance: $e.reportingInstance?

    eventTime: $event_time

    series: (
      if ($series | is-empty) {
        null
      } else {
        {
          count: $series.count?
          lastObserved: $last_observed
        }
      }
    )
  }
}
