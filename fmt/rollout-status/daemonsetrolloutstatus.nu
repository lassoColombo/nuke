export def main [output?: string = compact] {
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
