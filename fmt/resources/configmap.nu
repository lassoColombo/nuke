export def main [output?: string = compact] {
  let cm = $in
  {
    name: $cm.metadata.name
    data: ($cm.data? | default {} | transpose key value | length)
    age: ($cm.metadata.creationTimestamp? | helpers fmtage)
  }
}
