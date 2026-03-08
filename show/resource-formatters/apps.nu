use "../../fmt/helpers.nu"

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

# ----------------
#  replicasets
# ----------------

export def "replicasets v1" [output?: string = compact] {
  let rs = $in

  let meta = ($rs | helpers meta base)

  let replicas = {
    desired: ($rs.spec.replicas? | default 1)
    current: ($rs.status.replicas? | default 0)
    ready: ($rs.status.readyReplicas? | default 0)
    available: ($rs.status.availableReplicas? | default 0)
  }

  let status = (
    if ($replicas.current < $replicas.desired) {
      "Scaling"
    } else if ($replicas.ready < $replicas.desired) {
      "NotReady"
    } else {
      "Ready"
    }
  )

  let base = (
    $meta
    | merge {
      status: $status
      ...$replicas
    }
  )

  if $output == "compact" {
    return $base
  }

  let conditions = (
    $rs.status.conditions?
    | default []
    | each {|c|
        {
          type: $c.type
          status: $c.status
          reason: $c.reason?
          message: $c.message?
          updated: ($c.lastTransitionTime? | helpers fmt-time)
        }
      }
  )

  $base | merge {
    selector: ($rs | helpers spec selector)
    images: ($rs | helpers tpl images)
    containers: ($rs | helpers tpl containers)
    owner: ($rs | helpers meta owner)
    revision: $rs.metadata.annotations."deployment.kubernetes.io/revision"
    conditions: $conditions
  }
}

# -----------------------
#  controllerrevisions
# -----------------------

export def "controllerrevisions v1" [output?: string = compact] {
  let cr = $in

  let controller = ($cr | helpers meta owner)

  let base = (
    $cr
    | helpers meta base
    | merge {
      controller: $controller
      revision: ($cr.revision? | default 0)
    }
  )

  if $output == "compact" {
    return $base
  }

  let data = $cr.data? | default {}
  $base | merge {
    labels: ($cr.metadata.labels? | default {})
    annotations: ($cr.metadata.annotations? | default {})
    dataKeys: (
      if ($data | describe) =~ "^record" {
        $data | columns
      } else {
        []
      }
    )
  }
}
