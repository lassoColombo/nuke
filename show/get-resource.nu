use "./helpers.nu"
use "../http-get/"

export def --env main [
  resource: string
  resourcename?: string
  --selector(-l): string # filter resources by label
  --group(-g): string
  --version(-v): string
  --namespace(-n):string
  --conf(-c): any
  --context(-C): string
  --all-namespaces(-A)
] {
  let path = (helpers build-path 
    $resource
    $resourcename
    --group $group 
    --version $version
    --namespace $namespace
    --selector $selector
    --conf $conf
    --all-namespaces=$all_namespaces
  )
  return (http-get $path $conf -c $context)
}
