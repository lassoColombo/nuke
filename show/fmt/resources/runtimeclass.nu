export def main [output?: string = compact] {
  let rc = $in

  let res = {
    name: $rc.metadata.name
    handler: $rc.handler
    age: ($rc.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert generation ($rc.metadata.generation?)
}
