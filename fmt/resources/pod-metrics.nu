export def main [output?: string = compact] {
  let items = $in

  let rows = (
    $items
    | each {|pod|
      let containers = ($pod.containers? | default [])

      let cpu_total = (
        ( $containers
        | each {|c|
          $c.usage.cpu
          | str replace 'n' ''
          | into int
        }
        | math sum ) / 1000
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
      {
        namespace: $pod.metadata.namespace
        name: $pod.metadata.name
        millicores: $cpu_total
        memory: $mem_total
        containers: ($containers | length)
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
    }
  )

  if $output == compact {
    return $rows
  }

  $rows
  | each {|row|
    let pod = (
      $items
      | where metadata.name == $row.name and metadata.namespace == $row.namespace
      | first
    )

    $row
    | upsert containers_detail (
      $pod.containers
      | each {|c|
        {
          name: $c.name
          cpu: (
            ( $c.usage.cpu
            | str replace 'n' ''
            | into int ) / 1000
          )
          memory: (
            $c.usage.memory | into filesize
          )
        }
      }
    )
  }
}
