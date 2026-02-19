use "../helpers.nu"

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

  let cpu_mc = ($node.usage.cpu | helpers cpu-to-millicores) 

  let mem = ($node.usage.memory | into filesize)

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
        | str replace 'm' 'min'
        | str replace 'h' 'hour'
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
    $containers | each {|c|
      $c.usage.cpu | helpers cpu-to-millicores
    } | math sum
  )

  let mem_total = (
    $containers.usage.memory | into filesize | math sum
  )

  let res = {
    namespace: $pod.metadata.namespace
    name: $pod.metadata.name
    millicores: $cpu_total
    memory: $mem_total
    containers: $pod.containers
    timestamp: (
      if ($pod.timestamp? | is-not-empty) {
        $pod.timestamp | into datetime
      } else {
        null
      }
    )
    window: (
      if ($pod.window? | is-not-empty) {
        $pod.window 
        | str replace 's' 'sec'
        | str replace 'm' 'min'
        | str replace 'h' 'hour'
        | into duration
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
        cpu: ($c.usage.cpu | helpers cpu-to-millicores)
        memory: ($c.usage.memory | into filesize)
      }
    }
  }
}
