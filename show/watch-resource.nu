use ./helpers.nu
use "../http-get/"
use "../config/"

export def --env main [
  resource: any
  --group(-g): string
  --version(-v): string
  --namespace(-n): string
  --conf(-c): any
  --decorators(-d): list
  --output(-o): string
  --all(-A)
] {

  mut resource_version = $resource.metadata.resourceVersion
  mut state = ($resource | fmt resource -o compact -d $decorators)
  print ($state | table -e)

  let base = (helpers build-path 
    ($resource.kind | str downcase | str replace --regex 'list$' '')
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
  | reduce --fold $state {|line, acc|
    let event = ($line | from json)

    if $event.type == "BOOKMARK" {
      # $resource_version = $event.object.metadata.resourceVersion
      return
    }

    if $event.type == "ERROR" {
      error make {msg: ($event | to json)}
    }

    let obj = $event.object
    # $resource_version = $obj.metadata.resourceVersion
    let name = $obj.metadata.name

    mut res = []
    if $event.type == "ADDED" {
      $res = ($acc | default [] | append ($obj | fmt resource -o compact -d $decorators))
    } else if $event.type == "MODIFIED" {
      $res = (
        $acc | default []
        | where {|o| $o.name != $name }
        | append ($obj | fmt resource -o compact -d $decorators)
      )
    } else if $event.type == "DELETED" {
      $res = (
        $acc | default []
        | where {|o| $o.name != $name }
      )
    }

    print ($res | table -e)
    $res
  }
}
