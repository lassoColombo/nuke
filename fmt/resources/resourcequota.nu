export def main [output?: string = compact] {
  let rq = $in

  let hard = ($rq.status?.hard? | default $rq.spec?.hard? | default {})
  let used = ($rq.status?.used? | default {})

  let resources = ($hard | columns)

  let usage = (
    $resources
    | each {|r|
      let h = ($hard | get $r)
      let u = ($used | get $r | default 0)

      {
        resource: $r
        hard: $h
        used: $u
      }
    }
  )

  let res = {
    name: $rq.metadata.name
    namespace: $rq.metadata.namespace?
    resources: ($resources | length)
    age: ($rq.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert quotas (
    $usage
    | each {|u|
      let pct = (
        try {
          ($u.used | into float) / ($u.hard | into float) * 100
        } catch {
          null
        }
      )
      {
        resource: $u.resource
        used: $u.used
        hard: $u.hard
        percent: $pct
      }
    }
  )
}
