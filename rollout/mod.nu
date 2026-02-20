use "../config"
use "../api"
use "../fmt"
use "../show/get-resource.nu"
use "../show/watch-resource.nu"
use "../show/show-completers.nu"

export def --env status [
  resource: string@"api resource-completer"
  resourcename: string@"show-completers resourcename"
  --namespace(-n): string@"show-completers namespace"
  # --watch(-w)
] {
  let conf = config
  let resource = if ($resource | str contains /) {
    $resource | split column -n 3 / group version name | first
  } else {
    let res = api resources -o wide $resource
    if ($res | length) == 0 {
      error make --unspanned {
        msg: $"($resource) is not a resource from the cluster. Run 'nuke api-resources | get names | flatten' to get the full list"
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

  let fmt_suffix = $"RolloutStatus"
  $res.kind = $"($res.kind)($fmt_suffix)"

  # if $watch {
  #   (
  #     watch-resource $res
  #     -n $namespace 
  #     -g $resource.group 
  #     -v $resource.version 
  #     -c $conf
  #     -s $fmt_suffix
  #   ) | ignore
  #   return
  # }

  $res | fmt $resource
}


export def --env history [
  resource: string@"api resource-completer"
  resourcename: string@"show-completers resourcename"
  --namespace(-n): string@"show-completers namespace"
  # --watch(-w)
] {
  let conf = config
  let resource = if ($resource | str contains /) {
    $resource | split column -n 3 / group version name | first
  } else {
    let res = api resources -o wide $resource
    if ($res | length) == 0 {
      error make --unspanned {
        msg: $"($resource) is not a resource from the cluster. Run 'nuke api-resources | get names | flatten' to get the full list"
      }
    } 
    $res | first | select group version name
  }

  let obj = (get-resource $resource.name $resourcename 
    -n $namespace 
    -g $resource.group 
    -v $resource.version 
    -c $conf
  )

  mut revisions = []

  if $obj.kind == "Deployment" {
    let rs = (get-resource "replicasets"
      -n $namespace
      -g "apps"
      -v "v1"
      -c $conf
    )

    return (
      $rs.items
      | where {|r| 
        $r.metadata.ownerReferences?
        | any {|o| $o.uid == $obj.metadata.uid }
      }
      | each {|r|
        {
          revision: ($r.metadata.annotations."deployment.kubernetes.io/revision"?)
          changeCause: ($r.metadata.annotations."kubernetes.io/change-cause"?)
          created: ( $r.metadata.creationTimestamp | into datetime )
        }
      }
      | sort-by revision
    )
  } 

  let cr = (get-resource "controllerrevisions"
    -n $namespace
    -g "apps"
    -v "v1"
    -c $conf
  )

  return (
    $cr.items
    | where {|r|
      $r.metadata.ownerReferences?
      | any {|o| $o.uid == $obj.metadata.uid }
    }
    | each {|r|
      {
        revision: $r.revision
        created: ( $r.metadata.creationTimestamp | into datetime )
      }
    }
    | sort-by revision
  )
}
