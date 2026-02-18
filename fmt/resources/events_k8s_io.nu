export def "events v1" [
  output?: string = compact
] {
  let ev = $in
  let mode = ($output | default compact)

  let ts = (
    $ev.eventTime?
    | default $ev.series?.lastObservedTime?
    | default $ev.metadata?.creationTimestamp?
    | default (0 | into datetime -f '%s')
    | into datetime
  )

  let obj_kind = ($ev.regarding.kind? | str downcase)
  let obj_name = ($ev.regarding.name?)

  let object = (
    if ($obj_kind | is-empty) or ($obj_name | is-empty) {
      null
    } else {
      $"($obj_kind)/($obj_name)"
    }
  )

  let message = ($ev.note? | default "")

  let base = {
    time: $ts
    type: ($ev.type? | default "Normal")
    reason: ($ev.reason? | default "")
    object: $object
    message: ($message | str substring 0..120)
  }

  if $mode == compact {
    $base
  } else {
    $base
    | insert namespace ($ev.regarding.namespace?)
    | insert count (
        $ev.series.count?
        | default 1
      )
    | insert source (
        $ev.reportingController?
        | default $ev.reportingInstance?
      )
    | upsert message $message
    | insert action ($ev.action?)
    | insert raw $ev
  }
}
