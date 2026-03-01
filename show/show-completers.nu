use "../api"
use "../config"
use "../fmt"
use ./get-resource.nu

export def resourcename [context?: string] {
  if ($context | is-empty) {
    return []
  } 
  mut prev = $context | parse --regex '(?P<word>\S+)' | get word

  let idx = $prev | enumerate | where {$in.item in ['-c', '--context', '-C', '--cluster']} | get index
  let current_context = if ($idx | is-not-empty) {
    $prev | get (($idx | first) + 1)
  } else {
    ''
  }

  let idx = $prev | enumerate | where {$in.item in ['-K', '--kubeconf']} | get index
  let kubeconf = if ($idx | is-not-empty) {
    $prev | get (($idx | first) + 1)
  } else {
    {}
  }

  let idx = $prev | enumerate | where {$in.item in ['-n', '--namespace']} | get index
  let namespace = if ($idx | is-not-empty) {
    $prev | get (($idx | first) + 1)
  } else {
    ''
  }

  let resources = api resources -K $kubeconf -c $current_context -o wide | get -o names | flatten
  let resource = $prev | get (($prev | enumerate | where {|arg| $arg.item in $resources} | first | get index))

  get-resource $resource -K $kubeconf -n $namespace -c $current_context
  | get items.metadata.name
}

export def namespace [context?: string] {
  "here" | save -f test.yaml
  if ($context | is-empty) {
    return []
  } 
  mut prev = $context | parse --regex '(?P<word>\S+)' | get word
  let idx = $prev | enumerate | where {$in.item in ['-c', '--context', '-C', '--cluster']} | get index
  let current_context = if ($idx | is-not-empty) {
    $prev | get (($idx | first) + 1)
  } else {
    ''
  }
  if ($idx | is-not-empty) {
    $prev = $prev | reject ($idx | first) (($idx | first) + 1)
  }

  get-resource --context $current_context namespaces | get items.metadata.name
}
