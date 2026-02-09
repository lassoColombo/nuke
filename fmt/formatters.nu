export def main [] {
  {
    apiservice: {| output?: string = compact |
      let as = $in

      let available_cond = ($as.status.conditions? | default [] | where type == Available | first)
      let service = if ($as.spec.service? | is-empty) { {} } else {
        ($as.spec.service? | default {} | select -o namespace name)
      }

      let res = {
        name: $as.metadata.name
        service: $service
        available: ($available_cond.status?)
        age: ($as.metadata.creationTimestamp? | helpers fmtage)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        # | upsert reason ($available_cond.reason?)
        | upsert message ($available_cond.message?)
        | upsert groupPriority ($as.spec.groupPriorityMinimum?)
        | upsert versionPriority ($as.spec.versionPriority?)
        | upsert caBundle ($as.spec.caBundle? | is-not-empty)
      }
    }
    clusterrole: {| output?: string = compact |
      let r = $in
      let res = {
        name: $r.metadata.name
        created: $r.metadata.creationTimestamp
        rules: ($r.rules? | default [] | length)
      }
      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | upsert aggregationRule ($r.aggregationRule?.clusterRoleSelectors?)
        | update rules ($r.rules)
      }
    }
    clusterrolebinding: {| output?: string = compact |
      let r = $in
      let subjects = ($r.subjects? | default [])
      let users = ($subjects | where kind == User | get name | default [])
      let groups = ($subjects | where kind == Group | get name | default [])
      let serviceaccounts = ($subjects | where kind == ServiceAccount | each {|s| 
        $'($s.namespace | default '')/($s.name)'
      } | default [])

      let res = {
        name: $r.metadata.name
        role: ($r.roleRef.name)
        users: ($users | length)
        groups: ($groups | length)
        serviceaccounts: ($serviceaccounts | length)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | upsert subjects $subjects
        | update role ($r.roleRef | select -o kind name)
        | update users $users
        | update groups $groups
        | update serviceaccounts $serviceaccounts
        | insert created $r.metadata.creationTimestamp
      }
    }
    configmap: {| output?: string = compact|
      let cm = $in
      {
        name: $cm.metadata.name
        data: ($cm.data? | default {} | transpose key value | length)
        age: ($cm.metadata.creationTimestamp? | helpers fmtage)
      }
    }
    controllerrevision: {| output?: string = compact |
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
        $res
      } else {
        $res
        | upsert namespace ($cr.metadata.namespace?)
        | upsert controller_uid ($owner.uid?)
        | upsert generation ($cr.metadata.annotations.'deprecated.daemonset.template.generation'?)
        | upsert labels ($cr.metadata.labels?)
        | upsert container_names ($cr.data.spec.template.spec.containers? | get name)
        | upsert container_images ($cr.data.spec.template.spec.containers? | get image)
      }
    }
    cronjob: {| output?: string = compact|
      let cj = $in
      let res = {
        name: $cj.metadata.name
        schedule: $cj.spec.schedule
        timezone: $cj.spec.timezone?
        suspend: ($cj.status.suspend? | default false)
        last-schedule: ((date now) - ($cj.status.lastScheduleTime | into datetime))
        last-success: ((date now) - ($cj.status.lastSuccessfulTime | into datetime))
        active: ($cj.status.active? | default [] | length)
        age: ($cj.metadata.creationTimestamp? | helpers fmtage)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | upsert containers ($cj | helpers fmtcontainers)
        | upsert selector ($cj.spec.selector?.matchLabels?)
      }
    }
    customresourcedefinition: {| output?: string = compact |
      let crd = $in

      let versions = ($crd.spec.versions? | default [])
      let storage_version = (
        $versions
        | where storage == true
        | get name
        | first
      )

      let established_cond = (
        $crd.status.conditions?
        | default []
        | where type == Established
        | first
        | default {}
      )

      let res = {
        name: $crd.metadata.name
        group: $crd.spec.group
        kind: $crd.spec.names.kind
        scope: $crd.spec.scope
        version: $storage_version
        established: ($established_cond.status?)
        age: ($crd.metadata.creationTimestamp? | helpers fmtage)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | upsert versions (
          $versions
          | select -o name served storage
        )
        | upsert names (
          $crd.spec.names
          | select -o plural singular kind listKind
        )
        | upsert printerColumns (
          $versions
          | where name == $storage_version
          | get additionalPrinterColumns?
          | first
        )
        | upsert conditions ($crd.status.conditions?)
      }
    }
    daemonset: {| output?: string = compact|
      let daem = $in
      let status = $daem.status?
      | default {}
      | reject observedGeneration numberMisscheduled
      | insert current ($in.currentNumberScheduled? | default 0)
      | insert desired ($in.desiredNumberScheduled? | default 0)
      | insert ready ($in.numberReady? | default 0)
      | insert up-to-date ($in.updatedNumberScheduled? | default 0)
      | insert available ($in.numberAvailable? | default 0)
      | reject -o currentNumberScheduled desiredNumberScheduled numberReady updatedNumberScheduled numberAvailable

      let res = {
        name: $daem.metadata.name
        age: ($daem.metadata.creationTimestamp? | helpers fmtage)
        selector: ($daem | helpers fmtselector)
      } | merge $status

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res | insert containers ($daem | helpers fmtcontainers)
      }
    }
    deployment: {| output?: string = compact|
      let deploy = $in
      let res = {
        name: $deploy.metadata.name
        status: ($deploy.status.conditions?.type? | default [null] | first)
        replicas: ($deploy.status.replicas? | default 0)
        ready: ($deploy.status.readyReplicas? | default 0)
        available: ($deploy.status.availableReplicas? | default 0)
        updated: ($deploy.status.updatedReplicas? | default 0)
        age: ($deploy.metadata.creationTimestamp? | helpers fmtage)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | insert selector ($deploy | helpers fmtselector)
        | insert containers ($deploy | helpers fmtcontainers)
      }
    }
    endpointslice: {|output?: string = compact|
      let ep = $in
      {
        name: $ep.metadata.name
        address-type: $ep.addressType
        ports: $ep.ports
        endpoints: ($ep.endpoints? | default [] | get addresses | flatten)
        age: ($ep.metadata.creationTimestamp? | helpers fmtage)
      }
    }
    event: {|output?: string = compact|
      let ev = $in
      let res = {
        last-seen: $ev.lastTimestamp
        type: $ev.type
        reason: $ev.reason
        object: $'($ev.involvedObject | get kind | str downcase)/($ev.involvedObject | get name)'
        message: $ev.message
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | insert source ($ev.source.component)
        | insert first-seen ($ev.firstTimestamp)
        | insert count ($ev.count)
        | insert name ($ev.metadata.name)
      }
    }
    ingress: {| output?: string = compact |
      let ing = $in

      let rules = ($ing.spec.rules? | default [])

      let paths = (
        $rules
        | each {|r|
          $r.http?.paths? | default []
        }
        | flatten
      )

      let backends = (
        $paths
        | get backend.service?
        | where $it != null
      )

      let hosts = (
        $rules
        | get host?
        | where $it != null
        | uniq
      )

      let lb = ($ing.status.loadBalancer.ingress? | default [])

      let res = {
        name: $ing.metadata.name
        class: $ing.spec.ingressClassName?
        hosts: ($hosts | length)
        paths: ($paths | length)
        backends: ($backends | length)
        age: ($ing.metadata.creationTimestamp? | helpers fmtage)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | upsert namespace ($ing.metadata.namespace?)
        | upsert generation ($ing.metadata.generation?)
        | upsert loadBalancer (
          $lb
          | each {|i|
            {
              ip: $i.ip?
              hostname: $i.hostname?
            }
          }
        )
        | upsert rules (
          $rules
          | each {|r|
            {
              host: ($r.host? | default "*")
              paths: (
                $r.http?.paths?
                | default []
                | each {|p|
                  {
                    path: $p.path?
                    pathType: $p.pathType?
                    service: $p.backend.service.name?
                    port: (
                      $p.backend.service.port.number?
                      | default $p.backend.service.port.name?
                    )
                  }
                }
              )
            }
          }
        )
      }
    }
    ingressclass: {| output?: string = compact |
      let ic = $in
      let params = ($ic.spec.parameters? | default {})

      let res = {
        name: $ic.metadata.name
        controller: $ic.spec.controller?
        parameters: (
          if ($params | is-empty) {
            null
          } else {
            $params | select -o kind name
          }
        )
        scope: ($params.scope? | default "Cluster")
        age: ($ic.metadata.creationTimestamp? | helpers fmtage)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | upsert apiGroup ($params.apiGroup?)
      }
    }
    ipaddress: {| output?: string = compact |
      let ip = $in

      let parent = $ip.spec.parentRef?

      let res = {
        name: $ip.metadata.name
        parent: ($parent | default {} | select -o resource namespace name)
        ipFamily: ($ip.metadata.labels.'ipaddress.kubernetes.io/ip-family'?)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | upsert managedBy ($ip.metadata.labels.'ipaddress.kubernetes.io/managed-by'? | default <none>)
        | upsert age ($ip.metadata.creationTimestamp? | helpers fmtage)
      }
    }
    job: {| output?: string = compact|
      let j = $in
      let res = {
        name: $j.metadata.name
        status: ($j.status.conditions? | default [{type: Running}] | last | get type)
        completed: ($j.status.succeeded? | default 0)
        completions: $j.spec.completions
        duration: ((($j.status.endTime? | default {date now} | into datetime) - ($j.status.startTime | into datetime)))
        age: ($j.metadata.creationTimestamp? | helpers fmtage)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | upsert containers ($j | helpers fmtcontainers)
        | upsert selector ($j.spec.selector?.matchLabels?)
      }
    }
    limitrange: {| output?: string = compact |
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
        $res
      } else {
        $res
        | upsert limits (
          $limits
          | each {|limit|
            {
              type: $limit.type
              min: ($limit.min? | helpers fmtresources)
              max: ($limit.max? | helpers fmtresources)
              default: ($limit.default? | helpers fmtresources)
              defaultRequest: ($limit.defaultRequest? | helpers fmtresources)
            }
          }
        )
      }
    }
    namespace: {|output?: string = compact|
      let ns = $in
      {
        name: $ns.metadata.name
        status: $ns.status.phase?
        age: ($ns.metadata.creationTimestamp? | helpers fmtage)
      }
    }
    networkpolicy: {| output?: string = compact|
      let np = $in
      {
        name: $np.metadata.name
        selector: $np.spec.podSelector.matchLabels
        ingress: ($np.spec.ingress? | default [] | length)
        egress: ($np.spec.egress? | default [] | length)
        age: ($np.metadata.creationTimestamp? | helpers fmtage)
      }
    }
    node: {| output?: string = compact|
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
        $res
      } else {
        $res
        | insert kernel $no.status.nodeInfo.kernelVersion?
        | insert image $no.status.nodeInfo.osImage?
        | insert internalIPs ($no.status.addresses? | default [] | where type == InternalIP | get address)
        | insert externalIPs ($no.status.addresses? | default [] | where type == ExternalIP | get address)
      }
    }
    persistentvolume: {|output?: string = compact|
      let pv = $in
      let res = {
        name: $pv.metadata.name
        capacity: $pv.spec.capacity?.storage?
        accessModes: $pv.spec.accessModes?
        reclaimPolicy: $pv.spec.persistentVolumeReclaimPolicy?
        phase: $pv.status.phase?
        claim: ($pv.spec.claimRef? | if ($in | is-empty) {$in} else {$in | select -o namespace name})
        storageclass: $pv.spec.storageClassName?
        age: ($pv.metadata.creationTimestamp? | helpers fmtage)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res | insert volumeMode ($pv.spec.volumeMode)
      }
    }
    persistentvolumeclaim: {|output?: string = compact|
      let pvc = $in
      let res = {
        name: $pvc.metadata.name
        phase: $pvc.status.phase
        volume: $pvc.spec.volumeName?
        capacity: $pvc.status.capacity.storage
        accessModes: $pvc.spec.accessModes
        storageclass: $pvc.spec.storageClassName?
        age: ($pvc.metadata.creationTimestamp? | helpers fmtage)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res | insert volumeMode ($pvc.spec.volumeMode)
      }
    }
    pod: {|output?: string = compact|
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
        ready: $"($ready_count)/($total_count)"
        restarts: (
          $cs | reduce --fold 0 {|c acc| $acc + ($c.restartCount? | default 0)}
        )
        age: ($pod.metadata.creationTimestamp? | helpers fmtage)
        podIP: $pod.status.podIP?
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        let owner = (
          $pod.metadata.ownerReferences?
          | default []
          | where controller == true
          | first
          | default {}
        )

        $res
        | upsert namespace $pod.metadata.namespace?
        | upsert node $pod.spec.nodeName?
        | upsert qos $pod.status.qosClass?
        | upsert owner (
          if ($owner | is-empty) {
            null
          } else {
            $"($owner.kind | str downcase)/($owner.name)"
          }
        )
        | upsert containers (
          $pod.spec.containers
          | each {|c|
            let cstat = ($cs | where name == $c.name | first | default {})
            {
              name: $c.name
              image: $c.image
              ready: $cstat.ready?
              restarts: $cstat.restartCount?
              state: (
                if ($cstat.state?.running? != null) {
                  "running"
                } else if ($cstat.state?.waiting? != null) {
                  { waiting: $cstat.state.waiting.message? }
                } else if ($cstat.state?.terminated? != null) {
                  {
                    terminated: {
                      reason: $cstat.state.terminated.message?
                      exitCode: $cstat.state.terminated.exitCode?
                    }
                  }
                } else {
                  null
                }
              )
            }
          }
        )
      }
    }
    podtemplate: {|output?: string = compact|
      let pt = $in
      {
        name: $pt.metadata.name
        containers: ( $pt.template.spec.containers | select -o name image )
        pod-labels: ( $pt.template.metadata.labels?)
        restart-policy: ( $pt.template.spec.restartPolicy )
      }
    }
    priorityclass: {|output?: string = compact|
      let pc = $in
      {
        name: $pc.metadata.name
        value: $pc.value?
        preemption-policy: ($pc.preemptionPolicy? | default false)
        global-default: ($pc.globalDefault? | default false)
        age: ($pc.metadata.creationTimestamp? | helpers fmtage)
      }
    }
    replicaset: {| output?: string = compact|
      let rs = $in
      let status = $rs.status?
      | default {}
      | reject -o observedGeneration fullyLabeledReplicas
      | insert ready ($in.readyReplicas? | default 0)
      | insert available ($in.availableReplicas? | default 0)
      | reject -o readyReplicas availableReplicas

      let res = {
        name: $rs.metadata.name
        age: ($rs.metadata.creationTimestamp? | helpers fmtage)
      } | merge $status

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | insert selector ($rs | helpers fmtselector)
        | insert containers ($rs | helpers fmtcontainers)
      }
    }
    resourcequota: {| output?: string = compact |
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
        $res
      } else {
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
    }
    role: {| output?: string = compact |
      let r = $in
      let res = {
        name: $r.metadata.name
        created: $r.metadata.creationTimestamp
        rules: ($r.rules? | default [] | length)
      }
      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | update rules ($r.rules)
      }
    }
    rolebinding: {| output?: string = compact |
      let r = $in
      let subjects = ($r.subjects? | default [])
      let users = ($subjects | where kind == User | get name | default [])
      let groups = ($subjects | where kind == Group | get name | default [])
      let serviceaccounts = ($subjects | where kind == ServiceAccount | each {|s| 
        $'($s.namespace | default $r.metadata.namespace)/($s.name)'
      } | default [])

      let res = {
        name: $r.metadata.name
        namespace: ($r.metadata.namespace?)
        role: ($r.roleRef.name)
        users: ($users | length)
        groups: ($groups | length)
        serviceaccounts: ($serviceaccounts | length)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | upsert subjects $subjects
        | update role ($r.roleRef | select -o kind name)
        | update users $users
        | update groups $groups
        | update serviceaccounts $serviceaccounts
        | insert created $r.metadata.creationTimestamp
      }
    }
    secret: {| output?: string = compact|
      let sec = $in
      {
        name: $sec.metadata.name
        data: ($sec.data? | default {} | transpose key value | length)
        age: ($sec.metadata.creationTimestamp? | helpers fmtage)
      }
    }
    service: {| output?: string = compact|
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
    serviceaccount: {|output?: string = compact|
      let sa = $in
      {
        name: $sa.metadata.name
        secrets: ($sa.secrets? | default [] | get -o name)
        age: ($sa.metadata.creationTimestamp? | helpers fmtage)
      }
    }
    servicecidr: {| output?: string = compact |
      let sc = $in

      # find Ready condition, if present
      let ready_cond = ($sc.status.conditions? | where type == Ready | first | default {})

      let res = {
        name: $sc.metadata.name
        cidrs: ($sc.spec.cidrs? | default [])
        ready: ($ready_cond.status?)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | upsert message ($ready_cond.message?)
        | upsert reason ($ready_cond.reason?)
        | upsert finalizers ($sc.metadata.finalizers?)
        | upsert age ($sc.metadata.creationTimestamp?)
      }
    }
    statefulset: {| output?: string = compact|
      let sts = $in
      let res = {
        name: $sts.metadata.name
        replicas: ($sts.status.replicas? | default 0)
        ready: ($sts.status.readyReplicas? | default 0)
        updated: ($sts.status.updatedReplicas? | default 0)
        available: ($sts.status.availableReplicas? | default 0)
        age: ($sts.metadata.creationTimestamp? | helpers fmtage)
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res | insert containers ($sts | helpers fmtcontainers)
      }
    }
  }
}
