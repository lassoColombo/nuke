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
    created: ($m.creationTimestamp? | fmt-time)
    name: $m.name
  }
}

export def "meta owner" [] {
  let ref = (
    $in.metadata.ownerReferences?
    | default []
    | where controller == true
    | get -o 0
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
  mut result = {}
  if ($r.requests? | is-not-empty) {
    $result.requests = $r.requests
  }
  if ($r.limits? | is-not-empty) {
    $result.limits = $r.limits
  }
  $result
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
  | each {|c| $c | container base}
}

export def "tpl images" [] {
  $in.spec.template.spec.containers?
  | default []
  | get image
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

# -------------------
#  resource parsing
# -------------------

export def "res cpu-millicores" [] {
  let v = $in

  if ($v | is-empty) { 
    null 
  } else if ($v | str ends-with "m") {
    ($v | str replace "m" "" | into float)
  } else {
    (($v | into float) * 1000.0)
  }
}

export def "res memory-bytes" [] {
  let v = $in
  if ($v | is-empty) { return null }

  let units = {
    Ki: 1024
    Mi: (1024 ** 2)
    Gi: (1024 ** 3)
    Ti: (1024 ** 4)
    Pi: (1024 ** 5)
    Ei: (1024 ** 6)

    k: 1000
    M: (1000 ** 2)
    G: (1000 ** 3)
    T: (1000 ** 4)
    P: (1000 ** 5)
    E: (1000 ** 6)

    m: 0.001
    u: 0.000001
    n: 0.000000001
  }

  let suffix = (
    $units
    | columns
    | sort-by {|u| $u | str length } -r   # longest match first
    | where {|u| $v | str ends-with $u }
    | first
  )

  if ($suffix | is-empty) {
    ($v | into float | into int | into filesize)
  } else {
    let num = ($v | str replace $suffix "" | into float)
    let mult = ($units | get $suffix)

    (($num * $mult) | into int | into filesize)
  }
}

export def "res normalize" [] {
  let r = ($in | default {})

  {
    cpu: ($r.cpu? | res cpu-millicores)
    memory: ($r.memory? | res memory-bytes)
  }
}
