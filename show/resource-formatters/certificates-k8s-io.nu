use "../../fmt/helpers.nu"

# -----------------------
# helpers
# -----------------------

def "csr status" [] {
  let conds = ($in.status.conditions? | default [])

  if ($conds | where type == "Denied" | length) > 0 {
    return "Denied"
  }

  if ($conds | where type == "Approved" | length) > 0 {
    if ($in.status.certificate? | is-not-empty) {
      return "Issued"
    }
    return "Approved"
  }

  "Pending"
}

# -----------------------
# CertificateSigningRequests
# -----------------------

export def "certificatesigningrequests v1" [output?: string = compact] {
  let csr = $in

  let req = ($csr.spec.username? | default null)
  let signer = ($csr.spec.signerName? | default null)
  let usages = ($csr.spec.usages? | default [])

  let state = ($csr | csr status)

  let base = (
    $csr
    | helpers meta base
    | merge {
        signer: $signer
        requestor: $req
        status: $state
      }
  )

  if $output == "compact" {
    return $base
  }

  let groups = ($csr.spec.groups? | default [])

  let conditions = (
    $csr.status.conditions?
    | default []
    | each {|c|
      {
        type: $c.type
        status: $c.status
        reason: $c.reason?
        message: $c.message?
        updated: ($c.lastUpdateTime? | helpers fmt-time)
      }
    }
  )

  $base | merge {
    owner: ($csr | helpers meta owner)

    groups: $groups
    usages: $usages

    certificateIssued: (
      if ($csr.status.certificate? | is-not-empty) {
        true
      } else {
        false
      }
    )

    conditions: $conditions
  }
}
