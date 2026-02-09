export def main [output?: string = compact] {
  let ns = $in
  {
    name: $ns.metadata.name
    status: $ns.status.phase?
    age: ($ns.metadata.creationTimestamp? | helpers fmtage)
  }
}
