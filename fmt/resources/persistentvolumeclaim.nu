export def main [output?: string = compact] {
  let pvc = $in
  let res = {
    name: $pvc.metadata.name
    phase: $pvc.status.phase
    volume: $pvc.spec.volumeName?
    capacity: $pvc.status.capacity.storage
    accessModes: $pvc.spec.accessModes
    storageclass: $pvc.spec.storageClassName?
    age: ($pvc.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res | insert volumeMode ($pvc.spec.volumeMode)
}
