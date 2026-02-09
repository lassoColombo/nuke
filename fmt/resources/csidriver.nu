export def main [output?: string = compact] {
  let csi = $in

  let modes = ($csi.spec.volumeLifecycleModes? | default [])

  let res = {
    name: $csi.metadata.name
    attach: ($csi.spec.attachRequired? | default true)
    ephemeral: ($modes | any {|m| $m == "Ephemeral"})
    capacity: ($csi.spec.storageCapacity? | default false)
    fsGroupPolicy: ($csi.spec.fsGroupPolicy? | default null)
    age: ($csi.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert generation ($csi.metadata.generation?)
  | upsert volumeLifecycleModes $modes
  | upsert podInfoOnMount ($csi.spec.podInfoOnMount? | default false)
  | upsert requiresRepublish ($csi.spec.requiresRepublish? | default false)
  | upsert seLinuxMount ($csi.spec.seLinuxMount? | default false)
  | upsert attachRequired ($csi.spec.attachRequired? | default true)
  | upsert storageCapacity ($csi.spec.storageCapacity? | default false)
}
