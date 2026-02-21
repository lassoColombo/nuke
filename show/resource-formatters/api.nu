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

  let ts = (
    $e.eventTime?
    | default $e.lastTimestamp?
    | default $e.firstTimestamp?
    | default $e.metadata.creationTimestamp?
    | helpers fmt-time
  )

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
        time: $ts
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

# ---------------
#  limitranges   
# ---------------

def fmtresources [] {
  $in 
  | transpose kind amount
  | each {|l|
    $l | update amount ($l.amount | into filesize)} 
  | reduce --fold {} {|elt acc| 
    $acc | merge {$elt.kind: $elt.amount}
  }
}

export def "limitranges v1" [output?: string = compact] {
  let lr = $in

  let limits = ($lr.spec.limits? | default [])

  let types = (
    $limits
    | get type
    | uniq
  )

  let resources = (
    $limits
    | each {|l|
      [
        ($l.min? | default {} | columns)
        ($l.max? | default {} | columns)
        ($l.default? | default {} | columns)
        ($l.defaultRequest? | default {} | columns)
      ]
      | flatten
    }
    | flatten
    | uniq
  )

  let res = {
    name: $lr.metadata.name
    namespace: $lr.metadata.namespace?
    types: $types
    resources: $resources
    age: ($lr.metadata.creationTimestamp? | helpers fmt-age)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert limits (
    $limits
    | each {|limit|
      {
        type: $limit.type
        min: ($limit.min? | fmtresources)
        max: ($limit.max? | fmtresources)
        default: ($limit.default? | fmtresources)
        defaultRequest: ($limit.defaultRequest? | fmtresources)
      }
    }
  )
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

      {
        name: $c.name
        image: $c.image
        ready: ($cstat.ready? | default false)
        restarts: ($cstat.restartCount? | default 0)

        state: (
          if ($cstat.state?.running? != null) {
            "Running"
          } else if ($cstat.state?.waiting? != null) {
            $cstat.state.waiting.reason? | default "Waiting"
          } else if ($cstat.state?.terminated? != null) {
            {
              terminated: {
                reason: $cstat.state.terminated.reason?
                exitCode: $cstat.state.terminated.exitCode?
              }
            }
          } else {
            null
          }
        )

        ...($c.resources? | helpers resources base)
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

# ------------------
#  resourcequotas   
# ------------------

export def "resourcequotas v1" [output?: string = compact] {
  let rq = $in

  let hard = ($rq.status?.hard? | default $rq.spec?.hard? | default {})
  let used = ($rq.status?.used? | default {})

  let resources = ($hard | columns)

  let usage = (
    $resources
    | each {|r|
      let h = ($hard | get $r)
      let u = ($used | get $r | default 0)

      {
        resource: $r
        hard: $h
        used: $u
      }
    }
  )

  let res = {
    name: $rq.metadata.name
    namespace: $rq.metadata.namespace?
    resources: ($resources | length)
    age: ($rq.metadata.creationTimestamp? | helpers fmt-age)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert quotas (
    $usage
    | each {|u|
      let pct = (
        try {
          ($u.used | into float) / ($u.hard | into float) * 100
        } catch {
          null
        }
      )
      {
        resource: $u.resource
        used: $u.used
        hard: $u.hard
        percent: $pct
      }
    }
  )
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
