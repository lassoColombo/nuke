use "../helpers.nu"


export def "configmaps v1" [output?: string = compact] {
  let cm = $in
  {
    name: $cm.metadata.name
    data: ($cm.data? | default {} | transpose key value | length)
    age: ($cm.metadata.creationTimestamp? | helpers fmtage)
  }
}

export def "events v1" [
  output?: string = compact
] {
  let ev = $in
  let mode = ($output | default compact)

  let ts = (
    $ev.lastTimestamp?
    | default $ev.eventTime?
    | default $ev.metadata.creationTimestamp?
    | into datetime
  )

  let obj_kind = ($ev.involvedObject.kind? | str downcase)
  let obj_name = ($ev.involvedObject.name?)

  let object = (
    if ($obj_kind | is-empty) or ($obj_name | is-empty) {
      null
    } else {
      $"($obj_kind)/($obj_name)"
    }
  )

  let message = ($ev.message? | default "")

  let base = {
    time: $ts
    type: ($ev.type? | default "Normal")
    reason: ($ev.reason? | default "")
    object: $object
    message: ($message | str substring 0..120)
  }

  if $mode == compact {
    $base
  } else {
    $base
    | insert namespace ($ev.involvedObject.namespace?)
    | insert count ($ev.count? | default 1)
    | insert source (
      $ev.source.component?
      | default $ev.reportingController?
    )
    | upsert message $message
    | insert firstSeen ($ev.firstTimestamp? | into datetime)
    | insert lastSeen $ts
    | insert raw $ev
  }
}

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
    age: ($lr.metadata.creationTimestamp? | helpers fmtage)
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

export def "namespaces v1" [output?: string = compact] {
  let ns = $in
  {
    name: $ns.metadata.name
    status: $ns.status.phase?
    age: ($ns.metadata.creationTimestamp? | helpers fmtage)
  }
}

export def "nodes v1" [output?: string = compact] {
  let no = $in
  let directroles = ($no.metadata.labels | transpose key value | where key == kubernetes.io/role | get value)
  let indirectroles = ($no.metadata.labels | transpose key value | where {$in.key | str starts-with node-role.kubernetes.io/} | get key | each { $in | split row / | last })
  let roles = $directroles | append $indirectroles

  let res = {
    name: $no.metadata.name
    roles: $roles
    status: ($no.status.conditions?.type? | default [null] | last)
    age: ($no.metadata.creationTimestamp? | helpers fmtage)
    version: $no.status.nodeInfo.kubeletVersion?
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | insert kernel $no.status.nodeInfo.kernelVersion?
  | insert image $no.status.nodeInfo.osImage?
  | insert internalIPs ($no.status.addresses? | default [] | where type == InternalIP | get address)
  | insert externalIPs ($no.status.addresses? | default [] | where type == ExternalIP | get address)
}

export def "pods v1" [output?: string = compact] {
  let pod = $in
  let cs = ($pod.status.containerStatuses? | default [])

  let waiting = (
    $cs
    | where state?.waiting? != null
    | get state.waiting
  )

  let terminated = (
    $cs
    | where state?.terminated? != null
    | get state.terminated
  )

  let ready_count = ($cs | where ready == true | length)
  let total_count = ($pod.spec.containers | length)

  let ready_cond = (
    $pod.status.conditions?
    | default []
    | where type == "Ready"
    | first
    | default {}
  )

  let status = (
    if ($waiting | is-not-empty) {
      $waiting | first | get -o reason
    } else if ($terminated | is-not-empty) {
      $terminated | first | get -o reason
    } else if ($ready_cond.status? == "False") {
      "NotReady"
    } else {
      $pod.status.phase
    }
  )

  let res = {
    name: $pod.metadata.name
    status: $status
    ready: $ready_count
    total: $total_count
    restarts: (
      $cs | reduce --fold 0 {|c acc| $acc + ($c.restartCount? | default 0)}
    )
    age: ($pod.metadata.creationTimestamp? | helpers fmtage)
    podIP: $pod.status.podIP?
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  let owner = (
    $pod.metadata.ownerReferences?
    | default []
    | where controller == true
    | if (($in | length) != 0) {$in} else {[{}]}
    | first
  )
  let owner = if ($owner | is-empty) { null } else { $"($owner.kind | str downcase)/($owner.name)" }

  let containers = (
    $pod.spec.containers
    | each {|c|
      let cstat = ($cs | where name == $c.name | first | default {})
      {
        name: $c.name
        image: $c.image
        command: $c.command?
        args: $c.args?
        ready: $cstat.ready?
        restarts: $cstat.restartCount?
        state: (
          if ($cstat.state?.running? != null) {
            "running"
          } else if ($pod.status.phase == "Running" and $ready_count < $total_count) {
            "NotReady"
          } else if ($cstat.state?.waiting? != null) {
            {
              waiting: {
                reason: $cstat.state.waiting.reason?
                message: $cstat.state.waiting.message?
              }
            }
          } else if ($cstat.state?.terminated? != null) {
            {
              terminated: {
                reason: $cstat.state.terminated.reason?
                message: $cstat.state.terminated.message?
                exitCode: $cstat.state.terminated.exitCode?
              }
            }
          } else {
            null
          }
        )
        ...($c.resources? | helpers fmtresources)
      }
    }
  )
  $res | merge {
    qos: $pod.status.qosClass?
    owner: $owner
    containers: $containers
    node: $pod.spec.nodeName?
  }
}

export def "podtemplates v1" [output?: string = compact] {
  let pt = $in
  {
    name: $pt.metadata.name
    containers: ( $pt.template.spec.containers | select -o name image )
    pod-labels: ( $pt.template.metadata.labels?)
    restart-policy: ( $pt.template.spec.restartPolicy )
  }
}

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
    age: ($rq.metadata.creationTimestamp? | helpers fmtage)
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

export def "secrets v1" [output?: string = compact] {
  let sec = $in
  {
    name: $sec.metadata.name
    data: ($sec.data? | default {} | transpose key value | length)
    age: ($sec.metadata.creationTimestamp? | helpers fmtage)
  }
}

export def "serviceaccounts v1" [output?: string = compact] {
  let sa = $in
  {
    name: $sa.metadata.name
    secrets: ($sa.secrets? | default [] | get -o name)
    age: ($sa.metadata.creationTimestamp? | helpers fmtage)
  }
}

export def "services v1" [output?: string = compact] {
  let svc = $in
  {
    name: $svc.metadata.name
    type: $svc.spec.type
    clusterIP: $svc.spec.clusterIP?
    age: ($svc.metadata.creationTimestamp? | helpers fmtage)
    ports: ($svc.spec.ports? | default [] | select -o protocol port targetPort)
    selector: ($svc.spec.selector? | default {} | transpose key value)
  }
}
