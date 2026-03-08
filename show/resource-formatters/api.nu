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

export def "nodes v1" [output?: string = compact] {
  let no = $in

  let base = (
    $no
    | helpers meta base
    | merge {
      roles: ($no | helpers node roles)
      status: ($no | helpers status node-ready)
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

export def "pods v1" [output?: string = compact] {
  let pod = $in
  let cs = ($pod | helpers status containers)

  let total = ($pod.spec.containers? | default [] | length)
  let ready = ($pod | helpers status ready-count)

  let base = (
    $pod
    | helpers meta base
    | merge {
      status: ($pod | helpers status pod-phase)
      ready: $ready
      total: $total
      restarts: ($pod | helpers status restart-sum)
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
      let cstat = ($cs | where name == $c.name | first | default {})
      let state = (
        if ($cstat.state?.running? != null) {
          "Running"
        } else if ($cstat.state?.terminated? != null) {
          "Terminated"
        } else if ($cstat.state?.waiting? != null) {
          $cstat.state.waiting.reason? | default "Waiting"
        } else {
          null
        }
      )
      let exitcode = if $state != "Terminated" {{}} else {
        exitCode: $cstat.state.terminated.exitCode?
      }

      {
        name: $c.name
        image: $c.image
        ready: ($cstat.ready? | default false)
        restarts: ($cstat.restartCount? | default 0)
        state: $state
        ...$exitcode
        # ...($c.resources? | helpers resources base)
      }
    }
  )

  $base | merge {
    qos: $pod.status.qosClass?
    owner: $owner
    node: $pod.spec.nodeName?
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
  def "helpers svc ports" [] {
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
      ports: ($svc | helpers svc ports | length)
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

    portsSpec: ($svc | helpers svc ports)
  }
}

# ---------------
#  limitranges   
# ---------------

# ----------------
#  limitranges
# ----------------

export def "limitranges v1" [output?: string = compact] {
  let lr = $in

  let limits = (
    $lr.spec.limits?
    | default []
    | each {|l|

      let cpu = {
        min: ($l.min?.cpu? | helpers res cpu-millicores)
        max: ($l.max?.cpu? | helpers res cpu-millicores)
        default: ($l.default?.cpu? | helpers res cpu-millicores)
        defaultRequest: ($l.defaultRequest?.cpu? | helpers res cpu-millicores)
      }

      let memory = {
        min: ($l.min?.memory? | helpers res memory-bytes)
        max: ($l.max?.memory? | helpers res memory-bytes)
        default: ($l.default?.memory? | helpers res memory-bytes)
        defaultRequest: ($l.defaultRequest?.memory? | helpers res memory-bytes)
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
          used: ($u | helpers res cpu-millicores)
          hard: ($h | helpers res cpu-millicores)
        }
      } else if ($k | str contains "memory") {
        {
          resource: $k
          used: ($u | helpers res memory-bytes)
          hard: ($h | helpers res memory-bytes)
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
