use ./helpers.nu
use "../http-get/"
use "../config/"
use "../fmt"

export def --env main [
  resource: any
  --labels(-l): string # filter resources by label
  --group(-g): string
  --version(-v): string
  --namespace(-n): string
  --conf(-c): any
  --decorators(-d): list
  --output(-o): string
  --fmt-suffix(-s): string
  --all(-A)
] {
  mut rv = $resource.metadata.resourceVersion
  mut state = []

  mut spec = (helpers build-path 
    $resource.kind
    $resource.metadata?.name?
    --labels $labels
    --group $group
    --version $version
    --namespace $namespace
    --conf $conf
    --all=$all
  )
  $spec.params = $spec.params? 
  | default [] 
  | append [
    {key: watch, value: true},
    {key: allowWatchBookmarks, value: true}
  ]

  while true {
    let result = (
      http-get $spec $conf -w
      | lines
      # terribly inefficient, sorry
      | reduce --fold {lines: '' state: $state rv: $rv} {|line, acc|

        let combined = $"($acc.lines)\n($line)"
        let event = try { $combined | from json }


        if ($event | is-empty) {
          return { lines: $combined state: $acc.state rv: $acc.rv }
        }

        if $event.type? == "BOOKMARK" {
          return {
            lines: ''
            state: $acc.state
            rv: $event.object.metadata.resourceVersion
          }
        }

        if $event.type == "ERROR" {
          error make { msg: ($event | to json) }
        }

        mut obj = $event.object
        if ($fmt_suffix | is-not-empty) {
          $obj.kind = $"($obj.kind)($fmt_suffix)"
        }

        let new_rv = $obj.metadata.resourceVersion
        let name = $obj.metadata.name

        mut new_state = $acc.state

        if $event.type == "ADDED" {
          $new_state = ($new_state | default [] | append ($obj | fmt resource -o compact -d $decorators))

        } else if $event.type == "MODIFIED" {
          $new_state = (
            $new_state
            | default []
            | where {|o| $o.name != $name }
            | append ($obj | fmt resource -o compact -d $decorators)
          )

        } else if $event.type == "DELETED" {
          $new_state = (
            $new_state
            | default []
            | where {|o| $o.name != $name }
          )
        }

        print ($new_state | table -e)

        {
          lines: ''
          state: $new_state
          rv: $new_rv
        }
      }
    )

    $state = $result.state
    $rv = $result.rv
    $spec.params = $spec.params
    | where key != resourceVersion 
    | append { key: resourceVersion, value: $rv }

    sleep 2sec
  }
}
