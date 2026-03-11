use "../../fmt/helpers.nu"

export def "nodes v1" [output?: string = compact] {
  let node = $in

  {
    name: $node.metadata.name
    roles: ($node | helpers node roles)
    millicores: ($node.usage.cpu | helpers cvt-cpu)
    memory: ($node.usage.memory | helpers cvt-filesize)
    timestamp: ($node.timestamp? | helpers fmt-time)
    window: ($node.window? | helpers cvt-duration)
  }
}

export def "pods v1" [output?: string = compact] {
  let pod = $in
  let containers = ($pod.containers? | default [])

  let cpu_total = (
    $containers
    | each {|c| $c.usage.cpu | helpers cvt-cpu }
    | if ($in | is-empty) { 0 } else { math sum }
  )

  let mem_total = (
    $containers
    | each {|c| $c.usage.memory | helpers cvt-filesize }
    | if ($in | is-empty) { 0b } else { math sum }
  )

  let base = {
    namespace: $pod.metadata.namespace
    name: $pod.metadata.name
    millicores: $cpu_total
    memory: $mem_total
    timestamp: ($pod.timestamp? | helpers fmt-time)
    window: ($pod.window? | helpers cvt-duration)
  }

  if $output == "compact" {
    return $base
  }

  $base | insert containers (
    $containers | each {|c|
      {
        name: $c.name
        millicores: ($c.usage.cpu | helpers cvt-cpu)
        memory: ($c.usage.memory | helpers cvt-filesize)
      }
    }
  )
}
