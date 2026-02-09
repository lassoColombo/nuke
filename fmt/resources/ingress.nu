export def main [output?: string = compact] {
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
