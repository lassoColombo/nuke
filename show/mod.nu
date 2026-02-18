use "../config"
use "../api"
use "../api/resolve-resource.nu"
use "../fmt"
use "../fmt/fmt-completers.nu"
use "../config/config-completers.nu"
use ./get-resource.nu
# use ./watch-resource.nu
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
  # --watch(-w) # watch the required objects for changes
] {
  if ($output | is-not-empty) and not ($output in (fmt supported-outputs)) {
    error make --unspanned { msg: $'Supported outputs are (fmt supported-outputs)' }
  }
  let conf = config

  let resources = if ($resource | str contains /) {
    $resource | split column -n 3 / group version name 
  } else if $resource != all {
    resolve-resource $resource (
      api resources -o wide | where {$resource in $in.names?}  
    ) | select group version name
  } else {
    [
      {group: api version: v1 name: pods}
      {group: api version: v1 name: services}
      {group: api version: v1 name: replicationcontrollers}
      {group: api version: v1 name: deployments}
      {group: api version: v1 name: replicasets}
      {group: api version: v1 name: statefulsets}
      {group: api version: v1 name: daemonsets}
    ]
  }

  # if $watch {
  #   let res = $resources | first
  #   return (watch-resource $res $resourcename
  #     -n $namespace 
  #     -g $res.group 
  #     -v $res.version 
  #     -l $labels
  #     -c $conf
  #     -C $context
  #     -o $output
  #     --all=$all
  #   ) 
  # }

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
      $acc | merge { $resource.name: ($r | fmt $resource -o $output -d $decorators) }
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
