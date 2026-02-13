export def main [output?: string = compact] {
  let obj = $in

  let conditions = ($obj.status.conditions? | default [])

  let get_condition = {|conds, t|
    $conds | where type == $t | first | default null
  }

  let progressing = (do $get_condition $conditions "Progressing")
  let failedCond  = (do $get_condition $conditions "ReplicaFailure")

  let desired = ($obj.spec.replicas? | default 1)

  let replicas = {
    desired: $desired
    total: ($obj.status.replicas? | default 0)
    updated: ($obj.status.updatedReplicas? | default 0)
    ready: ($obj.status.readyReplicas? | default 0)
    available: ($obj.status.availableReplicas? | default 0)
  }

  let observed = ($obj.status.observedGeneration? | default 0)
  let generation = ($obj.metadata.generation? | default 0)

  let rolloutComplete = (
    $observed >= $generation
    and $replicas.updated == $replicas.desired
    and $replicas.available == $replicas.desired
  )

  {
    kind: $obj.kind
    name: $obj.metadata.name
    namespace: $obj.metadata.namespace?
    replicas: $replicas
    rolloutComplete: $rolloutComplete
    progressing: ($progressing.status? == "True")
    failed: ($failedCond.status? == "True")
    strategy: ($obj.spec.strategy? | default {})
    age: ($obj.metadata.creationTimestamp? | helpers fmtage)
  }
}
