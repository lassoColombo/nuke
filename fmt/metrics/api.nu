export def "nodes v1" [output?: string = compact] {
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

export def "pods v1" [output?: string = compact] {
  let pod = $in

  let containers = ($pod.containers? | default [])

  let cpu_total = (
    ( $containers
      | each {|c|
        $c.usage.cpu
        | str replace 'n' ''
        | into int
      }
      | math sum ) / 1000000
  )

  let mem_total = (
    $containers
    | each {|c|
      $c.usage.memory
      | str replace 'Ki' ''
      | into filesize
    }
    | math sum
  )

  let res = {
    namespace: $pod.metadata.namespace
    name: $pod.metadata.name
    millicores: $cpu_total
    memory: $mem_total
    containers: $pod.containers
    container_count: ($containers | length)
    timestamp: (
      if ($pod.timestamp? | is-not-empty) {
        $pod.timestamp | into datetime
      } else {
        null
      }
    )
    window: (
      if ($pod.window? | is-not-empty) {
        $pod.window | str replace 's' 'sec' | into duration
      } else {
        null
      }
    )
  }

  if $output == compact {
    return ($res | reject containers)
  }

  $res
  | upsert containers {|item|
    $item.containers
    | each {|c|
      {
        name: $c.name
        cpu: (
          ( $c.usage.cpu
            | str replace 'n' ''
            | into int ) / 1000000
        )
        memory: ($c.usage.memory | into filesize)
      }
    }
  }
}
