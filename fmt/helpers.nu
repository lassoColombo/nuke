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

export def "status condition" [type: string] {
  (
    $in.status.conditions?
    | default []
    | where type == $type
    | first
    | default {}
  )
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

# -------------------
#  resource parsing
# -------------------

export def "cvt-cpu" [] {
  let v = $in
  if ($v | is-empty) { return null }
  let s = ($v | str trim)

  if ($s | str ends-with "n") {
    $s | str replace "n" "" | into int | $in / 1_000_000 | math round | into int
  } else if ($s | str ends-with "u") {
    $s | str replace "u" "" | into int | $in / 1_000 | math round | into int
  } else if ($s | str ends-with "m") {
    $s | str replace "m" "" | into int
  } else {
    $s | into float | $in * 1000 | math round | into int
  }
}

# Parses a Go duration string ("30s", "1m0s", "15.911s", "1h5m30s") into a nushell duration.
export def "cvt-duration" [] {
  let s = ($in | default "" | str trim)
  if ($s | is-empty) { return null }

  let hours = (
    if ($s | str contains "h") {
      $s | parse --regex '(\d+)h' | get -o 0 | get -o capture0 | default "0" | into int
    } else { 0 }
  )
  let minutes = (
    if ($s | str contains "m") {
      $s | parse --regex '(\d+(?:\.\d+)?)m(?!s)' | get -o 0 | get -o capture0 | default "0" | into float | into int
    } else { 0 }
  )
  let seconds = (
    if ($s | str contains "s") {
      $s | parse --regex '(\d+(?:\.\d+)?)s' | get -o 0 | get -o capture0 | default "0" | into float | into int
    } else { 0 }
  )

  let total_sec = ($hours * 3600) + ($minutes * 60) + $seconds
  $"($total_sec)sec" | into duration
}

export def "cvt-filesize" [] {
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
