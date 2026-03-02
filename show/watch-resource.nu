use ./helpers.nu
use "../http-get/"
use "../config/"
use "../fmt"

export def main [
  resource: record
  resourcename?: string
  --group(-g): string
  --version(-v): string
  --namespace(-n): string
  --all-namespaces(-A)
  --selector(-l): string # filter resources by label

  --decorators(-d): list
  --output(-o): string

  --kubeconf(-K): any
  --context(-c): string
  --cluster(-C): string
] {
  let base = (get-resource $resource.name $resourcename 
    -n $namespace 
    -g $resource.group 
    -v $resource.version 
    -l $selector
    -c $kubeconf
    -C $context
    --all-namespaces=$all_namespaces
  )
  mut rv = $base.metadata.resourceVersion
  mut state = []

  mut spec = (helpers build-path 
    $base.kind
    $base.metadata?.name?
    --selector $selector
    --group $group
    --version $version
    --namespace $namespace
    --kubeconf $kubeconf
    --all-namespaces=$all_namespaces
  )
  $spec.params = $spec.params? 
  | default [] 
  | append [
    {key: watch, value: true},
    {key: allowWatchBookmarks, value: true}
  ]

  let formatters = resource-formatters
  | merge deep ($env.NUKE_RESOURCE_FORMATTERS? | default {})

  while true {
    let result = (
      http-get $spec -K $kubeconf -c $context -r 
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

        let obj = $event.object
        let new_rv = $obj.metadata.resourceVersion
        let name = $obj.metadata.name

        mut new_state = $acc.state


        if $event.type == "ADDED" {
          $new_state = ($new_state | default [] | append ($obj | fmt $resource $formatters -d $decorators -o compact))

        } else if $event.type == "MODIFIED" {
          $new_state = (
            $new_state
            | default []
            | where {|o| $o.name != $name }
            | append ($obj | fmt $resource $formatters -d $decorators -o compact)
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
