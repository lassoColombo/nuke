use "../../fmt/helpers.nu"

# -----------------------
# storage helpers
# -----------------------

export def "stor driver-name" [] {
  $in.spec.driver?
}

export def "stor volume-mode" [] {
  $in.spec.volumeMode? | default "Filesystem"
}

export def "stor access-modes" [] {
  $in.spec.accessModes? | default []
}

export def "stor capacity" [] {
  $in.status.capacity.storage?
  | helpers res memory-bytes
}

export def "csidrivers v1" [output?: string = compact] {
  let drv = $in

  let base = (
    $drv
    | helpers meta base
    | merge {
      attachRequired: ($drv.spec.attachRequired? | default true)
      podInfoOnMount: ($drv.spec.podInfoOnMount? | default false)
      storageCapacity: ($drv.spec.storageCapacity? | default false)
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    owner: ($drv | helpers meta owner)

    fsGroupPolicy: $drv.spec.fsGroupPolicy?

    volumeLifecycleModes: (
      $drv.spec.volumeLifecycleModes?
      | default []
    )

    requiresRepublish: ($drv.spec.requiresRepublish? | default false)

    tokenRequests: (
      $drv.spec.tokenRequests?
      | default []
      | each {|t|
        {
          audience: $t.audience?
          expirationSeconds: $t.expirationSeconds?
        }
      }
    )
  }
}

export def "csinodes v1" [output?: string = compact] {
  let node = $in

  let drivers = ($node.spec.drivers? | default [])

  let base = (
    $node
    | helpers meta base
    | merge {
      drivers: ($drivers | length)
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    driversSpec: (
      $drivers
      | each {|d|
        {
          name: $d.name
          nodeID: $d.nodeID?
          topologyKeys: ($d.topologyKeys? | default [])
        }
      }
    )
  }
}

export def "csistoragecapacities v1" [output?: string = compact] {
  let cap = $in

  let base = (
    $cap
    | helpers meta base
    | merge {
      storageClass: $cap.storageClassName?
      capacity: ($cap | stor capacity)
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    nodeTopology: $cap.nodeTopology?
    maximumVolumeSize: (
      $cap.maximumVolumeSize?
      | helpers res memory-bytes
    )

    owner: ($cap | helpers meta owner)
  }
}

export def "storageclasses v1" [output?: string = compact] {
  let sc = $in

  let base = (
    $sc
    | helpers meta base
    | merge {
      provisioner: $sc.provisioner?
      reclaimPolicy: ($sc.reclaimPolicy? | default "Delete")
      volumeBindingMode: ($sc.volumeBindingMode? | default "Immediate")
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    parameters: ($sc.parameters? | default {})

    allowVolumeExpansion: ($sc.allowVolumeExpansion? | default false)

    mountOptions: ($sc.mountOptions? | default [])

    isDefault: (
      $sc.metadata.annotations."storageclass.kubernetes.io/is-default-class"? == "true"
    )
  }
}

export def "volumeattachments v1" [output?: string = compact] {
  let va = $in

  let src = $va.spec.source?

  let volume = (
    if ($src.persistentVolumeName? | is-not-empty) {
      $"pv/($src.persistentVolumeName)"
    } else {
      null
    }
  )

  let base = (
    $va
    | helpers meta base
    | merge {
      node: $va.spec.nodeName?
      attached: ($va.status.attached? | default false)
      volume: $volume
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    attachError: $va.status.attachError?
    detachError: $va.status.detachError?

    attachmentMetadata: (
      $va.status.attachmentMetadata?
      | default {}
    )

    owner: ($va | helpers meta owner)
  }
}

export def "volumeattributesclasses v1" [output?: string = compact] {
  let vac = $in

  let base = (
    $vac
    | helpers meta base
    | merge {
      driver: $vac.driverName?
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    parameters: ($vac.parameters? | default {})

    owner: ($vac | helpers meta owner)
  }
}
