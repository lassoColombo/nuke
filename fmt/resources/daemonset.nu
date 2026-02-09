export def main [output: string = compact] {
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
