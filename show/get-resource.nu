use "./helpers.nu"
use "../http-get/"

export def --env main [

  resource: string
  resourcename?: string
  --group(-g): string
  --version(-v): string
  --namespace(-n):string
  --all-namespaces(-A)
  --selector(-l): string # filter resources by label

  --kubeconf(-K): any
  --context(-c): string
  --cluster(-C): string
] {
  let path = (helpers build-path 
    $resource
    $resourcename
    --group $group 
    --version $version
    --namespace $namespace
    --selector $selector
    --kubeconf $kubeconf
    --all-namespaces=$all_namespaces
  )
  return (http-get $path -K $kubeconf -c $context -C $cluster)
}
