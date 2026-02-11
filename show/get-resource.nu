use "./helpers.nu"
use "../http-get/"

export def --env main [
  resource: string
  resourcename?: string
  --group(-g): string
  --version(-v): string
  --namespace(-n):string
  --conf(-c): any
  --all(-A)
] {
  let path = (helpers build-path 
    $resource
    $resourcename
    --group $group 
    --version $version
    --namespace $namespace
    --conf $conf
    --all=$all
  )
  return (http-get $path $conf)
}
