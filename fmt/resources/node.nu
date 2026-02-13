export def main [output?: string = compact] {
  let no = $in
  let directroles = ($no.metadata.labels | transpose key value | where key == kubernetes.io/role | get value)
  let indirectroles = ($no.metadata.labels | transpose key value | where {$in.key | str starts-with node-role.kubernetes.io/} | get key | each { $in | split row / | last })
  let roles = $directroles | append $indirectroles

  let res = {
    name: $no.metadata.name
    roles: $roles
    status: ($no.status.conditions?.type? | default [null] | last)
    age: ($no.metadata.creationTimestamp? | helpers fmtage)
    version: $no.status.nodeInfo.kubeletVersion?
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | insert kernel $no.status.nodeInfo.kernelVersion?
  | insert image $no.status.nodeInfo.osImage?
  | insert internalIPs ($no.status.addresses? | default [] | where type == InternalIP | get address)
  | insert externalIPs ($no.status.addresses? | default [] | where type == ExternalIP | get address)
}
