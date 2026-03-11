use "../../fmt/helpers.nu"

# --------------
#  configmaps   
# --------------

export def "configmaps v1" [output?: string = compact] {
  let cm = $in

  let data_count = ($cm.data? | default {} | columns | length)
  let binary_count = ($cm.binaryData? | default {} | columns | length)

  let base = (
    $cm
    | helpers meta base
    | merge {
      data: $data_count
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    binaryData: $binary_count
    totalEntries: ($data_count + $binary_count)
    immutable: ($cm.immutable? | default false)
    owner: ($cm | helpers meta owner)

    keys: ($cm.data? | default {} | columns)
    binaryKeys: ($cm.binaryData? | default {} | columns)
  }
}

# ----------
#  events   
# ----------

export def "events v1" [output?: string = compact] {
  let e = $in

  let o = $e.involvedObject?
  let obj = (
    if ($o | is-empty) {
      null
    } else {
      $"($o.kind | str downcase)/($o.name)"
    }
  )

  let s = ($e.source? | default {})
  let src = (
    if ($s.host? | is-empty) {
      $s.component?
    } else {
      $"($s.component?)@($s.host?)"
    }
  )

  let base = (
    $e
    | helpers meta base
    | merge {
      type: $e.type?
      reason: $e.reason?
      object: $obj
      count: ($e.count? | default 1)
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    message: $e.message?
    source: $src

    firstSeen: ($e.firstTimestamp? | helpers fmt-time)
    lastSeen: ($e.lastTimestamp? | helpers fmt-time)

    reportingComponent: $e.reportingComponent?
    reportingInstance: $e.reportingInstance?
  }
}

# --------------
#  namespaces   
# --------------

export def "namespaces v1" [output?: string = compact] {
  let ns = $in

  let base = (
    $ns
    | helpers meta base
    | merge {
      status: ($ns.status.phase? | default "Unknown")
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    finalizers: $ns.spec.finalizers?
  }
}

# ---------
#  nodes   
# ---------

export def "fmt-noderoles" [] {
  let labels = ($in.metadata.labels? | default {})

  let direct = (
    $labels
    | get -o kubernetes.io/role?
    | default {}
  )

  let indirect = (
    $labels
    | columns
    | where {|k| $k | str starts-with "node-role.kubernetes.io/" }
    | each {|k| $k | split row "/" | last }
  )

  ($direct | append $indirect | uniq | where {$in | is-not-empty})
}

export def "fmt-nodestatus-notready" [] {
  let cond = ($in | status condition "Ready")

  if ($cond.status? == "True") {
    "Ready"
  } else if ($cond.status? == "False") {
    "NotReady"
  } else {
    "Unknown"
  }
}

export def "nodes v1" [output?: string = compact] {
  let no = $in

  let base = (
    $no
    | helpers meta base
    | merge {
      roles: ($no | fmt-noderoles)
      status: ($no | fmt-nodestatus-notready)
      version: $no.status.nodeInfo.kubeletVersion?
    }
  )

  if $output == "compact" {
    return $base
  }

  $base
  | insert kernel $no.status.nodeInfo.kernelVersion?
  | insert container-runtime $no.status.nodeInfo.containerRuntimeVersion?
  | insert image $no.status.nodeInfo.osImage?
  | insert internalIPs (
    $no.status.addresses?
    | default []
    | where type == InternalIP
    | get -o address
  )
  | insert externalIPs (
    $no.status.addresses?
    | default []
    | where type == ExternalIP
    | get -o address
  )
}

# --------
#  pods   
# --------

def "fmt pod-phase" [] {
  let pod = $in
  let init_statuses = ($pod.status.initContainerStatuses? | default [])
  let container_statuses = ($pod.status.containerStatuses? | default [])
  let init_containers = ($pod.spec.initContainers? | default [])

  # Start with phase, override with pod-level reason if present
  mut reason = ($pod.status.phase? | default "Unknown")
  if ($pod.status.reason? | is-not-empty) {
    $reason = $pod.status.reason
  }

  # SchedulingGated condition overrides
  let scheduling_gated = (
    $pod.status.conditions?
    | default []
    | where { $in.type == "PodScheduled" and $in.reason? == "SchedulingGated" }
    | is-not-empty
  )
  if $scheduling_gated {
    $reason = "SchedulingGated"
  }

  # --------------------------------------------------
  # Init containers: iterate in order, find first
  # non-completed one and derive reason from it
  # --------------------------------------------------
  let init_count = ($init_containers | length)

  if $init_count > 0 {
    mut init_done = false
    mut init_idx = 0

    for i in 0..<$init_count {
      let name = ($init_containers | get $i).name
      let st = (
        $init_statuses
        | where name == $name
        | get -o 0
      )

      if ($st | is-empty) {
        # no status yet for this init container
        $reason = $"Init:($i)/($init_count)"
        $init_done = true
        break
      }

      let terminated = $st.state?.terminated?
      let waiting = $st.state?.waiting?

      if ($terminated | is-not-empty) {
        if $terminated.exitCode? != 0 {
          # init container failed
          let r = (
            if ($terminated.reason? | is-not-empty) { $terminated.reason }
            else if ($terminated.signal? | default 0) != 0 { $"Signal:($terminated.signal)" }
            else { "Error" }
          )
          $reason = $"Init:($r)"
          $init_done = true
          break
        }
        # exit 0 → this init container completed, continue to next
      } else if ($waiting | is-not-empty) {
        let wr = ($waiting.reason? | default "")
        if $wr != "" and $wr != "PodInitializing" {
          $reason = $"Init:($wr)"
        } else {
          $reason = $"Init:($i)/($init_count)"
        }
        $init_done = true
        break
      } else {
        # running but not completed
        $reason = $"Init:($i)/($init_count)"
        $init_done = true
        break
      }
    }

    # If we broke early (init not done), return now — don't look at regular containers
    if $init_done {
      return $reason
    }
  }

  # --------------------------------------------------
  # Regular containers
  # --------------------------------------------------
  mut has_running = false

  for st in $container_statuses {
    let waiting = $st.state?.waiting?
    let terminated = $st.state?.terminated?
    let running = $st.state?.running?

    if ($waiting | is-not-empty) {
      let wr = ($waiting.reason? | default "")
      if $wr != "" {
        $reason = $wr
      }
    } else if ($terminated | is-not-empty) {
      let tr = ($terminated.reason? | default "")
      if $tr != "" {
        $reason = $tr
      } else if ($terminated.signal? | default 0) != 0 {
        $reason = $"Signal:($terminated.signal)"
      } else if $terminated.exitCode? != 0 {
        $reason = "Error"
      } else {
        $reason = "Completed"
      }
    } else if ($running | is-not-empty) {
      $has_running = true
    }
  }

  # --------------------------------------------------
  # Final overrides
  # --------------------------------------------------

  # If pod failed but no container gave us a reason
  if $pod.status.phase? == "Failed" and $reason == "Failed" {
    $reason = "Error"
  }

  # Terminating: deletionTimestamp set and pod not already terminal
  let is_terminal = ($pod.status.phase? == "Succeeded" or $pod.status.phase? == "Failed")
  if ($pod.metadata.deletionTimestamp? | is-not-empty) and (not $is_terminal) {
    $reason = "Terminating"
  }

  $reason
}

export def "pods v1" [output?: string = compact] {
  let pod = $in
  let cs = ($pod.status.containerStatuses? | default [])
  let init_cs = ($pod.status.initContainerStatuses? | default [])

  let total = ($pod.spec.containers? | default [] | length)

  # Ready: count containers where ready == true
  let ready = ($cs | where { $in.ready? == true } | length)

  # Restarts: sum of restartCount across all containers
  # (kubectl actually shows the highest single container restart count,
  #  plus any restarts from terminated state of previous container)
  let restarts = (
    $cs
    | each {|c| $c.restartCount? | default 0 }
    | if (($in | length) == 0) {[0]} else {$in}
    |  math max
  )

  let base = (
    $pod
    | helpers meta base
    | merge {
      status: ($pod | fmt pod-phase)
      ready: $ready
      total: $total
      restarts: $restarts
      podIP: $pod.status.podIP?
    }
  )

  if $output == "compact" {
    return $base
  }

  let owner = ($pod | helpers meta owner)

  let containers = (
    $pod.spec.containers?
    | default []
    | each {|c|
      let cstat = ($cs | where name == $c.name | get -o 0 | default {})
      let state = (
        if ($cstat.state?.running? | is-not-empty) {
          "Running"
        } else if ($cstat.state?.terminated? | is-not-empty) {
          let r = $cstat.state.terminated.reason?
          if ($r | is-not-empty) { $r } else { "Terminated" }
        } else if ($cstat.state?.waiting? | is-not-empty) {
          let r = $cstat.state.waiting.reason?
          if ($r | is-not-empty) { $r } else { "Waiting" }
        } else {
          null
        }
      )

      {
        name: $c.name
        image: $c.image
        ready: ($cstat.ready? | default false)
        restarts: ($cstat.restartCount? | default 0)
        state: $state
        ...($c.resources? | helpers resources base)
      }
    }
  )

  $base | merge {
    qos: $pod.status.qosClass?
    owner: $owner
    node: $pod.spec.nodeName?
    nominatedNode: $pod.status.nominatedNodeName?
    containers: $containers
  }
}

# ----------------
#  podtemplates   
# ----------------

export def "podtemplates v1" [output?: string = compact] {
  let pt = $in

  let spec = $pt.template.spec?
  let meta = $pt.template.metadata?

  let containers = ($spec.containers? | default [])
  let images = ($containers | get image | default [])

  let base = (
    $pt
    | helpers meta base
    | merge {
      containers: ($containers | length)
      images: $images
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    labels: ($meta.labels? | default {})
    restartPolicy: ($spec.restartPolicy? | default "Always")
    serviceAccount: (
      $spec.serviceAccountName?
      | default $spec.serviceAccount?
    )

    nodeSelector: ($spec.nodeSelector? | default {})
    owner: ($pt | helpers meta owner)

    containersSpec: (
      $containers
      | each {|c| $c | helpers container base }
    )
  }
}

# -----------
#  secrets   
# -----------

export def "secrets v1" [output?: string = compact] {
  let s = $in

  let data_count = ($s.data? | default {} | columns | length)
  let string_count = ($s.stringData? | default {} | columns | length)

  let base = (
    $s
    | helpers meta base
    | merge {
      type: ($s.type? | default "Opaque")
      data: $data_count
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    stringData: $string_count
    totalEntries: ($data_count + $string_count)
    immutable: ($s.immutable? | default false)
    owner: ($s | helpers meta owner)

    keys: ($s.data? | default {} | columns)
    stringKeys: ($s.stringData? | default {} | columns)
  }
}

# -------------------
#  serviceaccounts   
# -------------------

export def "serviceaccounts v1" [output?: string = compact] {
  let sa = $in

  # Base metadata: name, creation timestamp
  let base = (
    $sa
    | helpers meta base
    | merge {
      secrets: ($sa.secrets? | default [] | length)
      imagePullSecrets: ($sa.imagePullSecrets? | default [] | length)
    }
  )

  if $output == "compact" {
    return $base
  }

  # Wide output with more details
  $base | merge {
    owner: ($sa | helpers meta owner)
    secretsList: ($sa.secrets? | default [] | get name)
    imagePullSecretsList: ($sa.imagePullSecrets? | default [] | get name)
    automountServiceAccountToken: ($sa.automountServiceAccountToken? | default true)
  }
}

# ------------
#  services   
# ------------

export def "services v1" [output?: string = compact] {
  let svc = $in

  # Helper: summarize ports
  def "svc ports" [] {
    $in.spec.ports? 
    | default [] 
    | each {|p| {
      name: $p.name,
      protocol: ($p.protocol? | default "TCP"),
      port: $p.port,
      targetPort: $p.targetPort?,
      nodePort: $p.nodePort?
    } }
  }

  # Base metadata
  let base = (
    $svc
    | helpers meta base
    | merge {
      type: ($svc.spec.type? | default "ClusterIP"),
      clusterIP: ($svc.spec.clusterIP? | default ""),
      ports: ($svc | svc ports | length)
    }
  )

  if $output == "compact" {
    return $base
  }

  # Wide output
  $base | merge {
    owner: ($svc | helpers meta owner)
    selector: ($svc | helpers spec selector)
    sessionAffinity: ($svc.spec.sessionAffinity? | default "None")
    externalIPs: ($svc.spec.externalIPs? | default [])
    loadBalancerIP: ($svc.spec.loadBalancerIP? | default "")
    loadBalancerIngress: ($svc.status.loadBalancer.ingress? | default [])

    portsSpec: ($svc | svc ports)
  }
}

# ---------------
#  limitranges   
# ---------------

export def "limitranges v1" [output?: string = compact] {
  let lr = $in

  let limits = (
    $lr.spec.limits?
    | default []
    | each {|l|

      let cpu = {
        min: ($l.min?.cpu? | helpers cvt-cpu)
        max: ($l.max?.cpu? | helpers cvt-cpu)
        default: ($l.default?.cpu? | helpers cvt-cpu)
        defaultRequest: ($l.defaultRequest?.cpu? | helpers cvt-cpu)
      }

      let memory = {
        min: ($l.min?.memory? | helpers cvt-filesize)
        max: ($l.max?.memory? | helpers cvt-filesize)
        default: ($l.default?.memory? | helpers cvt-filesize)
        defaultRequest: ($l.defaultRequest?.memory? | helpers cvt-filesize)
      }

      {
        type: $l.type
        cpu: $cpu
        memory: $memory
      }
    }
  )

  let base = (
    $lr
    | helpers meta base
    | merge {
      types: ($limits | get type)
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    owner: ($lr | helpers meta owner)
    limits: $limits
  }
}

# ----------------
#  resourcequotas
# ----------------

export def "resourcequotas v1" [output?: string = compact] {
  let rq = $in

  let hard = ($rq.status.hard? | default {})
  let used = ($rq.status.used? | default {})

  let resources = (
    $hard
    | columns
    | each {|k|

      let h = ($hard | get $k)
      let u = ($used | get -o $k)

      if ($k | str contains "cpu") {
        {
          resource: $k
          used: ($u | helpers cvt-cpu)
          hard: ($h | helpers cvt-cpu)
        }
      } else if ($k | str contains "memory") or ($k | str contains "storage") {
        {
          resource: $k
          used: ($u | helpers cvt-filesize)
          hard: ($h | helpers cvt-filesize)
        }
      } else {
        {
          resource: $k
          used: $u
          hard: $h
        }
      }
    }
  )

  let base = (
    $rq
    | helpers meta base
    | merge {
      resources: ($resources | length)
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    owner: ($rq | helpers meta owner)
    quotas: $resources
  }
}

# ----------
# bindings
# ----------

export def "bindings v1" [output?: string = compact] {
  let b = $in

  let target = (
    if ($b.target? | is-empty) {
      null
    } else {
      $"($b.target.kind | str downcase)/($b.target.name)"
    }
  )

  let base = (
    $b
    | helpers meta base
    | merge {
      target: $target
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    targetRef: $b.target?
  }
}

# -------------------
# componentstatuses
# -------------------

export def "componentstatuses v1" [output?: string = compact] {
  let cs = $in

  let cond = ($cs | helpers status condition "Healthy")

  let base = (
    $cs
    | helpers meta base
    | merge {
      status: (
        if ($cond.status? == "True") {
          "Healthy"
        } else if ($cond.status? == "False") {
          "Unhealthy"
        } else {
          "Unknown"
        }
      )
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    message: $cond.message?
    error: $cond.error?
  }
}

# ----------
# endpoints
# ----------

export def "endpoints v1" [output?: string = compact] {
  let ep = $in

  let subsets = ($ep.subsets? | default [])

  let ready = (
    $subsets
    | each {|s| $s.addresses? | default [] | length }
    | if (($in | length) == 0) { 0 } else { math sum }
  )
  let notready = (
    $subsets
    | each {|s| $s.notReadyAddresses? | default [] | length }
    | if (($in | length) == 0) { 0 } else { math sum }
  )

  let base = (
    $ep
    | helpers meta base
    | merge {
      ready: $ready
      notReady: $notready
    }
  )

  if $output == "compact" {
    return $base
  }

  let addresses = (
    $subsets
    | each {|s|
      $s.addresses?
      | default []
      | each {|a|
        {
          ip: $a.ip
          node: $a.nodeName?
          target: (
            if ($a.targetRef? | is-empty) {
              null
            } else {
              $"($a.targetRef.kind | str downcase)/($a.targetRef.name)"
            }
          )
        }
      }
    }
    | flatten
  )

  $base | merge {
    owner: ($ep | helpers meta owner)
    addresses: $addresses
  }
}

# ------------------------
# persistentvolumeclaims
# ------------------------

export def "persistentvolumeclaims v1" [output?: string = compact] {
  let pvc = $in

  let capacity = (
    $pvc.status.capacity.storage?
    | helpers cvt-filesize
  )

  let req = (
    $pvc.spec.resources.requests.storage?
    | helpers cvt-filesize
  )

  let base = (
    $pvc
    | helpers meta base
    | merge {
      status: ($pvc.status.phase? | default "Unknown")
      volume: $pvc.spec.volumeName?
      capacity: $capacity
      requested: $req
      accessModes: ($pvc.spec.accessModes? | default [])
      storageClass: $pvc.spec.storageClassName?
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    owner: ($pvc | helpers meta owner)
    volumeMode: ($pvc.spec.volumeMode? | default "Filesystem")
    selector: ($pvc.spec.selector? | default {})
  }
}

# -------------------
# persistentvolumes
# -------------------

export def "persistentvolumes v1" [output?: string = compact] {
  let pv = $in

  let cap = (
    $pv.spec.capacity.storage?
    | helpers cvt-filesize
  )

  let claim = (
    if ($pv.spec.claimRef? | is-empty) {
      null
    } else {
      $"($pv.spec.claimRef.namespace)/($pv.spec.claimRef.name)"
    }
  )

  let base = (
    $pv
    | helpers meta base
    | merge {
      capacity: $cap
      accessModes: ($pv.spec.accessModes? | default [])
      reclaimPolicy: ($pv.spec.persistentVolumeReclaimPolicy? | default "Retain")
      status: ($pv.status.phase? | default "Unknown")
      claim: $claim
      storageClass: $pv.spec.storageClassName?
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    volumeMode: ($pv.spec.volumeMode? | default "Filesystem")
    nodeAffinity: $pv.spec.nodeAffinity?
    owner: ($pv | helpers meta owner)
  }
}

# ------------------------
# replicationcontrollers
# ------------------------

export def "replicationcontrollers v1" [output?: string = compact] {
  let rc = $in

  let replicas = {
    desired: ($rc.spec.replicas? | default 1)
    current: ($rc.status.replicas? | default 0)
    ready: ($rc.status.readyReplicas? | default 0)
    available: ($rc.status.availableReplicas? | default 0)
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
    $rc
    | helpers meta base
    | merge {
      status: $status
      ...$replicas
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    selector: ($rc.spec.selector? | default {})
    containers: ($rc | helpers tpl containers)
    owner: ($rc | helpers meta owner)
  }
}
