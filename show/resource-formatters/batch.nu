use "../../fmt/helpers.nu"

# ----------------
#  cronjobs
# ----------------

export def "cronjobs v1" [output?: string = compact] {
  let cj = $in

  let active = (
    $cj.status.active?
    | default []
    | length
  )

  let last_schedule = (
    $cj.status.lastScheduleTime?
    | helpers cvt-time
  )

  let base = (
    $cj
    | helpers meta base
    | merge {
      schedule: $cj.spec.schedule
      suspend: ($cj.spec.suspend? | default false)
      active: $active
      lastSchedule: $last_schedule
    }
  )

  if $output == "compact" {
    return $base
  }

  let tpl = $cj.spec.jobTemplate.spec?

  $base | merge {

    owner: ($cj | helpers meta owner)

    concurrencyPolicy: ($cj.spec.concurrencyPolicy? | default "Allow")

    startingDeadlineSeconds: $cj.spec.startingDeadlineSeconds?

    successfulJobsHistory: (
      $cj.spec.successfulJobsHistoryLimit?
      | default 3
    )

    failedJobsHistory: (
      $cj.spec.failedJobsHistoryLimit?
      | default 1
    )

    containers: (
      $cj.spec.jobTemplate
      | helpers fmt-contaienrs
    )

    images: (
      $cj.spec.jobTemplate
      | helpers fmt-images
    )

    restartPolicy: $tpl.template.spec.restartPolicy?

    lastSuccessful: (
      $cj.status.lastSuccessfulTime?
      | helpers cvt-time
    )
  }
}


# ----------------
#  jobs
# ----------------

export def "jobs v1" [output?: string = compact] {
  let job = $in

  let completions = ($job.spec.completions? | default 1)
  let succeeded = ($job.status.succeeded? | default 0)
  let failed = ($job.status.failed? | default 0)
  let active = ($job.status.active? | default 0)

  let start = (
    $job.status.startTime?
    | helpers cvt-time
  )

  let completion = (
    $job.status.completionTime?
    | helpers cvt-time
  )

  let duration = (
    if ($start | is-empty) {
      null
    } else if ($completion | is-empty) {
      (date now) - $start
    } else {
      $completion - $start
    }
  )

  let status = (
    if $succeeded >= $completions {
      "Complete"
    } else if $failed > 0 and ($job.spec.backoffLimit? | default 6) <= $failed {
      "Failed"
    } else if $active > 0 {
      "Running"
    } else {
      "Pending"
    }
  )

  let base = (
    $job
    | helpers meta base
    | merge {
      status: $status
      completions: $succeeded
      desired: $completions
      duration: $duration
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {

    owner: ($job | helpers meta owner)

    active: $active
    failed: $failed

    parallelism: ($job.spec.parallelism? | default 1)

    backoffLimit: ($job.spec.backoffLimit? | default 6)

    startTime: $start
    completionTime: $completion

    containers: ($job | helpers fmt-contaienrs)
    images: ($job | helpers fmt-images)

    restartPolicy: (
      $job.spec.template.spec.restartPolicy?
    )

    conditions: (
      $job.status.conditions?
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
  }
}
