
# ------
#  v1   
# ------

export def "customresourcedefinitions v1" [ output?: string = compact ] {
  let crd = $in
  let versions = ($crd.spec.versions? | default [])
  let storage_version = (
    $versions
    | where storage == true
    | get name
    | first
  )

  let established_cond = (
    $crd.status.conditions?
    | default []
    | where type == Established
    | first
    | default {}
  )

  let res = {
    name: $crd.metadata.name
    group: $crd.spec.group
    kind: $crd.spec.names.kind
    scope: $crd.spec.scope
    version: $storage_version
    established: ($established_cond.status?)
    age: ($crd.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert versions (
    $versions
    | select -o name served storage
  )
  | upsert names (
    $crd.spec.names
    | select -o plural singular kind listKind
  )
  | upsert printerColumns (
    $versions
    | where name == $storage_version
    | get additionalPrinterColumns?
    | first
  )
  | upsert conditions ($crd.status.conditions?)
}
