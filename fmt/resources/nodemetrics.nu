export def main [output?: string = compact] {
  let node = $in
  let labels = ($node.metadata.labels? | default {})

  let directroles = (
    $labels
    | transpose key value
    | where key == kubernetes.io/role
    | get value
  )

  let indirectroles = (
    $labels
    | transpose key value
    | where { $in.key | str starts-with node-role.kubernetes.io/ }
    | get key
    | each { $in | split row / | last }
  )

  let roles = ($directroles | append $indirectroles)

  let cpu_mc = (
    $node.usage.cpu
    | str replace 'n' ''
    | into int
  ) / 1000000

  let mem = (
    $node.usage.memory
    | str replace 'Ki' ''
    | into filesize
  )

  let res = {
    name: $node.metadata.name
    roles: $roles
    millicores: $cpu_mc
    memory: $mem
    timestamp: (
      if ($node.timestamp? | is-not-empty) {
        $node.timestamp | into datetime
      } else {
        null
      }
    )
    creationTimestamp: (
      if ($node.metadata.creationTimestamp? | is-not-empty) {
        $node.metadata.creationTimestamp | into datetime
      } else {
        null
      }
    )
    window: (
      if ($node.window? | is-not-empty) {
        $node.window
        | str replace 's' 'sec'
        | into duration
      } else {
        null
      }
    )
  }

  return $res
}
