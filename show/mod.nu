use "../config"
use "../api"
use "../fmt"
use "../fmt/fmt-completers.nu"
use ./get-resource.nu
use ./watch-resource.nu
use ./show-completers.nu


# display one or many resources
export def --env main [
  resource: string@"show-completers api-resource" # the resource you want to get (po, deploy etc)
  resourcename?: string@"show-completers resourcename" # the name of the resource you want to get
  --namespace(-n): string@"show-completers namespace" # the namespace you want to get your resource(s) from
  --all(-A) # get all the specified resources
  --labels(-l): string # filter resources by label
  --output(-o): string@"fmt-completers output" # the format of the output
  --show-annotations(-a) # appends the object's annotations to the output
  --show-labels(-l) # appends the object's labels to the output
  --show-conditions(-c) # appends the object's conditions to the output
  --watch(-w) # watch the required objects for changes
] {
  if ($output | is-not-empty) and not ($output in (fmt supported-outputs)) {
    error make {
      msg: $'Supported outputs are (fmt supported-outputs)'
      label: {
        text: $'($output) is not a supported output'
        span: (metadata $output).span
      }
    }
  }

  let conf = config
  let resource = if ($resource | str contains /) {
    $resource | split column -n 3 / group version name | first
  } else {
    let res = api resources -o wide
    | where {$resource in $in.names?} 
    if ($res | length) == 0 {
      error make {
        msg: "run 'nuke api resources' to get the full list"
        label: {
          text: $'($resource) is not a resource from the cluster'
          span: (metadata $resource).span
        }
      }
    } 
    $res | first | select group version name
  }

  mut res = (get-resource $resource.name $resourcename 
    -n $namespace 
    -g $resource.group 
    -v $resource.version 
    -l $labels
    -c $conf
    --all=$all
  )

  let decorators = [
    ...(if $all {['namespace']} else {[]})
    ...(if $show_labels {['labels']} else {[]})
    ...(if $show_annotations {['annotations']} else {[]})
    ...(if $show_conditions {['conditions']} else {[]})
  ]

  if not $watch {
    return ($res | fmt resource -o $output -d $decorators)
  }

  (
    watch-resource $res
    -n $namespace 
    -g $resource.group 
    -v $resource.version 
    -l $labels
    -c $conf
    -o $output
    -d $decorators
    --all=$all
  ) | ignore
}
