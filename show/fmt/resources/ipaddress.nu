export def main [output?: string = compact] {
  let ip = $in

  let parent = $ip.spec.parentRef?

  let res = {
    name: $ip.metadata.name
    parent: ($parent | default {} | select -o resource namespace name)
    ipFamily: ($ip.metadata.labels.'ipaddress.kubernetes.io/ip-family'?)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert managedBy ($ip.metadata.labels.'ipaddress.kubernetes.io/managed-by'? | default <none>)
  | upsert age ($ip.metadata.creationTimestamp? | helpers fmtage)
}
