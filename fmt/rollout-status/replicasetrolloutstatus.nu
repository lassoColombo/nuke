export def main [output?: string = compact] {
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
