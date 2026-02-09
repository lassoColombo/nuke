export def main [output?: string = compact] {
  let r = $in
  let res = {
    name: $r.metadata.name
    created: $r.metadata.creationTimestamp
    rules: ($r.rules? | default [] | length)
  }
  if ($output | is-empty) or $output == compact {
    $res
  } else {
    $res
    | upsert aggregationRule ($r.aggregationRule?.clusterRoleSelectors?)
    | update rules ($r.rules)
  }
}
