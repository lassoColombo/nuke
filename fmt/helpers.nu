export def fmtage [] {
  (date now) - ($in | default {date now | into string} | into datetime)
}

export def fmtresources [] {
  let resources = $in | default {}
  {
    requests: $resources.requests?
    limits: $resources.limits?
  }
}

export def fmtcontainers [
  --output(-o): string
] {
  $in.spec.template.spec.containers? 
  | default [] 
  | select -o name image command args resources
  | each {|c|
    $c | merge ($c.resources? | fmtresources)
  }
  | reject resources
  | if $output != wide {
    $in | reject args resources
  } else {
    $in
  }
}

export def fmtselector [] {
  $in.spec.selector?.matchLabels? | default {} | transpose key value
}


# -------
#  new   
# -------

export def "meta base" [] {
  let m = $in.metadata

  {
    name: $m.name
    created: ($m.creationTimestamp? | into datetime)
    age: ($m.creationTimestamp? | fmtage)
  }
}

export def "meta controller-owner" [] {
  $in.metadata.ownerReferences?
  | default []
  | where controller == true
  | if ($in | length) != 0 { $in } else { [null] }
  | first
  | if ($in | is-empty) { null } else {
      $"($in.kind | str downcase)/($in.name)"
    }
}

export def "meta labels" [] {
  $in.metadata.labels? | default {}
}

export def "meta annotations" [] {
  $in.metadata.annotations? | default {}
}

export def "status condition" [type: string] {
  $in.status.conditions?
  | default []
  | where type == $type
  | if ($in | length) != 0 { $in } else { [{}] }
  | first
}

export def "tpl get" [] {
  $in.spec.template?
}

export def "tpl containers" [] {
  $in.spec.template.spec.containers?
  | default []
  | each {|c|
      {
        name: $c.name
        image: $c.image
        command: $c.command?
        args: $c.args?
        ...($c.resources? | fmtresources)
      }
    }
}

export def "tpl images" [] {
  $in.spec.template.spec.containers?
  | default []
  | get image
}

export def "spec selector" [] {
  $in.spec.selector.matchLabels? | default {}
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

export def "spec rolling-strategy" [] {
  let s = $in.spec.strategy? | default {}

  let rolling = ($s.rollingUpdate? | default {})

  {
    type: ($s.type? | default "RollingUpdate")
    maxUnavailable: $rolling.maxUnavailable?
    maxSurge: $rolling.maxSurge?
  }
}
