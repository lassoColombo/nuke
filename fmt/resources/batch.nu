export def "cronjobs v1" [output?: string = compact] {
  let cj = $in

  let res = {
    name: $cj.metadata.name
    schedule: $cj.spec.schedule
    timezone: $cj.spec.timezone?
    suspend: ($cj.spec.suspend? | default false)
    concurrency: $cj.spec.concurrencyPolicy?
    successfulJobsHistory: ($cj.spec.successfulJobsHistoryLimit? | default null)
    failedJobsHistory: ($cj.spec.failedJobsHistoryLimit? | default null)
    last-schedule: (
      if ($cj.status.lastScheduleTime? | is-not-empty) {
        $cj.status.lastScheduleTime | into datetime
      } else {
        null
      }
    )
    last-success: (
      if ($cj.status.lastSuccessfulTime? | is-not-empty) {
        $cj.status.lastSuccessfulTime | into datetime
      } else {
        null
      }
    )
    active: ($cj.status.active? | default [] | length)
    age: ($cj.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert generation ($cj.metadata.generation?)
  | upsert restartPolicy ($cj.spec.jobTemplate.spec.template.spec.restartPolicy?)
  | upsert containers (
    $cj.spec.jobTemplate.spec.template.spec.containers?
    | default []
    | select name image command
  )
}

export def "jobs v1" [output?: string = compact] {
  let j = $in

  let conditions = ($j.status.conditions? | default [])

  let complete_cond = ($conditions | where type == Complete
    | if ($in | is-not-empty) {$in} else {[{}]} | first)
  let failed_cond   = ($conditions | where type == Failed
    | if ($in | is-not-empty) {$in} else {[{}]} | first)

  let status = (
    if ($complete_cond.status? == "True") {
      "Complete"
    } else if ($failed_cond.status? == "True") {
      "Failed"
    } else if (($j.status.active? | default 0) > 0) {
      "Running"
    } else {
      "Pending"
    }
  )

  let start = (
    if ($j.status.startTime? | is-not-empty) {
      $j.status.startTime | into datetime
    } else {
      null
    }
  )

  let end = (
    if ($j.status.completionTime? | is-not-empty) {
      $j.status.completionTime | into datetime
    } else {
      null
    }
  )

  let duration = (
    if ($start != null and $end != null) {
      $end - $start
    } else {
      null
    }
  )

  let res = {
    name: $j.metadata.name
    status: $status
    succeeded: ($j.status.succeeded? | default 0)
    failed: ($j.status.failed? | default 0)
    active: ($j.status.active? | default 0)
    completions: $j.spec.completions
    duration: $duration
    age: ($j.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  let owner = (
    $j.metadata.ownerReferences?
    | default []
    | where controller == true
    | first
  )

  $res
  | upsert startTime $start
  | upsert completionTime $end
  | upsert backoffLimit ($j.spec.backoffLimit?)
  | upsert parallelism ($j.spec.parallelism?)
  | upsert owner (
    if ($owner != null) {
      $"($owner.kind | str downcase)/($owner.name)"
    } else {
      null
    }
  )
  | upsert condition (
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
  | upsert containers ($j | helpers fmtcontainers)
}
