use ./fmt
use ./cfg.nu
use ./call.nu
use ./api.nu
use ./fmt/formatters.nu

export def supported-formatters [] {
  formatters | transpose formatter closure | get formatter
}

def --env get-resource [
  resource: string
  --group(-g): string
  --version(-v): string
  --namespace(-n):string
  --resourcename(-N):string
  --conf(-c): any
  --all(-A)
] {
  let conf = $conf | default (cfg show)
  let namespace = if ($namespace | is-not-empty) {$namespace} else {cfg current-namespace $conf} 
  let resourcename = if ($resourcename | is-not-empty) {$resourcename} else { '' } 

  let resource = if ($group | is-not-empty) and ($version | is-not-empty) {
    {group: $group version: $version name: $resource}
    api resources -o wide
    | where {$resource in $in.names} 
    | first
    | upsert group $group
    | upsert version $version
    | select group version name namespaced
  } else if ($group | is-empty) and ($version | is-empty) {
    api resources -o wide
    | where {$resource in $in.names?} 
    | first
    | select group version name namespaced
  }
  let prefix = if $resource.group == "api" and $resource.version == "v1" {
    "api/v1"
  } else {
    $"apis/($resource.group)/($resource.version)"
  }

  let path = if $resource.namespaced {
    if $all {
      $"($prefix)/($resource.name)"
    } else if ($resourcename | is-empty) {
      $"($prefix)/namespaces/($namespace)/($resource.name)"
    } else {
      $"($prefix)/namespaces/($namespace)/($resource.name)/($resourcename)"
    }
  } else {
    if ($resourcename | is-empty) {
      $"($prefix)/($resource.name)"
    } else {
      $"($prefix)/($resource.name)/($resourcename)"
    }
  }

  return (call $conf $path)
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
  --watch-interval(-W): duration = 5sec # set the refresh interval for the --watch option
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

  mut res = (get-resource $resource.name 
    -n $namespace 
    -g $resource.group 
    -v $resource.version 
    -N $resourcename 
    -c $conf
    --all=$all
  | fmt resource -o $output -d $decorators )

  if not $watch {
    return $res
  }

  loop {
    clear
    print ($res | table -e)
    sleep $watch_interval
    $res = (get-resource $resource.name
      -n $namespace 
      -g $resource.group 
      -v $resource.version 
      -N $resourcename 
      -c $conf
      --all=$all
    | fmt resource -o $output -d $decorators )
  }

}
