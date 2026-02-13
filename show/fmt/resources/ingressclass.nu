export def main [output?: string = compact] {
  let ic = $in
  let params = ($ic.spec.parameters? | default {})

  let res = {
    name: $ic.metadata.name
    controller: $ic.spec.controller?
    parameters: (
      if ($params | is-empty) {
        null
      } else {
        $params | select -o kind name
      }
    )
    scope: ($params.scope? | default "Cluster")
    age: ($ic.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert apiGroup ($params.apiGroup?)
}
