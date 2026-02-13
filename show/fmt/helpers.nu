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

export def fmtcontainers [] {
  $in.spec.template.spec.containers? 
  | default [] 
  | select -o name image command args resources
  | each {|c|
    $c | merge ($c.resources? | fmtresources)
  }
  | reject resources
}

export def fmtselector [] {
  $in.spec.selector?.matchLabels? | default {} | transpose key value
}
