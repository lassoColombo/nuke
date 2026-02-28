export def "fmt-time" [] {
  if ($in | is-empty) { null } else { $in | into datetime }
}

export def "fmt-age" [] {
  let ts = ($in | fmt-time)
  if $ts == null { null } else { (date now) - $ts }
}

export def "meta base" [] {
  let m = $in.metadata
  {
    name: $m.name
    created: ($m.creationTimestamp? | fmt-time)
  }
}

export def "meta owner" [] {
  let ref = (
    $in.metadata.ownerReferences?
    | default []
    | where controller == true
    | first
  )

  if ($ref | is-empty) {
    null
  } else {
    $"($ref.kind | str downcase)/($ref.name)"
  }
}

export def "status replicas" [] {
  {
    desired: ($in.spec.replicas? | default 1)
    updated: ($in.status.updatedReplicas? | default 0)
    ready: ($in.status.readyReplicas? | default 0)
    available: ($in.status.availableReplicas? | default 0)
    unavailable: ($in.status.unavailableReplicas? | default 0)
  }
}

export def "status condition" [type: string] {
  (
    $in.status.conditions?
    | default []
    | where type == $type
    | first
    | default {}
  )
}

export def "status condition-of" [type: string] {
  $in | status condition $type
}

export def "spec selector" [] {
  $in.spec.selector? | default {}
}

export def "spec strategy" [] {
  let s = $in.spec.strategy? | default {}
  let rolling = ($s.rollingUpdate? | default {})

  {
    type: ($s.type? | default "RollingUpdate")
    maxUnavailable: $rolling.maxUnavailable?
    maxSurge: $rolling.maxSurge?
  }
}

export def "resources base" [] {
  let r = ($in | default {})

  {
    requests: ($r.requests? | default {})
    limits: ($r.limits? | default {})
  }
}

export def "container base" [] {
  {
    name: $in.name
    image: $in.image
    ...($in.resources? | resources base)
  }
}

export def "tpl containers" [] {
  $in.spec.template.spec.containers?
  | default []
  | each {|c| $c | container base }
}

export def "tpl images" [] {
  $in.spec.template.spec.containers?
  | default []
  | get image
}

export def "rbac subjects" [] {
  let subs = ($in | default [])

  {
    users: ($subs | where kind == User | get name | default [])
    groups: ($subs | where kind == Group | get name | default [])
    serviceaccounts: (
      $subs
      | where kind == ServiceAccount
      | each {|s| $"($s.namespace | default '')/($s.name)" }
      | default []
    )
  }
}

export def "status containers" [] {
  $in.status.containerStatuses? | default []
}

export def "status ready-count" [] {
  let cs = ($in | status containers)
  $cs | where ready == true | length
}

export def "status restart-sum" [] {
  let cs = ($in | status containers)

  $cs | reduce --fold 0 {|c acc|
    $acc + ($c.restartCount? | default 0)
  }
}

export def "status pod-phase" [] {
  let pod = $in
  let cs = ($pod | status containers)

  let waiting = (
    $cs
    | where state?.waiting? != null
    | get state.waiting
    | first
  )

  let terminated = (
    $cs
    | where state?.terminated? != null
    | get state.terminated
    | first
  )

  if ($waiting | is-not-empty) {
    $waiting.reason? | default "Waiting"
  } else if ($terminated | is-not-empty) {
    $terminated.reason? | default "Terminated"
  } else {
    $pod.status.phase? | default "Unknown"
  }
}

export def "node roles" [] {
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

export def "status node-ready" [] {
  let cond = ($in | status condition "Ready")

  if ($cond.status? == "True") {
    "Ready"
  } else if ($cond.status? == "False") {
    "NotReady"
  } else {
    "Unknown"
  }
}
