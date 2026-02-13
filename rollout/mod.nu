use "../config"
use "../api"
use "../show/fmt"
use "../show/get-resource.nu"
use "../show/completers.nu"
use ./formatters.nu

def --env _rollout [
  action: string
  resource: string
  resourcename: string
  --namespace(-n): string
] {
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
    -c $conf
  )

  let fmt = formatters | get -o $action
  if ($fmt | is-empty) {
    error make {
      msg: $'Supported rollout actions are (rollout-action-completer)'
      label: {
        text: $'($action) is not a supported action'
        span: (metadata $action).span
      }
    }
  }
  $res | do $fmt
}

export def history [
  resource: string@"completers api-resource" # the resource you want to get (po, deploy etc)
  resourcename: string@"completers resourcename" # the name of the resource you want to get
  --namespace(-n): string@"completers namespace" # the namespace you want to get your resource(s) from
] {
  (_rollout 
    history 
    $resource
    $resourcename
    -n $namespace
  )
}


export def status [
  resource: string@"completers api-resource" # the resource you want to get (po, deploy etc)
  resourcename: string@"completers resourcename" # the name of the resource you want to get
  --namespace(-n): string@"completers namespace" # the namespace you want to get your resource(s) from
] {
  (_rollout 
    status 
    $resource
    $resourcename
    -n $namespace
  )
}
