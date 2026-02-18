export def "daemonsets v1" [output?: string = compact] {
  let obj = $in

  let desired = ($obj.status.desiredNumberScheduled? | default 0)

  let replicas = {
    desired: $desired
    total: ($obj.status.currentNumberScheduled? | default 0)
    updated: ($obj.status.updatedNumberScheduled? | default 0)
    ready: ($obj.status.numberReady? | default 0)
    available: ($obj.status.numberAvailable? | default 0)
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
    progressing: (not $rolloutComplete)
    failed: false
    strategy: ($obj.spec.updateStrategy? | default {})
    age: ($obj.metadata.creationTimestamp? | helpers fmtage)
  }
}

export def "deployments v1" [output?: string = compact] {
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

export def "replicasets v1" [output?: string = compact] {
  let obj = $in

  let desired = ($obj.spec.replicas? | default 1)

  let replicas = {
    desired: $desired
    total: ($obj.status.replicas? | default 0)
    updated: null
    ready: ($obj.status.readyReplicas? | default 0)
    available: ($obj.status.availableReplicas? | default 0)
  }

  let observed = ($obj.status.observedGeneration? | default 0)
  let generation = ($obj.metadata.generation? | default 0)

  let rolloutComplete = (
    $observed >= $generation
    and $replicas.ready == $replicas.desired
  )

  {
    kind: $obj.kind
    name: $obj.metadata.name
    namespace: $obj.metadata.namespace?
    replicas: $replicas
    rolloutComplete: $rolloutComplete
    progressing: (not $rolloutComplete)
    failed: false
    strategy: {}
    age: ($obj.metadata.creationTimestamp? | helpers fmtage)
  }
}

export def "replicationcontrollers v1" [output?: string = compact] {
  let obj = $in

  let desired = ($obj.spec.replicas? | default 1)

  let replicas = {
    desired: $desired
    total: ($obj.status.replicas? | default 0)
    updated: null
    ready: ($obj.status.readyReplicas? | default 0)
    available: null
  }

  let rolloutComplete = (
    $replicas.ready == $replicas.desired
  )

  {
    kind: $obj.kind
    name: $obj.metadata.name
    namespace: $obj.metadata.namespace?
    replicas: $replicas
    rolloutComplete: $rolloutComplete
    progressing: (not $rolloutComplete)
    failed: false
    strategy: {}
    age: ($obj.metadata.creationTimestamp? | helpers fmtage)
  }
}

export def "statefulsets v1" [output?: string = compact] {
  let obj = $in

  let desired = ($obj.spec.replicas? | default 1)

  let replicas = {
    desired: $desired
    total: ($obj.status.replicas? | default 0)
    updated: ($obj.status.updatedReplicas? | default 0)
    ready: ($obj.status.readyReplicas? | default 0)
    available: null
  }

  let currentRev = $obj.status.currentRevision?
  let updateRev  = $obj.status.updateRevision?

  let rolloutComplete = (
    $currentRev == $updateRev
    and $replicas.ready == $replicas.desired
  )

  {
    kind: $obj.kind
    name: $obj.metadata.name
    namespace: $obj.metadata.namespace?
    replicas: $replicas
    rolloutComplete: $rolloutComplete
    progressing: (not $rolloutComplete)
    failed: false
    strategy: {
      type: "RollingUpdate"
      partition: ($obj.spec.updateStrategy.rollingUpdate?.partition? | default 0)
    }
    age: ($obj.metadata.creationTimestamp? | helpers fmtage)
  }
}
