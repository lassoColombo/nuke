export def main [output?: string = compact] {
  let sec = $in
  {
    name: $sec.metadata.name
    data: ($sec.data? | default {} | transpose key value | length)
    age: ($sec.metadata.creationTimestamp? | helpers fmtage)
  }
}
