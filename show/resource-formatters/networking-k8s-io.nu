use "../../fmt/helpers.nu"

# -----------------------
# networking helpers
# -----------------------

export def "net ingress-class" [] {
  $in.spec.ingressClassName?
}

export def "net ingress-hosts" [] {
  $in.spec.rules?
  | default []
  | get host
  | where {|h| $h | is-not-empty }
}

export def "net ingress-tls-hosts" [] {
  $in.spec.tls?
  | default []
  | each {|t| $t.hosts? | default [] }
  | flatten
}

export def "net policy-types" [] {
  $in.spec.policyTypes? | default []
}

export def "net pod-selector" [] {
  $in.spec.podSelector.matchLabels? | default {}
}

export def "ingressclasses v1" [output?: string = compact] {
  let ic = $in

  let base = (
    $ic
    | helpers meta base
    | merge {
      controller: $ic.spec.controller?
    }
  )

  if $output == "compact" {
    return $base
  }

  let params = $ic.spec.parameters?

  $base | merge {
    owner: ($ic | helpers meta owner)

    parameters: (
      if ($params | is-empty) {
        null
      } else {
        {
          apiGroup: $params.apiGroup?
          kind: $params.kind?
          name: $params.name?
          namespace: $params.namespace?
        }
      }
    )

    isDefault: (
      $ic.metadata.annotations."ingressclass.kubernetes.io/is-default-class"? == "true"
    )
  }
}

export def "ingresses v1" [output?: string = compact] {
  let ing = $in

  let hosts = ($ing | net ingress-hosts)
  let tls_hosts = ($ing | net ingress-tls-hosts)

  let base = (
    $ing
    | helpers meta base
    | merge {
      class: ($ing | net ingress-class)
      hosts: $hosts
      tlsHosts: $tls_hosts
    }
  )

  if $output == "compact" {
    return $base
  }

  let rules = (
    $ing.spec.rules?
    | default []
    | each {|r|
      {
        host: $r.host?
        paths: (
          $r.http.paths?
          | default []
          | each {|p|
            {
              path: ($p.path? | default "/")
              pathType: ($p.pathType? | default "ImplementationSpecific")

              backend: (
                if ($p.backend.service? | is-not-empty) {
                  {
                    service: $p.backend.service.name
                    port: $p.backend.service.port.number?
                  }
                } else {
                  null
                }
              )
            }
          }
        )
      }
    }
  )

  $base | merge {
    owner: ($ing | helpers meta owner)

    tls: (
      $ing.spec.tls?
      | default []
      | each {|t|
        {
          secret: $t.secretName?
          hosts: ($t.hosts? | default [])
        }
      }
    )

    rules: $rules
  }
}

export def "ipaddresses v1" [output?: string = compact] {
  let ip = $in

  let parent = (
    if ($ip.spec.parentRef? | is-empty) {
      null
    } else {
      let r = $ip.spec.parentRef
      $"($r.resource | str downcase)/($r.name)"
    }
  )

  let base = (
    $ip
    | helpers meta base
    | merge {
      address: $ip.spec.address?
      parent: $parent
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    owner: ($ip | helpers meta owner)
    parentRef: $ip.spec.parentRef?
    labels: ($ip.metadata.labels? | default {})
  }
}

export def "networkpolicies v1" [output?: string = compact] {
  let np = $in

  let types = ($np | net policy-types)

  let base = (
    $np
    | helpers meta base
    | merge {
      podSelector: ($np | net pod-selector)
      policyTypes: $types
    }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {

    ingress: (
      $np.spec.ingress?
      | default []
      | each {|r|
        {
          ports: ($r.ports? | default [])
          from: ($r.from? | default [])
        }
      }
    )

    egress: (
      $np.spec.egress?
      | default []
      | each {|r|
        {
          ports: ($r.ports? | default [])
          to: ($r.to? | default [])
        }
      }
    )

    owner: ($np | helpers meta owner)
  }
}

export def "servicecidrs v1" [output?: string = compact] {
  let sc = $in

  let cidrs = ($sc.spec.cidrs? | default [])

  let base = (
    $sc
    | helpers meta base
    | merge {
      cidrs: $cidrs
    }
  )

  if $output == "compact" {
    return $base
  }

  let conditions = (
    $sc.status.conditions?
    | default []
    | each {|c|
      {
        type: $c.type
        status: $c.status
        reason: $c.reason?
        message: $c.message?
        updated: ($c.lastTransitionTime? | helpers cvt-time)
      }
    }
  )

  $base | merge {
    conditions: $conditions
    owner: ($sc | helpers meta owner)
  }
}
