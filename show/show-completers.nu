use "../api"
use "../config"
use "../fmt"
use ./get-resource.nu

export def api-resource [context: string] {
  api resources -o wide | get -o names | flatten
}

export def resourcename [context: string] {
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

export def namespace [] {
  get-resource namespaces | get items.metadata.name
}
