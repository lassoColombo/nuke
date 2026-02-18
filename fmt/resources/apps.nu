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

export def "daemonsets v1" [output: string = compact] {
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
    return $res
  } 
  $res | insert containers ($daem | helpers fmtcontainers)
}

export def "deployments v1" [output: string = compact] {
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
    return $res
  } 
  $res
  | insert selector ($deploy | helpers fmtselector)
  | insert containers ($deploy | helpers fmtcontainers)
}

export def "replicasets v1" [output?: string = compact] {
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
    return $res
  } 
  $res
  | insert selector ($rs | helpers fmtselector)
  | insert containers ($rs | helpers fmtcontainers)
}

export def "statefulsets v1" [output?: string = compact] {
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
    return $res
  } 
  $res | insert containers ($sts | helpers fmtcontainers)
}
