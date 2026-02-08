export def main [] {
  {
    pod: {|output?: string = compact|
      let pod = $in
      let res = {
        name: $pod.metadata.name
        status: (
          if ($pod.status.containerStatuses?.state?.waiting?.reason? | default [] | where {$in | is-not-empty} | is-not-empty) {
            $pod.status.containerStatuses.state.waiting.reason | first
          } else if ($pod.status.containerStatuses?.state?.terminated?.reason? | default [] | where {$in | is-not-empty} | is-not-empty) {
            $pod.status.containerStatuses.state.terminated.reason | first
          } else {
            $pod.status.phase
          }
        )
        containers: ($pod.spec.containers | length)
        ready: ($pod.status.containerStatuses? | default [] | where {($in.ready? | default false) == true} | length)
        restarts: ($pod.status.containerStatuses? | default [] | reduce --fold 0 {|status acc| $acc + $status.restartCount?})
        uptime: ($pod.status.startTime? | helpers fmtage)
        podIP: $pod.status.podIP?
      }

      if ($output | is-empty) or $output == compact {
        $res
      } else {
        $res
        | upsert containers ($pod.spec.containers
          | select name image
          | insert status {|container|
            $pod.status.containerStatuses?
            | where name == $container.name
            | get state
            | transpose key value
            | first
            | if ($in.value.message? | is-not-empty) {
              {$in.key: $in.value.message?}
            } else {
              $in.key
            }
          }
        )
        | upsert node $pod.spec.nodeName?
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

    configmap: {| output?: string = compact|
      let cm = $in
      {
        name: $cm.metadata.name
        data: ($cm.data? | default {} | transpose key value | length)
        age: ($cm.metadata.creationTimestamp? | helpers fmtage)
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

    node: {| output?: string = compact|
      let no = $in
      let directroles = ($no.metadata.labels | transpose key value | where key == "kubernetes.io/role" | get value)
      let indirectroles = ($no.metadata.labels | transpose key value | where {$in.key | str starts-with "node-role.kubernetes.io/"} | get key | each { $in | split row / | last })
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
        | insert internalIPs ($no.status.addresses? | default [] | where type == "InternalIP" | get address)
        | insert externalIPs ($no.status.addresses? | default [] | where type == "ExternalIP" | get address)
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

    app: {|output?: string = compact|
      let app = $in
      {
        name: $app.metadata.name
        chart: $app.spec.chart.metadata.name
        version: $app.spec.chart.metadata.version
        release-name: $app.spec.name
        release-version: $app.spec.version
        status: $app.status.summary.state
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

    event: {|output?: string = compact|
      let ev = $in
      let res = {
        last-seen: $ev.lastTimestamp
        type: $ev.type
        reason: $ev.reason
        object: $"($ev.involvedObject | get kind | str downcase)/($ev.involvedObject | get name)"
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

    apiservice: {| output?: string = compact |
      let as = $in

      let available_cond = ($as.status.conditions? | default [] | where type == "Available" | first)
      let service = if ($as.spec.service? | is-empty) { {} } else {
        ($as.spec.service? | default {} | select -o namespace name)
      }

      let res = {
        name: $as.metadata.name
        service: $service
        available: ($available_cond.status?)
        age: ($as.metadata.creationTimestamp? | helpers fmtage)
      }

      if ($output | is-empty) or $output == "compact" {
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
          $"($owner.kind | str downcase).($owner.apiVersion)/($owner.name)"
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

      if ($output | is-empty) or $output == "compact" {
        $res
      } else {
        $res
        | upsert namespace ($cr.metadata.namespace?)
        | upsert controller_uid ($owner.uid?)
        | upsert generation ($cr.metadata.annotations."deprecated.daemonset.template.generation"?)
        | upsert labels ($cr.metadata.labels?)
        | upsert container_names ($cr.data.spec.template.spec.containers? | get name)
        | upsert container_images ($cr.data.spec.template.spec.containers? | get image)
      }
    }

    ipaddress: {| output?: string = compact |
      let ip = $in

      let parent = $ip.spec.parentRef?

      let res = {
        name: $ip.metadata.name
        parent: ($parent | default {} | select -o resource namespace name)
        ipFamily: ($ip.metadata.labels."ipaddress.kubernetes.io/ip-family"?)
      }

      if ($output | is-empty) or $output == "compact" {
        $res
      } else {
        $res
        | upsert managedBy ($ip.metadata.labels."ipaddress.kubernetes.io/managed-by"? | default "<none>")
        | upsert age ($ip.metadata.creationTimestamp? | helpers fmtage)
      }
    }

    servicecidr: {| output?: string = "compact" |
      let sc = $in

      # find Ready condition, if present
      let ready_cond = ($sc.status.conditions? | where type == "Ready" | first | default {})

      let res = {
        name: $sc.metadata.name
        cidrs: ($sc.spec.cidrs? | default [])
        ready: ($ready_cond.status?)
      }

      if ($output | is-empty) or $output == "compact" {
        $res
      } else {
        $res
        | upsert message ($ready_cond.message?)
        | upsert reason ($ready_cond.reason?)
        | upsert finalizers ($sc.metadata.finalizers?)
        | upsert age ($sc.metadata.creationTimestamp?)
      }
    }

    role: {| output?: string = "compact" |
      let r = $in
      let res = {
        name: $r.metadata.name
        created: $r.metadata.creationTimestamp
        rules: ($r.rules? | default [] | length)
      }
      if ($output | is-empty) or $output == "compact" {
        $res
      } else {
        $res
        | update rules ($r.rules)
      }
    }

    clusterrole: {| output?: string = "compact" |
      let r = $in
      let res = {
        name: $r.metadata.name
        created: $r.metadata.creationTimestamp
        rules: ($r.rules? | default [] | length)
      }
      if ($output | is-empty) or $output == "compact" {
        $res
      } else {
        $res
        | upsert aggregationRule ($r.aggregationRule?.clusterRoleSelectors?)
        | update rules ($r.rules)
      }
    }

    rolebinding: {| output?: string = "compact" |
      let r = $in
      let subjects = ($r.subjects? | default [])
      let users = ($subjects | where kind == "User" | get name | default [])
      let groups = ($subjects | where kind == "Group" | get name | default [])
      let serviceaccounts = ($subjects | where kind == "ServiceAccount" | each {|s| 
          $"($s.namespace | default $r.metadata.namespace)/($s.name)"
        } | default [])

      let res = {
        name: $r.metadata.name
        namespace: ($r.metadata.namespace?)
        role: ($r.roleRef.name)
        users: ($users | length)
        groups: ($groups | length)
        serviceaccounts: ($serviceaccounts | length)
      }

      if ($output | is-empty) or $output == "compact" {
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

    clusterrolebinding: {| output?: string = "compact" |
      let r = $in
      let subjects = ($r.subjects? | default [])
      let users = ($subjects | where kind == "User" | get name | default [])
      let groups = ($subjects | where kind == "Group" | get name | default [])
      let serviceaccounts = ($subjects | where kind == "ServiceAccount" | each {|s| 
          $"($s.namespace | default '')/($s.name)"
        } | default [])

      let res = {
        name: $r.metadata.name
        role: ($r.roleRef.name)
        users: ($users | length)
        groups: ($groups | length)
        serviceaccounts: ($serviceaccounts | length)
      }

      if ($output | is-empty) or $output == "compact" {
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

  }
}
