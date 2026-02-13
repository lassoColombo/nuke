use ./helpers.nu
use "../http-get/"
use "../config/"
use "../fmt"

export def --env main [
  resource: any
  --group(-g): string
  --version(-v): string
  --namespace(-n): string
  --conf(-c): any
  --decorators(-d): list
  --output(-o): string
  --fmt-suffix(-s): string
  --all(-A)
] {

  mut resource_version = $resource.metadata.resourceVersion
  mut state = ($resource | fmt resource -o compact -d $decorators)
  print ($state | table -e)

  let actualkind = $resource.kind 
  | str downcase 
  | str replace --regex 'list$' ''
  | str replace --regex 'rolloutstatus$' ''
  | str replace --regex 'rollouthistory$' ''
  | str replace --regex 'metrics$' ''

  let base = (helpers build-path 
    $actualkind
    $resource.metadata?.name?
    --group $group
    --version $version
    --namespace $namespace
    --conf $conf
    --all=$all
  )
  let watch_path = $"($base)?watch=true&resourceVersion=($resource_version)&allowWatchBookmarks=true"
  http-get $watch_path $conf -w
  | lines
  | reduce --fold {lines: '' state: $state} {|line, acc|
    mut res = {
      state: $acc.state
      lines: $"($acc.lines)\n($line)"
    }
    let event = try { $acc.lines | from json } 
    if ($event | is-empty) {return {
      state: $acc.state
      lines: $"($acc.lines)\n($line)"
    }}

    if $event.type == "BOOKMARK" {
      # $resource_version = $event.object.metadata.resourceVersion
      return
    }

    if $event.type == "ERROR" {
      error make {msg: ($event | to json)}
    }

    mut obj = $event.object
    if ($fmt_suffix | is-not-empty) {
      $obj.kind = $"($obj.kind)($fmt_suffix)"
    }
    # $resource_version = $obj.metadata.resourceVersion
    let name = $obj.metadata.name

    if $event.type == "ADDED" {
      $res.state = ($res.state | default [] | append ($obj | fmt resource -o compact -d $decorators))
    } else if $event.type == "MODIFIED" {
      $res.state = (
        $res.state | default []
        | where {|o| $o.name != $name }
        | append ($obj | fmt resource -o compact -d $decorators)
      )
    } else if $event.type == "DELETED" {
      $res.state = (
        $res.state | default []
        | where {|o| $o.name != $name }
      )
    }

    print ($res.state | table -e)
    $res
  }
}
