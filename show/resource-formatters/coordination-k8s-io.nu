use "../../fmt/helpers.nu"

# -----------------------
# Leases
# -----------------------

export def "leases v1" [output?: string = compact] {
  let l = $in
  let spec = ($l.spec? | default {})

  let renew = ($spec.renewTime? | helpers cvt-time)

  let base = (
    $l
    | helpers meta base
    | merge {
        holder: $spec.holderIdentity?
        renewTime: $renew
      }
  )

  if $output == "compact" {
    return $base
  }

  let acquire = ($spec.acquireTime? | helpers cvt-time)

  let age = (
    if ($renew | is-empty) {
      null
    } else {
      (date now) - $renew
    }
  )

  $base | merge {
    owner: ($l | helpers meta owner)

    leaseDurationSeconds: $spec.leaseDurationSeconds?
    acquireTime: $acquire
    transitions: ($spec.leaseTransitions? | default 0)

    renewAge: $age
  }
}
