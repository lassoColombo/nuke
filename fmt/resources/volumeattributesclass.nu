export def main [output?: string = compact] {
  let vac = $in

  let params = ($vac.parameters? | default {})

  let res = {
    name: $vac.metadata.name
    driver: ($vac.driverName?)
    parameters: ($params | columns | length)
    age: ($vac.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  $res
  | upsert parameters $params
  | upsert finalizers ($vac.metadata.finalizers? | default [])
  | upsert created (
    if ($vac.metadata.creationTimestamp? | is-not-empty) {
      $vac.metadata.creationTimestamp | into datetime
    } else {
      null
    }
  )
}
