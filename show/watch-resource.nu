use ./helpers.nu
use "../http-get/"
use "../cfg/"

export def --env main [
  resource: string
  resourcename?: string
  --group(-g): string
  --version(-v): string
  --namespace(-n): string
  --conf(-c): any
  --decorators(-d): list
  --output(-o): string
  --all(-A)
] {
  let base = (helpers build-path 
    $resource
    $resourcename
    --group $group 
    --version $version
    --namespace $namespace
    --conf $conf
    --all=$all
  )

  let initial = (http-get $base $conf)
  mut resource_version = $initial.metadata.resourceVersion

  mut state = ($initial | fmt resource -o compact -d $decorators)
  print ($state | table -e)

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
      $res = ($acc | append ($obj | fmt resource -o compact -d $decorators))
    } else if $event.type == "MODIFIED" {
      $res = (
        $acc
        | where {|o| $o.name != $name }
        | append ($obj | fmt resource -o compact -d $decorators)
      )
    } else if $event.type == "DELETED" {
      $res = (
        $acc
        | where {|o| $o.name != $name }
      )
    }

    print ($res | table -e)
    $res
  }
}
