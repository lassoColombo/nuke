export def main [output?: string = compact] {
  let r = $in
  let res = {
    name: $r.metadata.name
    created: $r.metadata.creationTimestamp
    rules: ($r.rules? | default [] | length)
  }
  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | update rules ($r.rules)
}
