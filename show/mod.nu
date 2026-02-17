use "../config"
use "../api"
use "../fmt"
use "../fmt/fmt-completers.nu"
use "../config/config-completers.nu"
use ./get-resource.nu
use ./watch-resource.nu
use ./show-completers.nu

# display one or many resources
export def --env main [
  resource: string@"show-completers api-resource" # the resource you want to get (po, deploy etc)
  resourcename?: string@"show-completers resourcename" # the name of the resource you want to get
  --context(-C): string@"config-completers context" # the context you want to use to get your resources
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
  let resources = if ($resource | str contains /) {
    $resource 
    | split column -n 3 / group version name 
  } else if $resource == all {
    let targets = [
      pods
      services
      replicationcontrollers
      deployments
      replicasets
      statefulsets
      daemonsets
    ]
    api resources -o wide
    | where {|resource| 
      $resource.names | any {|name| $name in $targets}
    }
  } else {
    let res = api resources -o wide
    | where {$resource in $in.names?} 

    if ($res | length) == 0 {
      error make {
        msg: "run 'nuke api resources' to get the full list"
        label: {
          text: $'($resource) is not a resource from the cluster'
          span: (metadata $resource).span} 
      }
    } 
    $res 
    | select group version name
  }

  if $watch {
    let res = $resources | first
    return (watch-resource $res $resourcename
      -n $namespace 
      -g $res.group 
      -v $res.version 
      -l $labels
      -c $conf
      -C $context
      -o $output
      --all=$all
    ) 
  }

  mut res = $resources | reduce --fold {} {|resource, acc|
    let r = (get-resource $resource.name $resourcename 
      -n $namespace 
      -g $resource.group 
      -v $resource.version 
      -l $labels
      -c $conf
      -C $context
      --all=$all
    )

    let decorators = [
      ...(if $all {['namespace']} else {[]})
      ...(if $show_labels {['labels']} else {[]})
      ...(if $show_annotations {['annotations']} else {[]})
      ...(if $show_conditions {['conditions']} else {[]})
    ]

    if ($r | is-not-empty) {
      $acc | merge { $resource.name: ($r | fmt -o $output -d $decorators) }
    } else {
      $acc
    }
  }

  if ($resources | length) == 1 {
    $res = $res 
    | transpose resource items 
    | get items 
    | first
  }

  $res
}
