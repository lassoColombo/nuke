export def fmtduration [] {
  let d = (
    $in
    | into string
    | parse --regex '^(?:(?P<wk>\d+)wk)?(?:\s*(?P<day>\d+)day)?(?:.*?(?P<hr>\d+)hr)?(?:.*?(?P<min>\d+)min)?(?:.*?(?P<sec>\d+)sec)?'
    | first
  )

  let res = if ($d.wk | is-not-empty) {
    $"($d.wk)w ($d.day)d"
  } else if ($d.day | is-not-empty) {
    $"($d.day)d ($d.hr)h"
  } else if ($d.hr | is-not-empty) {
    $"($d.hr)h ($d.min)m"
  } else if ($d.min | is-not-empty) {
    $"($d.min)m ($d.sec)s"
  } else if ($d.sec | is-not-empty) {
    $"($d.sec)s"
  } else {
    "0s"
  }

  $res
}

export def fmtage [] {
  (((date now) - ($in | default {date now | into string} | into datetime)) | fmtduration)
}

export def fmtcontainers [] {
  $in.spec.template.spec.containers? | default [] | select name image
}

export def fmtselector [] {
  $in.spec.selector?.matchLabels? | default {} | transpose key value
}
