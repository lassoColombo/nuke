export def main [output: string = compact] {
  let dc = $in

  let selectors = ($dc.spec.selectors? | default [])

  let selector_types = (
    $selectors
    | each {|s|
      $s | columns | first
    }
    | uniq
  )

  let res = {
    name: $dc.metadata.name
    resource: ($dc.spec.extendedResourceName?)
    selectors: ($selectors | length)
    selector-types: $selector_types
    age: ($dc.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert generation ($dc.metadata.generation?)
  | upsert selectors (
    $selectors
    | each {|s|
      if ($s.cel? != null) {
        {
          type: "cel"
          expression: $s.cel.expression?
        }
      } else {
        {
          type: ($s | columns | first)
          value: ($s | values | first)
        }
      }
    }
  )
}
