export def main [output?: string = compact] {
  let rs = $in
  let status = $rs.status?
  | default {}
  | reject -o observedGeneration fullyLabeledReplicas
  | insert ready ($in.readyReplicas? | default 0)
  | insert available ($in.availableReplicas? | default 0)
  | reject -o readyReplicas availableReplicas

  let res = {
    name: $rs.metadata.name
    age: ($rs.metadata.creationTimestamp? | helpers fmtage)
  } | merge $status

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | insert selector ($rs | helpers fmtselector)
  | insert containers ($rs | helpers fmtcontainers)
}
