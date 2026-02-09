export def main [output?: string = compact] {
  let sa = $in
  {
    name: $sa.metadata.name
    secrets: ($sa.secrets? | default [] | get -o name)
    age: ($sa.metadata.creationTimestamp? | helpers fmtage)
  }
}
