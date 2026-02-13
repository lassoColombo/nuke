use "../show/fmt/helpers.nu"

export def main [
] {
  {
    status: {
      let obj = $in
      let kind = $obj.kind

      let conditions = ($obj.status.conditions? | default [])

      let progressing = (
        $conditions
        | where type == "Progressing"
        | first
        | default {}
      )

      let available = (
        $conditions
        | where type == "Available"
        | first
        | default {}
      )

      let failed = (
        $conditions
        | where type == "ReplicaFailure"
        | first
        | default {}
      )

      mut res = {
        observedGeneration: $obj.status.observedGeneration?
        generation: $obj.metadata.generation?
        replicas: ($obj.status.replicas? | default 0)
        updated: ($obj.status.updatedReplicas? | default 0)
        ready: ($obj.status.readyReplicas? | default 0)
        available: ($obj.status.availableReplicas? | default 0)
        progressing: ($progressing.status? | default null)
        strategy: ($obj.spec.strategy? | default {})
        age: ($obj.metadata.creationTimestamp? | helpers fmtage)
      }

      let failed = ($failed.status? | default null)
      if ($failed | is-not-empty) {
        $res = $res | insert failed $failed
      }


      let progressingCondition = (
        if ($progressing | is-empty) { null } else {
          {
            status: $progressing.status
            reason: $progressing.reason?
            message: $progressing.message?
            lastUpdateTime: (
              if ($progressing.lastUpdateTime? | is-not-empty) {
                $progressing.lastUpdateTime | into datetime
              } else { null }
            )
          }
        }
      )
      if ($progressingCondition | is-not-empty) {
        $res = $res | insert progressingCondition $progressingCondition
      }

      let availableCondition = (
        if ($available | is-empty) { null } else {
          {
            status: $available.status
            reason: $available.reason?
            message: $available.message?
          }
        }
      )
      if ($availableCondition | is-not-empty) {
        $res = $res | insert availableCondition $availableCondition
      }


      let failureCondition = (
        if ($failed | is-empty) { null } else {
          {
            status: $failed.status
            reason: $failed.reason?
            message: $failed.message?
          }
        }
      )
      if ($failureCondition | is-not-empty) {
        $res = $res | insert failureCondition $failureCondition
      }


      $res
    }

    history: {|output?: string = compact| 
      # todo
    }
  }
}

