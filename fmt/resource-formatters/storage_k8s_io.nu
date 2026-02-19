
# ------
#  v1   
# ------

export def "csidrivers v1" [output?: string = compact] {
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

export def "storageclasses v1" [output?: string = compact] {
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

export def "volumeattachments v1" [output?: string = compact] {
  let va = $in

  let source = ($va.spec.source? | default {})
  let attach_error = ($va.status.attachError? | default {})

  let res = {
    name: $va.metadata.name
    pv: ($source.persistentVolumeName?)
    node: ($va.spec.nodeName?)
    attached: ($va.status.attached? | default false)
    age: ($va.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  $res
  | upsert attacher ($va.spec.attacher?)
  | upsert source (
    if ($source | is-empty) {
      null
    } else {
      $source
    }
  )
  | upsert attachError (
    if ($attach_error | is-empty) {
      null
    } else {
      {
        message: $attach_error.message?
        time: (
          if ($attach_error.time? | is-not-empty) {
            $attach_error.time | into datetime
          } else {
            null
          }
        )
      }
    }
  )
  | upsert created (
    if ($va.metadata.creationTimestamp? | is-not-empty) {
      $va.metadata.creationTimestamp | into datetime
    } else {
      null
    }
  )
}

export def "volumeattributeclasses v1" [output?: string = compact] {
  let vac = $in

  let params = ($vac.parameters? | default {})

  let res = {
    name: $vac.metadata.name
    driver: ($vac.driverName?)
    parameters: ($params | columns | length)
    age: ($vac.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  $res
  | upsert parameters $params
  | upsert finalizers ($vac.metadata.finalizers? | default [])
  | upsert created (
    if ($vac.metadata.creationTimestamp? | is-not-empty) {
      $vac.metadata.creationTimestamp | into datetime
    } else {
      null
    }
  )
}

export def "persistentvolumes v1" [output?: string = compact] {
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

export def "persistentvolumeclaims v1" [output?: string = compact] {
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
