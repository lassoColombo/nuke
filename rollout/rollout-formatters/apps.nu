use "../../fmt/helpers.nu"

# -------------------------------
# helpers (local to this file)
# -------------------------------

def "rollout fmt-conditions" [] {
  $in
  | default []
  | each {|c|
      {
        type:    $c.type
        status:  $c.status
        reason:  ($c.reason? | default null)
        message: ($c.message? | default null)
        updated: ($c.lastTransitionTime? | helpers cvt-time)
      }
    }
}

def "rollout get-condition" [type: string] {
  $in | where type == $type | first | default null
}

export def "deployments v1" [output?: string = compact] {
  let obj = $in

  let conditions  = ($obj.status.conditions? | default [] | rollout fmt-conditions)
  let progressing = ($conditions | rollout get-condition "Progressing")
  let available   = ($conditions | rollout get-condition "Available")
  let replica_failure = ($conditions | rollout get-condition "ReplicaFailure")

  let desired   = ($obj.spec.replicas? | default 1)
  let updated   = ($obj.status.updatedReplicas? | default 0)
  let ready     = ($obj.status.readyReplicas? | default 0)
  let avail     = ($obj.status.availableReplicas? | default 0)
  let total     = ($obj.status.replicas? | default 0)

  let gen      = ($obj.metadata.generation? | default 0)
  let obs_gen  = ($obj.status.observedGeneration? | default 0)
  let gen_current = ($obs_gen >= $gen)

  let paused = ($obj.spec.paused? | default false)

  let deadline_exceeded = (
    ($progressing | is-not-empty)
    and ($progressing.reason? == "ProgressDeadlineExceeded")
  )

  let replica_failed = (
    ($replica_failure | is-not-empty)
    and ($replica_failure.status? == "True")
  )

  let complete = (
    $gen_current
    and ($updated == $desired)
    and ($avail == $desired)
  )

  let phase = (
    if $deadline_exceeded or $replica_failed { "failed" }
    else if $paused                           { "paused" }
    else if not $gen_current                  { "stalled" }
    else if $complete                         { "complete" }
    else                                      { "progressing" }
  )

  let message = (
    match $phase {
      "failed"      => (
        if $deadline_exceeded {
          $"Deployment \"($obj.metadata.name)\" exceeded its progress deadline"
        } else {
          ($replica_failure.message? | default "One or more replicas failed to start")
        }
      )
      "paused"      => $"Deployment \"($obj.metadata.name)\" is paused"
      "stalled"     => "Waiting for controller to observe updated generation"
      "complete"    => (
        $"deployment \"($obj.metadata.name)\" successfully rolled out"
      )
      "progressing" => (
        if $updated < $desired {
          $"Waiting for deployment \"($obj.metadata.name)\" rollout to finish: ($updated) out of ($desired) new replicas have been updated..."
        } else if $total > $updated {
          let old = ($total - $updated)
          $"Waiting for deployment \"($obj.metadata.name)\" rollout to finish: ($old) old replicas are pending termination..."
        } else if $avail < $desired {
          $"Waiting for deployment \"($obj.metadata.name)\" rollout to finish: ($avail) of ($desired) updated replicas are available..."
        } else {
          "Rollout in progress"
        }
      )
      _ => null
    }
  )

  let replicas = {
    desired:   $desired
    updated:   $updated
    ready:     $ready
    available: $avail
  }

  let rollout = {
    generation:          $gen
    observedGeneration:  $obs_gen
    generationCurrent:   $gen_current
  }

  let base = {
    kind:       "Deployment"
    name:       $obj.metadata.name
    namespace:  ($obj.metadata.namespace? | default null)
    phase:      $phase
    message:    $message
    replicas:   $replicas
    rollout:    $rollout
    created:        ($obj.metadata.creationTimestamp? | helpers cvt-time)
  }

  if $output == "compact" {
    return $base
  }

  $base | merge {
    conditions: $conditions
    strategy: ($obj.spec.strategy? | default {})
    paused: $paused
    revision: ($obj.metadata.annotations."deployment.kubernetes.io/revision"? | default null)
  }
}

export def "daemonsets v1" [output?: string = compact] {
  let obj = $in

  let conditions = ($obj.status.conditions? | default [] | rollout fmt-conditions)

  let desired   = ($obj.status.desiredNumberScheduled? | default 0)
  let scheduled = ($obj.status.currentNumberScheduled? | default 0)
  let updated   = ($obj.status.updatedNumberScheduled? | default 0)
  let ready     = ($obj.status.numberReady? | default 0)
  let avail     = ($obj.status.numberAvailable? | default 0)
  let misscheduled = ($obj.status.numberMisscheduled? | default 0)

  let gen       = ($obj.metadata.generation? | default 0)
  let obs_gen   = ($obj.status.observedGeneration? | default 0)
  let gen_current = ($obs_gen >= $gen)

  let complete = (
    $gen_current
    and ($updated == $desired)
    and ($avail == $desired)
  )

  let phase = (
    if not $gen_current   { "stalled" }
    else if $complete     { "complete" }
    else                  { "progressing" }
  )

  let message = (
    match $phase {
      "stalled"     => "Waiting for controller to observe updated generation"
      "complete"    => (
        $"daemon set \"($obj.metadata.name)\" successfully rolled out"
      )
      "progressing" => (
        if $updated < $desired {
          $"Waiting for daemon set \"($obj.metadata.name)\" rollout to finish: ($updated) out of ($desired) new pods have been updated..."
        } else if $avail < $desired {
          $"Waiting for daemon set \"($obj.metadata.name)\" rollout to finish: ($avail) of ($desired) updated pods are available..."
        } else {
          "Rollout in progress"
        }
      )
      _ => null
    }
  )

  let replicas = {
    desired:   $desired
    updated:   $updated
    ready:     $ready
    available: $avail
  }

  let rollout = {
    generation:          $gen
    observedGeneration:  $obs_gen
    generationCurrent:   $gen_current
  }

  let base = {
    kind:       "DaemonSet"
    name:       $obj.metadata.name
    namespace:  ($obj.metadata.namespace? | default null)
    phase:      $phase
    message:    $message
    replicas:   $replicas
    rollout:    $rollout
    age:        ($obj.metadata.creationTimestamp? | helpers cvt-time)
  }

  if $output == "compact" {
    return $base
  }

  $base | merge {
    conditions:      $conditions
    misscheduled:    $misscheduled
    scheduled:       $scheduled
    updateStrategy:  ($obj.spec.updateStrategy? | default {})
  }
}

export def "statefulsets v1" [output?: string = compact] {
  let obj = $in

  let conditions = ($obj.status.conditions? | default [] | rollout fmt-conditions)

  let desired  = ($obj.spec.replicas? | default 1)
  let total    = ($obj.status.replicas? | default 0)
  let updated  = ($obj.status.updatedReplicas? | default 0)
  let ready    = ($obj.status.readyReplicas? | default 0)
  let current  = ($obj.status.currentReplicas? | default 0)

  let current_rev = ($obj.status.currentRevision? | default null)
  let update_rev  = ($obj.status.updateRevision? | default null)
  let revisions_match = (
    ($current_rev | is-not-empty)
    and ($update_rev | is-not-empty)
    and ($current_rev == $update_rev)
  )

  let gen       = ($obj.metadata.generation? | default 0)
  let obs_gen   = ($obj.status.observedGeneration? | default 0)
  let gen_current = ($obs_gen >= $gen)

  let partition = (
    $obj.spec.updateStrategy?.rollingUpdate?.partition?
    | default 0
  )
  let partitioned = ($partition > 0)

  let complete = (
    $gen_current
    and $revisions_match
    and ($ready == $desired)
  )

  let phase = (
    if not $gen_current  { "stalled" }
    else if $partitioned { "progressing" }  # partial rollout — intentionally progressing
    else if $complete    { "complete" }
    else                 { "progressing" }
  )

  let message = (
    match $phase {
      "stalled" => "Waiting for controller to observe updated generation"
      "complete" => (
        $"statefulset rolling update complete ($desired) pods at revision ($current_rev)..."
      )
      "progressing" => (
        if $partitioned {
          $"Waiting for partitioned roll out to finish: ($updated) out of ($desired) new pods have been updated..."
        } else if $updated < $desired {
          $"Waiting for ($desired) pods to be ready... ($ready) of ($desired) pods are ready"
        } else if $current < $updated {
          $"Waiting for statefulset rolling update to complete ($current) pods at revision ($current_rev) and ($updated) pods at revision ($update_rev)..."
        } else {
          "Rollout in progress"
        }
      )
      _ => null
    }
  )

  let replicas = {
    desired:   $desired
    updated:   $updated
    ready:     $ready
    available: null   # StatefulSet has no independent availability gate
  }

  let rollout = {
    generation:          $gen
    observedGeneration:  $obs_gen
    generationCurrent:   $gen_current
  }

  let base = {
    kind:       "StatefulSet"
    name:       $obj.metadata.name
    namespace:  ($obj.metadata.namespace? | default null)
    phase:      $phase
    message:    $message
    replicas:   $replicas
    rollout:    $rollout
    age:        ($obj.metadata.creationTimestamp? | helpers cvt-time)
  }

  if $output == "compact" {
    return $base
  }

  $base | merge {
    conditions:     $conditions
    currentRevision: $current_rev
    updateRevision:  $update_rev
    partition:       $partition
    updateStrategy:  ($obj.spec.updateStrategy? | default {})
    podManagementPolicy: ($obj.spec.podManagementPolicy? | default "OrderedReady")
  }
}


export def "replicasets v1" [output?: string = compact] {
  let obj = $in

  let conditions = ($obj.status.conditions? | default [] | rollout fmt-conditions)

  let desired  = ($obj.spec.replicas? | default 1)
  let total    = ($obj.status.replicas? | default 0)
  let ready    = ($obj.status.readyReplicas? | default 0)
  let avail    = ($obj.status.availableReplicas? | default 0)

  let gen       = ($obj.metadata.generation? | default 0)
  let obs_gen   = ($obj.status.observedGeneration? | default 0)
  let gen_current = ($obs_gen >= $gen)

  let complete = (
    $gen_current
    and ($ready == $desired)
  )

  let phase = (
    if not $gen_current { "stalled" }
    else if $complete   { "complete" }
    else                { "progressing" }
  )

  let message = (
    match $phase {
      "stalled"     => "Waiting for controller to observe updated generation"
      "complete"    => $"replicaset \"($obj.metadata.name)\" has ($ready) available replicas"
      "progressing" => $"Waiting for replicaset \"($obj.metadata.name)\": ($ready) of ($desired) replicas are ready"
      _ => null
    }
  )

  let replicas = {
    desired:   $desired
    updated:   null   # RS has no update-tracking field
    ready:     $ready
    available: $avail
  }

  let rollout = {
    generation:          $gen
    observedGeneration:  $obs_gen
    generationCurrent:   $gen_current
  }

  let base = {
    kind:       "ReplicaSet"
    name:       $obj.metadata.name
    namespace:  ($obj.metadata.namespace? | default null)
    phase:      $phase
    message:    $message
    replicas:   $replicas
    rollout:    $rollout
    age:        ($obj.metadata.creationTimestamp? | helpers cvt-time)
  }

  if $output == "compact" {
    return $base
  }

  $base | merge {
    conditions: $conditions
    owner:      ($obj | helpers meta owner)
  }
}

# ---------------------------------------------------------------------------
# ReplicationControllers
#
# Semantically identical to ReplicaSets for rollout purposes.
# No update revision, no rollout conditions.
# ---------------------------------------------------------------------------

export def "replicationcontrollers v1" [output?: string = compact] {
  let obj = $in

  let conditions = ($obj.status.conditions? | default [] | rollout fmt-conditions)

  let desired = ($obj.spec.replicas? | default 1)
  let total   = ($obj.status.replicas? | default 0)
  let ready   = ($obj.status.readyReplicas? | default 0)

  let gen       = ($obj.metadata.generation? | default 0)
  let obs_gen   = ($obj.status.observedGeneration? | default 0)
  let gen_current = ($obs_gen >= $gen)

  let complete = (
    $gen_current
    and ($ready == $desired)
  )

  let phase = (
    if not $gen_current { "stalled" }
    else if $complete   { "complete" }
    else                { "progressing" }
  )

  let message = (
    match $phase {
      "stalled"     => "Waiting for controller to observe updated generation"
      "complete"    => $"replication controller \"($obj.metadata.name)\" successfully scaled"
      "progressing" => $"Waiting for replication controller \"($obj.metadata.name)\": ($ready) of ($desired) replicas are ready"
      _ => null
    }
  )

  let replicas = {
    desired:   $desired
    updated:   null
    ready:     $ready
    available: null
  }

  let rollout = {
    generation:          $gen
    observedGeneration:  $obs_gen
    generationCurrent:   $gen_current
  }

  let base = {
    kind:       "ReplicationController"
    name:       $obj.metadata.name
    namespace:  ($obj.metadata.namespace? | default null)
    phase:      $phase
    message:    $message
    replicas:   $replicas
    rollout:    $rollout
    age:        ($obj.metadata.creationTimestamp? | helpers cvt-time)
  }

  if $output == "compact" {
    return $base
  }

  $base | merge {
    conditions: $conditions
    owner:      ($obj | helpers meta owner)
  }
}
