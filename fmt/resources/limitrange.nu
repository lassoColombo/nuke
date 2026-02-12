export def fmtresources [] {
  $in 
  | transpose kind amount
  | each {|l|
    $l | update amount ($l.amount | into filesize)} 
  | reduce --fold {} {|elt acc| 
    $acc | merge {$elt.kind: $elt.amount}
  }
}

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
        min: ($limit.min? | fmtresources)
        max: ($limit.max? | fmtresources)
        default: ($limit.default? | fmtresources)
        defaultRequest: ($limit.defaultRequest? | fmtresources)
      }
    }
  )
}
