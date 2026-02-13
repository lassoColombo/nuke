export def main [output?: string = compact] {
  let sts = $in
  let res = {
    name: $sts.metadata.name
    replicas: ($sts.status.replicas? | default 0)
    ready: ($sts.status.readyReplicas? | default 0)
    updated: ($sts.status.updatedReplicas? | default 0)
    available: ($sts.status.availableReplicas? | default 0)
    age: ($sts.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res | insert containers ($sts | helpers fmtcontainers)
}
