export def main [output: string = compact] {
  let ev = $in
  let res = {
    last-seen: $ev.lastTimestamp
    type: $ev.type
    reason: $ev.reason
    object: $'($ev.involvedObject | get kind | str downcase)/($ev.involvedObject | get name)'
    message: $ev.message
  }

  if ($output | is-empty) or $output == compact {
    return  $res
  } 
  $res
  | insert source ($ev.source.component)
  | insert first-seen ($ev.firstTimestamp)
  | insert count ($ev.count)
  | insert name ($ev.metadata.name)
}
