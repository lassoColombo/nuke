export def fmtage [] {
  (((date now) - ($in | default {date now | into string} | into datetime)))
}

export def fmtcontainers [] {
  $in.spec.template.spec.containers? | default [] | select name image
}

export def fmtselector [] {
  $in.spec.selector?.matchLabels? | default {} | transpose key value
}
