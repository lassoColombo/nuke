use "../../fmt/helpers.nu"

export def "controllerrevisions v1" [output?: string = compact ] {
  let cr = $in

  let owner_ref = (
    $cr.metadata.ownerReferences?
    | default []
    | where controller == true
  )

  let owner = if ($owner_ref | length) != 1 {
    null
  } else {
    let o = $owner_ref | first 
    $"($o.kind | str downcase)/($o.name)"
  }

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
    age: ($cr.metadata.creationTimestamp? | helpers fmt-age)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }
  $res
  | upsert generation ($cr.metadata.annotations.'deprecated.daemonset.template.generation'?)
  | upsert labels ($cr.metadata.labels?)
  | upsert container_names ($cr.data.spec.template.spec.containers? | default [] | get -o name)
  | upsert container_images ($cr.data.spec.template.spec.containers? | default [] | get -o image)
}

export def "daemonsets v1" [output?: string = compact] {
  let ds = $in
  
  let meta = ($ds | helpers meta base)
  
  let replicas = {
    desired: ($ds.status.desiredNumberScheduled? | default 0)
    current: ($ds.status.currentNumberScheduled? | default 0)
    ready: ($ds.status.numberReady? | default 0)
    available: ($ds.status.numberAvailable? | default 0)
    unavailable: ($ds.status.numberUnavailable? | default 0)
  }
  
  let progressing = ($ds | helpers status condition "Progressing")
  let status = (
    if ($progressing.reason? == "ProgressDeadlineExceeded") {
      "Failed"
    } else if ($replicas.current < $replicas.desired) {
      "Updating"
    } else if ($replicas.available < $replicas.desired) {
      "NotReady"
    } else {
      "Ready"
    }
  )

  let base = ($meta | merge {
    status: $status
    ...$replicas
  })

  if $output == "compact" {
    return $base
  }
  
  $base | merge {
    selector: ($ds | helpers spec selector)
    strategy: ($ds | helpers spec strategy)
    containers: ($ds | helpers tpl containers)
    revision: ($ds.metadata.annotations."daemonset.kubernetes.io/revision"?)
    paused: ($ds.spec.paused? | default false)
    owner: ($ds | helpers meta owner)
  }
}

export def "deployments v1" [output?: string = compact] {
  let d = $in

  let meta = ($d | helpers meta base)
  let replicas = ($d | helpers status replicas)
  let progressing = ($d | helpers status condition "Progressing")

  let status = (
    if ($progressing.reason? == "ProgressDeadlineExceeded") {
      "Failed"
    } else if ($replicas.updated < $replicas.desired) {
      "Updating"
    } else if ($replicas.available < $replicas.desired) {
      "NotReady"
    } else {
      "Ready"
    }
  )

  let base = ($meta | merge {
    status: $status
    ...$replicas
  })

  if $output == "compact" {
    return $base
  }

  $base | merge {
    selector: ($d | helpers spec selector)
    strategy: ($d | helpers spec strategy)
    containers: ($d | helpers tpl containers)
    revision: ($d.metadata.annotations."deployment.kubernetes.io/revision"?)
    paused: ($d.spec.paused? | default false)
    owner: ($d | helpers meta owner)
  }
}

export def "replicasets v1" [output?: string = compact] {
  let rs = $in
  
  let desired = ($rs.spec.replicas? | default 1)
  
  let ready = ($rs.status.readyReplicas? | default 0)
  let available = ($rs.status.availableReplicas? | default 0)
  let fully_labeled = ($rs.status.fullyLabeledReplicas? | default 0)
  
  let status = (
    if ($available < $desired) {
      "NotReady"
    } else {
      "Ready"
    }
  )

  let owner_ref = (
    $rs.metadata.ownerReferences?
    | default []
    | where controller == true
  )

  let owner = if ($owner_ref | length) != 1 {
    null
  } else {
    let o = $owner_ref | first 
    $"($o.kind | str downcase)/($o.name)"
  }
  
  let revision = (
    $rs.metadata.annotations."deployment.kubernetes.io/revision"?
  )

  let res = {
    name: $rs.metadata.name
    status: $status
    desired: $desired
    ready: $ready
    available: $available
    fullyLabeled: $fully_labeled
    owner: $owner
    revision: $revision
    age: ($rs.metadata.creationTimestamp? | helpers fmt-age)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }
  
  let selector = ($rs.spec.selector.matchLabels? | default {})
  
  let tpl = $rs.spec.template

  let containers = (
    $tpl.spec.containers
    | each {|c|
      {
        name: $c.name
        image: $c.image
        command: $c.command?
        args: $c.args?
        ...($c.resources? | helpers fmt-resources)
      }
    }
  )

  let images = ($tpl.spec.containers | get image)

  let conds = ($rs.status.conditions? | default [])

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

# ----------------
#  statefulsets   
# ----------------

export def "statefulsets v1" [output?: string = compact] {
  let ss = $in
  
  let meta = ($ss | helpers meta base)

  let replicas = {
    desired: ($ss.spec.replicas? | default 1)
    current: ($ss.status.currentReplicas? | default 0)
    ready: ($ss.status.readyReplicas? | default 0)
    updated: ($ss.status.updatedReplicas? | default 0)
    available: ($ss.status.availableReplicas? | default 0)
  }
  
  let progressing = ($ss | helpers status condition "Progressing")

  let status = (
    if ($progressing.reason? == "ProgressDeadlineExceeded") {
      "Failed"
    } else if ($replicas.updated < $replicas.desired) {
      "Updating"
    } else if ($replicas.ready < $replicas.desired) {
      "NotReady"
    } else {
      "Ready"
    }
  )

  let base = ($meta | merge {
    status: $status
    ...$replicas
  })

  if $output == "compact" {
    return $base
  }

  
  let pvcs = (
    $ss.spec.volumeClaimTemplates?
    | default []
    | each {|v|
        {
          name: $v.metadata.name
          storageClass: $v.spec.storageClassName?
          accessModes: ($v.spec.accessModes? | default [])
          requests: ($v.spec.resources.requests? | default {})
        }
      }
  )

  let strategy = ($ss.spec.updateStrategy? | default {})
  let rolling = ($strategy.rollingUpdate? | default {})

  $base | merge {
    service: $ss.spec.serviceName?
    selector: ($ss | helpers spec selector)
    strategy: {
      type: ($strategy.type? | default "RollingUpdate")
      partition: $rolling.partition?
    }
    podManagementPolicy: ($ss.spec.podManagementPolicy? | default "OrderedReady")
    containers: ($ss | helpers tpl containers)
    volumeClaims: $pvcs
    revision: ($ss.metadata.annotations."controller-revision-hash"?)
    owner: ($ss | helpers meta owner)
  }
}
