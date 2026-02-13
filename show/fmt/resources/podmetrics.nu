export def main [output?: string = compact] {
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
