
# ------
#  v1   
# ------

export def "ingresses v1" [output?: string = compact] {
  let ing = $in

  let rules = ($ing.spec.rules? | default [])

  let paths = (
    $rules
    | each {|r|
      $r.http?.paths? | default []
    }
    | flatten
  )

  let backends = (
    $paths
    | get backend.service?
    | where $it != null
  )

  let hosts = (
    $rules
    | get host?
    | where $it != null
    | uniq
  )

  let lb = ($ing.status.loadBalancer.ingress? | default [])

  let res = {
    name: $ing.metadata.name
    class: $ing.spec.ingressClassName?
    hosts: ($hosts | length)
    paths: ($paths | length)
    backends: ($backends | length)
    age: ($ing.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert generation ($ing.metadata.generation?)
  | upsert loadBalancer (
    $lb
    | each {|i|
      {
        ip: $i.ip?
        hostname: $i.hostname?
      }
    }
  )
  | upsert rules (
    $rules
    | each {|r|
      {
        host: ($r.host? | default "*")
        paths: (
          $r.http?.paths?
          | default []
          | each {|p|
            {
              path: $p.path?
              pathType: $p.pathType?
              service: $p.backend.service.name?
              port: (
                $p.backend.service.port.number?
                | default $p.backend.service.port.name?
              )
            }
          }
        )
      }
    }
  )
}

export def "ingressclasses v1" [output?: string = compact] {
  let ic = $in
  let params = ($ic.spec.parameters? | default {})

  let res = {
    name: $ic.metadata.name
    controller: $ic.spec.controller?
    parameters: (
      if ($params | is-empty) {
        null
      } else {
        $params | select -o kind name
      }
    )
    scope: ($params.scope? | default "Cluster")
    age: ($ic.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert apiGroup ($params.apiGroup?)
}

export def "ipaddresses v1" [output?: string = compact] {
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

export def "networkpolicies v1" [output?: string = compact] {
  let np = $in
  {
    name: $np.metadata.name
    selector: $np.spec.podSelector.matchLabels
    ingress: ($np.spec.ingress? | default [] | length)
    egress: ($np.spec.egress? | default [] | length)
    age: ($np.metadata.creationTimestamp? | helpers fmtage)
  }
}

export def "servicecidr v1" [output?: string = compact] {
  let sc = $in

  let ready_cond = ($sc.status.conditions? | where type == Ready | first | default {})
  let res = {
    name: $sc.metadata.name
    cidrs: ($sc.spec.cidrs? | default [])
    ready: ($ready_cond.status?)
  }

  if ($output | is-empty) or $output == compact {
    reutrn $res
  } 
  $res
  | upsert message ($ready_cond.message?)
  | upsert reason ($ready_cond.reason?)
  | upsert finalizers ($sc.metadata.finalizers?)
  | upsert age ($sc.metadata.creationTimestamp?)
}
