use "../config"
use "../api"
use "../fmt"
use "../show/get-resource.nu"
use "../show/show-completers.nu"
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

  $res.kind = $"($res.kind)Rollout($action | str capitalize)"
  $res | fmt resource
}

# view previous rollout revisions and configurations
export def history [
  resource: string@"show-completers api-resource" # the resource you want to get (po, deploy etc)
  resourcename: string@"show-completers resourcename" # the name of the resource you want to get
  --namespace(-n): string@"show-completers namespace" # the namespace you want to get your resource(s) from
] {
  (_rollout 
    history 
    $resource
    $resourcename
    -n $namespace
  )
}

# view the status of the rollout
export def status [
  resource: string@"show-completers api-resource" # the resource you want to get (po, deploy etc)
  resourcename: string@"show-completers resourcename" # the name of the resource you want to get
  --namespace(-n): string@"show-completers namespace" # the namespace you want to get your resource(s) from
] {
  (_rollout 
    status 
    $resource
    $resourcename
    -n $namespace
  )
}
