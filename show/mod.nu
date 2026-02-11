use "../fmt"
use "../cfg"
use "../api"
use "../fmt/formatters.nu"
use ./get-resource.nu
use ./watch-resource.nu

# lists supported resource formatters
export def supported-formatters [] {
  formatters | transpose formatter closure | get formatter
}

def api-resource-completer [context: string] {
  api resources -o wide | get -o names | flatten
}

def resourcename-completer [context: string] {
  if ($context | is-empty) {
    return []
  } 
  mut prev = $context | parse --regex '(?P<word>\S+)' | get word

  let idx = $prev | enumerate | where {$in.item == '-n'} | get index
  let namespace = if ($idx | is-not-empty) {
    $prev | get (($idx | first) + 1)
  } else {
    ''
  }
  if ($idx | is-not-empty) {
    $prev = $prev | reject ($idx | first) (($idx | first) + 1)
  }

  let resources = api resources -o wide | get -o names | flatten
  let resource = $prev | get (($prev | enumerate | where {|arg| $arg.item in $resources } | first | get index))

  get-resource $resource -n $namespace 
  | get items 
  | get metadata.name
}

def namespace-completer [context: string] {
  get-resource namespaces | get items.metadata.name
}

def output-completer [context: string] {
  fmt supported-outputs
}

# displays the specified kubernetes resources
export def --env main [
  resource: string@api-resource-completer # the resource you want to get (po, deploy etc)
  resourcename?: string@resourcename-completer # the name of the resource you want to get
  --namespace(-n): string@namespace-completer # the namespace you want to get your resource(s) from
  --all(-A) # get all the specified resources
  --output(-o): string@output-completer # the format of the output
  --show-annotations(-a) # appends the object's annotations to the output
  --show-labels(-l) # appends the object's labels to the output
  --show-conditions(-c) # appends the object's conditions to the output
  --watch(-w) # watch the required objects for changes (early implementation)
  --interval(-I): duration = 5sec # set the refresh interval for the --watch option
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

  let conf = cfg show
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

  let namespace = if ($namespace | is-not-empty) {
    $namespace
  } else if $all {
    ''
  } else {
    cfg current-namespace $conf
  }

  let decorators = [
    ...(if $all {['namespace']} else {[]})
    ...(if $show_labels {['labels']} else {[]})
    ...(if $show_annotations {['annotations']} else {[]})
    ...(if $show_conditions {['conditions']} else {[]})
  ]

  mut res = (get-resource $resource.name $resourcename 
    -n $namespace 
    -g $resource.group 
    -v $resource.version 
    -c $conf
    --all=$all
  )

  if not $watch {
    return ($res | fmt resource -o $output -d $decorators)
  }

  ( 
    watch-resource $resource.name $resourcename 
    -n $namespace 
    -g $resource.group 
    -v $resource.version 
    -c $conf
    -o $output
    -d $decorators
    --all=$all
  )
}
