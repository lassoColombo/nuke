export def "events v1" [output: string = compact] {
  let ev = $in
  let res = {
    last-seen: ($ev.lastTimestamp? | default (0 | into datetime -f "%s") | into datetime)
    type: $ev.type
    reason: $ev.reason
    object: $'($ev.involvedObject? | default {} | get -o kind | default '' | str downcase)/($ev.involvedObject? | default {} | get -o name)'
    message: $ev.message?
  }

  if ($output | is-empty) or $output == compact {
    return  $res
  } 
  $res
  | insert source ($ev.source?.component?)
  | insert first-seen ($ev.firstTimestamp?)
  | insert count ($ev.count?)
  | insert name ($ev.metadata?.name?)
}
