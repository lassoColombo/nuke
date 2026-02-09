export def main [output?: string = compact] {
  let lr = $in

  let limits = ($lr.spec.limits? | default [])

  let types = (
    $limits
    | get type
    | uniq
  )

  let resources = (
    $limits
    | each {|l|
      [
        ($l.min? | default {} | columns)
        ($l.max? | default {} | columns)
        ($l.default? | default {} | columns)
        ($l.defaultRequest? | default {} | columns)
      ]
      | flatten
    }
    | flatten
    | uniq
  )

  let res = {
    name: $lr.metadata.name
    namespace: $lr.metadata.namespace?
    types: $types
    resources: $resources
    age: ($lr.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert limits (
    $limits
    | each {|limit|
      {
        type: $limit.type
        min: ($limit.min? | helpers fmtresources)
        max: ($limit.max? | helpers fmtresources)
        default: ($limit.default? | helpers fmtresources)
        defaultRequest: ($limit.defaultRequest? | helpers fmtresources)
      }
    }
  )
}
