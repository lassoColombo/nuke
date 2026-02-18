export def "controllerrevisions v1" [output?: string = compact ] {
  let cr = $in

  let owner = (
    $cr.metadata.ownerReferences? 
    | default [] 
    | where controller == true 
    | first
  )

  let controller_str = (
    if ($owner != null) {
      $'($owner.kind | str downcase).($owner.apiVersion)/($owner.name)'
    } else {
      null
    }
  )

  let res = {
    name: $cr.metadata.name
    controller: ($owner | default {} | select kind name)
    revision: $cr.revision?
    age: ($cr.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }
  $res
  | upsert generation ($cr.metadata.annotations.'deprecated.daemonset.template.generation'?)
  | upsert labels ($cr.metadata.labels?)
  | upsert container_names ($cr.data.spec.template.spec.containers? | get name)
  | upsert container_images ($cr.data.spec.template.spec.containers? | get image)
}

export def "daemonsets v1" [output?: string = compact] {
  let ds = $in

  # --- Desired pods (one per eligible node)
  let desired = ($ds.status.desiredNumberScheduled? | default 0)

  # --- Current status counters
  let current = ($ds.status.currentNumberScheduled? | default 0)
  let ready = ($ds.status.numberReady? | default 0)
  let available = ($ds.status.numberAvailable? | default 0)
  let unavailable = ($ds.status.numberUnavailable? | default 0)

  let updated = ($ds.status.updatedNumberScheduled? | default 0)
  let misscheduled = ($ds.status.numberMisscheduled? | default 0)

  # --- Update strategy
  let strategy = ($ds.spec.updateStrategy.type? | default "RollingUpdate")

  let rolling = ($ds.spec.updateStrategy.rollingUpdate? | default {})

  let max_unavailable = ($rolling.maxUnavailable?)
  let max_surge = ($rolling.maxSurge?)

  # --- Rollout status
  let status = (
    if ($updated < $desired) {
      "Updating"
    } else if ($ready < $desired) {
      "NotReady"
    } else if ($misscheduled > 0) {
      "Misscheduled"
    } else {
      "Ready"
    }
  )

  # --- Base compact record
  let res = {
    name: $ds.metadata.name
    status: $status
    desired: $desired
    current: $current
    ready: $ready
    available: $available
    unavailable: $unavailable
    updated: $updated
    misscheduled: $misscheduled
    strategy: $strategy
    age: ($ds.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  # =========================================================
  # WIDE / DESCRIBE VIEW
  # =========================================================

  # --- Selector
  let selector = ($ds.spec.selector.matchLabels? | default {})

  # --- Template
  let tpl = $ds.spec.template

  let containers = (
    $tpl.spec.containers
    | each {|c|
      {
        name: $c.name
        image: $c.image
        command: $c.command?
        args: $c.args?
        ...($c.resources? | helpers fmtresources)
      }
    }
  )

  let images = ($tpl.spec.containers | get image)

  # --- Scheduling constraints
  let node_selector = ($tpl.spec.nodeSelector? | default {})
  let tolerations = ($tpl.spec.tolerations? | default [])
  let affinity = ($tpl.spec.affinity?)

  # --- Owner
  let owner_ref = (
    $ds.metadata.ownerReferences?
    | default []
    | where controller == true
    | default [{}]
    | first
  )

  let owner = if ($owner_ref | is-empty) {
    null
  } else {
    $"($owner_ref.kind | str downcase)/($owner_ref.name)"
  }

  # --- Conditions
  let conds = ($ds.status.conditions? | default [])

  let progressing = (
    $conds
    | where type == "Progressing"
    | first
    | default {}
  )

  let labels = ($tpl.metadata.labels? | default {})
  let annotations = ($tpl.metadata.annotations? | default {})

  $res | merge {
    selector: $selector

    updateStrategy: {
      type: $strategy
      rollingUpdate: {
        maxUnavailable: $max_unavailable
        maxSurge: $max_surge
      }
    }

    scheduling: {
      nodeSelector: $node_selector
      tolerations: $tolerations
      affinity: $affinity
    }

    images: $images
    containers: $containers

    template: {
      labels: $labels
      annotations: $annotations
    }

    conditions: {
      progressing: {
        status: $progressing.status?
        reason: $progressing.reason?
        message: $progressing.message?
      }
    }

    owner: $owner
  }
}

export def "deployments v1" [output?: string = compact] {
  let d = $in

  # --- Spec basics
  let desired = ($d.spec.replicas? | default 1)
  let strategy = ($d.spec.strategy.type? | default "RollingUpdate")

  let rolling = ($d.spec.strategy.rollingUpdate? | default {})

  let max_unavailable = (
    $rolling.maxUnavailable?
    | default 1
  )

  let max_surge = (
    $rolling.maxSurge?
    | default 1
  )

  # --- Status counters (typed ints)
  let updated = ($d.status.updatedReplicas? | default 0)
  let ready = ($d.status.readyReplicas? | default 0)
  let available = ($d.status.availableReplicas? | default 0)
  let unavailable = ($d.status.unavailableReplicas? | default 0)

  # --- Conditions
  let conds = ($d.status.conditions? | default [])

  let progressing = (
    $conds
    | where type == "Progressing"
    | first
    | default {}
  )

  let available_cond = (
    $conds
    | where type == "Available"
    | first
    | default {}
  )

  # --- Rollout status (kubectl-like)
  let status = (
    if ($progressing.reason? == "ProgressDeadlineExceeded") {
      "Failed"
    } else if ($updated < $desired) {
      "Updating"
    } else if ($available < $desired) {
      "NotReady"
    } else {
      "Ready"
    }
  )

  # --- Base compact record
  let res = {
    name: $d.metadata.name
    status: $status
    desired: $desired
    updated: $updated
    ready: $ready
    available: $available
    unavailable: $unavailable
    age: ($d.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  # =========================================================
  # WIDE / DESCRIBE VIEW
  # =========================================================

  # --- Selector
  let selector = ($d.spec.selector.matchLabels? | default {})

  # --- Pod template info
  let tpl = $d.spec.template

  let images = (
    $tpl.spec.containers
    | get image
  )

  let container_names = (
    $tpl.spec.containers
    | get name
  )

  # --- Resources per container
  let containers = (
    $tpl.spec.containers
    | each {|c|
      {
        name: $c.name
        image: $c.image
        command: $c.command?
        args: $c.args?
        ...($c.resources? | helpers fmtresources)
      }
    }
  )

  # --- Owner
  let owner = (
    $d.metadata.ownerReferences?
    | default []
    | where controller == true
    | default [{}]
    | first
  )

  let owner = if ($owner | is-empty) {
    null
  } else {
    $"($owner.kind | str downcase)/($owner.name)"
  }

  # --- Paused flag
  let paused = ($d.spec.paused? | default false)

  # --- Revision (annotation)
  let revision = (
    $d.metadata.annotations."deployment.kubernetes.io/revision"?
  )

  # --- Progress deadline
  let progress_deadline = (
    $d.spec.progressDeadlineSeconds?
  )

  $res | merge {
    strategy: {
      type: $strategy
      rollingUpdate: {
        maxUnavailable: $max_unavailable
        maxSurge: $max_surge
      }
    }

    selector: $selector

    images: $images
    containers: $containers

    revision: $revision
    paused: $paused

    progressDeadlineSeconds: $progress_deadline

    conditions: {
      progressing: {
        status: $progressing.status?
        reason: $progressing.reason?
        message: $progressing.message?
      }
      available: {
        status: $available_cond.status?
        reason: $available_cond.reason?
        message: $available_cond.message?
      }
    }

    owner: $owner
  }
}

export def "replicasets v1" [output?: string = compact] {
  let rs = $in

  # --- Desired replicas
  let desired = ($rs.spec.replicas? | default 1)

  # --- Status counters
  let ready = ($rs.status.readyReplicas? | default 0)
  let available = ($rs.status.availableReplicas? | default 0)
  let fully_labeled = ($rs.status.fullyLabeledReplicas? | default 0)

  # --- Status (kubectl-like)
  let status = (
    if ($available < $desired) {
      "NotReady"
    } else {
      "Ready"
    }
  )

  # --- Owner (usually Deployment)
  let owner_ref = (
    $rs.metadata.ownerReferences?
    | default []
    | where controller == true
    | default [{}]
    | first
  )

  let owner = if ($owner_ref | is-empty) {
    null
  } else {
    $"($owner_ref.kind | str downcase)/($owner_ref.name)"
  }

  # --- Revision annotation (important for rollouts)
  let revision = (
    $rs.metadata.annotations."deployment.kubernetes.io/revision"?
  )

  # --- Base compact record
  let res = {
    name: $rs.metadata.name
    status: $status
    desired: $desired
    ready: $ready
    available: $available
    fullyLabeled: $fully_labeled
    owner: $owner
    revision: $revision
    age: ($rs.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  # =========================================================
  # WIDE / DESCRIBE VIEW
  # =========================================================

  # --- Selector
  let selector = ($rs.spec.selector.matchLabels? | default {})

  # --- Pod template
  let tpl = $rs.spec.template

  let containers = (
    $tpl.spec.containers
    | each {|c|
      {
        name: $c.name
        image: $c.image
        command: $c.command?
        args: $c.args?
        ...($c.resources? | helpers fmtresources)
      }
    }
  )

  let images = ($tpl.spec.containers | get image)

  # --- Conditions (ReplicaSets rarely use many, but include)
  let conds = ($rs.status.conditions? | default [])

  let failure_cond = (
    $conds
    | where type == "ReplicaFailure"
    | first
    | default {}
  )

  # --- Pod template metadata
  let labels = ($tpl.metadata.labels? | default {})
  let annotations = ($tpl.metadata.annotations? | default {})

  $res | merge {
    selector: $selector

    images: $images
    containers: $containers

    template: {
      labels: $labels
      annotations: $annotations
    }

    conditions: {
      replicaFailure: {
        status: $failure_cond.status?
        reason: $failure_cond.reason?
        message: $failure_cond.message?
      }
    }
  }
}

export def "statefulsets v1" [output?: string = compact] {
  let sts = $in

  # --- Desired replicas
  let desired = ($sts.spec.replicas? | default 1)

  # --- Status counters
  let ready = ($sts.status.readyReplicas? | default 0)
  let current = ($sts.status.currentReplicas? | default 0)
  let updated = ($sts.status.updatedReplicas? | default 0)
  let available = ($sts.status.availableReplicas? | default 0)

  # --- Update strategy
  let strategy = ($sts.spec.updateStrategy.type? | default "RollingUpdate")

  let partition = (
    $sts.spec.updateStrategy.rollingUpdate.partition?
  )

  # --- Rollout status (kubectl-like)
  let status = (
    if ($updated < $desired) {
      "Updating"
    } else if ($ready < $desired) {
      "NotReady"
    } else {
      "Ready"
    }
  )

  # --- Service name (identity)
  let service = $sts.spec.serviceName?

  # --- Pod management policy
  let pod_policy = ($sts.spec.podManagementPolicy? | default "OrderedReady")

  # --- Revision info
  let current_revision = $sts.status.currentRevision?
  let update_revision = $sts.status.updateRevision?

  # --- Base compact record
  let res = {
    name: $sts.metadata.name
    status: $status
    desired: $desired
    ready: $ready
    current: $current
    updated: $updated
    available: $available
    strategy: $strategy
    service: $service
    age: ($sts.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  # =========================================================
  # WIDE / DESCRIBE VIEW
  # =========================================================

  # --- Selector
  let selector = ($sts.spec.selector.matchLabels? | default {})

  # --- Template
  let tpl = $sts.spec.template

  let containers = (
    $tpl.spec.containers
    | each {|c|
      {
        name: $c.name
        image: $c.image
        command: $c.command?
        args: $c.args?
        ...($c.resources? | helpers fmtresources)
      }
    }
  )

  let images = ($tpl.spec.containers | get image)

  # --- Volume claims (important for StatefulSets)
  let volume_claims = (
    $sts.spec.volumeClaimTemplates?
    | default []
    | each {|v|
      {
        name: $v.metadata.name
        storageClass: $v.spec.storageClassName?
        accessModes: $v.spec.accessModes?
        size: $v.spec.resources.requests.storage?
      }
    }
  )

  # --- Owner
  let owner_ref = (
    $sts.metadata.ownerReferences?
    | default []
    | where controller == true
    | default [{}]
    | first
  )

  let owner = if ($owner_ref | is-empty) {
    null
  } else {
    $"($owner_ref.kind | str downcase)/($owner_ref.name)"
  }

  # --- Conditions
  let conds = ($sts.status.conditions? | default [])

  let failure_cond = (
    $conds
    | where type == "ReplicaFailure"
    | first
    | default {}
  )

  let labels = ($tpl.metadata.labels? | default {})
  let annotations = ($tpl.metadata.annotations? | default {})

  $res | merge {
    selector: $selector

    podManagementPolicy: $pod_policy

    revisions: {
      current: $current_revision
      update: $update_revision
    }

    partition: $partition

    images: $images
    containers: $containers

    volumeClaims: $volume_claims

    template: {
      labels: $labels
      annotations: $annotations
    }

    conditions: {
      replicaFailure: {
        status: $failure_cond.status?
        reason: $failure_cond.reason?
        message: $failure_cond.message?
      }
    }

    owner: $owner
  }
}
