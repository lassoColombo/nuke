export def main [output?: string = compact] {
  let sc = $in

  let is_default = (
    $sc.metadata.annotations.'storageclass.kubernetes.io/is-default-class'?
    | default "false"
    | str downcase
    | $in == "true"
  )

  let res = {
    name: $sc.metadata.name
    provisioner: $sc.provisioner
    reclaimPolicy: ($sc.reclaimPolicy? | default "Delete")
    bindingMode: ($sc.volumeBindingMode? | default "Immediate")
    default: $is_default
    expand: ($sc.allowVolumeExpansion? | default false)
    age: ($sc.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert generation ($sc.metadata.generation?)
  | upsert parameters ($sc.parameters? | default {})
  | upsert mountOptions ($sc.mountOptions? | default [])
}
