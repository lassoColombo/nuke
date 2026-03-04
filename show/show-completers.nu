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
    null
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
    null
  }

  let resources = api resources -K $kubeconf -c $current_context -o wide
  let namelist = $resources | get -o name shortNames singularName kind | flatten | flatten | compact | each {$in | str downcase}
  let needle = $prev | get (($prev | enumerate | where {|arg| $arg.item in $namelist} | first | get index))

  let resource = (
    $resources | where {|r|
      let names = ($r.names? | default [])
      let short = ($r.shortNames? | default [])
      let singular = ($r.singularName? | default "")
      let kind = ($r.kind? | default "")

      (
        $needle == ($r.name | str downcase)
        or $needle in ($names | each {|n| $n | str downcase })
        or $needle in ($short | each {|s| $s | str downcase })
        or $needle == ($singular | str downcase)
        or $needle == ($kind | str downcase)
      )
    }
  ) | first

  get-resource $resource -K $kubeconf -n $namespace -c $current_context | get items.metadata.name
}

export def namespace [context?: string] {
  if ($context | is-empty) {
    return (get-resource {name: namespaces, group: api, version: v1 namespaced: false} | get items.metadata.name)
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

  get-resource --context $current_context {name: namespaces, group: api, version: v1 namespaced: false} | get items.metadata.name
}
