export def main [output?: string = compact] {
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
