use "./helpers.nu"
use "../http-get/"

export def main [
  resource: record
  resourcename?: string
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
    --namespace $namespace
    --selector $selector
    --kubeconf $kubeconf
    --all-namespaces=$all_namespaces
  )
  return (http-get $path -K $kubeconf -c $context -C $cluster)
}
