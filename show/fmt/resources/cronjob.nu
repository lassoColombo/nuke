export def main [output?: string = compact] {
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
