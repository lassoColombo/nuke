use "../helpers.nu"

export def main [output?: string = compact] {
  let pod = $in
  let cs = ($pod.status.containerStatuses? | default [])

  let waiting = (
    $cs
    | where state?.waiting? != null
    | get state.waiting
  )

  let terminated = (
    $cs
    | where state?.terminated? != null
    | get state.terminated
  )

  let ready_count = ($cs | where ready == true | length)
  let total_count = ($pod.spec.containers | length)

  let ready_cond = (
    $pod.status.conditions?
    | default []
    | where type == "Ready"
    | first
    | default {}
  )

  let status = (
    if ($waiting | is-not-empty) {
      $waiting | first | get -o reason
    } else if ($terminated | is-not-empty) {
      $terminated | first | get -o reason
    } else if ($ready_cond.status? == "False") {
      "NotReady"
    } else {
      $pod.status.phase
    }
  )

  let res = {
    name: $pod.metadata.name
    status: $status
    ready: $ready_count
    total: $total_count
    restarts: (
      $cs | reduce --fold 0 {|c acc| $acc + ($c.restartCount? | default 0)}
    )
    age: ($pod.metadata.creationTimestamp? | helpers fmtage)
    podIP: $pod.status.podIP?
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  let owner = (
    $pod.metadata.ownerReferences?
    | default []
    | where controller == true
    | if (($in | length) != 0) {$in} else {[{}]}
    | first
  )
  let owner = if ($owner | is-empty) { null } else { $"($owner.kind | str downcase)/($owner.name)" }

  let containers = (
    $pod.spec.containers
    | each {|c|
      let cstat = ($cs | where name == $c.name | first | default {})
      {
        name: $c.name
        image: $c.image
        command: $c.command?
        args: $c.args?
        ready: $cstat.ready?
        restarts: $cstat.restartCount?
        state: (
          if ($cstat.state?.running? != null) {
            "running"
          } else if ($cstat.state?.waiting? != null) {
            { waiting: $cstat.state.waiting.message? }
          } else if ($cstat.state?.terminated? != null) {
            {
              terminated: {
                reason: $cstat.state.terminated.message?
                exitCode: $cstat.state.terminated.exitCode?
              }
            }
          } else {
            null
          }
        )
        ...($c.resources? | helpers fmtresources)
      }
    }
  )
  $res | merge {
    qos: $pod.status.qosClass?
    owner: $owner
    containers: $containers
    node: $pod.spec.nodeName?
  }
}
