export def main [output: string = compact] {
  let deploy = $in
  let res = {
    name: $deploy.metadata.name
    status: ($deploy.status.conditions?.type? | default [null] | first)
    replicas: ($deploy.status.replicas? | default 0)
    ready: ($deploy.status.readyReplicas? | default 0)
    available: ($deploy.status.availableReplicas? | default 0)
    updated: ($deploy.status.updatedReplicas? | default 0)
    age: ($deploy.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | insert selector ($deploy | helpers fmtselector)
  | insert containers ($deploy | helpers fmtcontainers)
}
