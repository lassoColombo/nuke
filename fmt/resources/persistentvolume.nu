export def main [output?: string = compact] {
  let pv = $in
  let res = {
    name: $pv.metadata.name
    capacity: $pv.spec.capacity?.storage?
    accessModes: $pv.spec.accessModes?
    reclaimPolicy: $pv.spec.persistentVolumeReclaimPolicy?
    phase: $pv.status.phase?
    claim: ($pv.spec.claimRef? | if ($in | is-empty) {$in} else {$in | select -o namespace name})
    storageclass: $pv.spec.storageClassName?
    age: ($pv.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res | insert volumeMode ($pv.spec.volumeMode)
}
