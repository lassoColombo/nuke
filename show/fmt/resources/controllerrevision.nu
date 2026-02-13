export def main [output?: string = compact ] {
  let cr = $in

  let owner = (
    $cr.metadata.ownerReferences? 
    | default [] 
    | where controller == true 
    | first
  )

  let controller_str = (
    if ($owner != null) {
      $'($owner.kind | str downcase).($owner.apiVersion)/($owner.name)'
    } else {
      null
    }
  )

  let res = {
    name: $cr.metadata.name
    controller: ($owner | default {} | select kind name)
    revision: $cr.revision?
    age: ($cr.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }
  $res
  | upsert generation ($cr.metadata.annotations.'deprecated.daemonset.template.generation'?)
  | upsert labels ($cr.metadata.labels?)
  | upsert container_names ($cr.data.spec.template.spec.containers? | get name)
  | upsert container_images ($cr.data.spec.template.spec.containers? | get image)
}
