
# ------
#  v1   
# ------

export def "apiservices v1" [output?: string = compact] {
  let as = $in

  let available_cond = ($as.status.conditions? | default [] | where type == Available | first)
  let service = if ($as.spec.service? | is-empty) { {} } else {
    ($as.spec.service? | default {} | select -o namespace name)
  }

  let res = {
    name: $as.metadata.name
    service: $service
    available: ($available_cond.status?)
    age: ($as.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  $res
  | upsert message ($available_cond.message?)
  | upsert groupPriority ($as.spec.groupPriorityMinimum?)
  | upsert versionPriority ($as.spec.versionPriority?)
  | upsert caBundle ($as.spec.caBundle? | is-not-empty)
}
