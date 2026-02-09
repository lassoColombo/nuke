export def fmtage [] {
  (date now) - ($in | default {date now | into string} | into datetime)
}

export def fmtcontainers [] {
  $in.spec.template.spec.containers? | default [] | select name image command
}

export def fmtselector [] {
  $in.spec.selector?.matchLabels? | default {} | transpose key value
}

export def fmtresources [] {
  $in 
  | transpose kind amount 
  | each {|l|
    $l | update amount ($l.amount | into filesize)} 
  | reduce --fold {} {|elt acc| 
    $acc | merge {$elt.kind: $elt.amount}
  }
}
