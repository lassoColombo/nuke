export def main [output: string = compact] {
  let ep = $in
  {
    name: $ep.metadata.name
    address-type: $ep.addressType
    ports: $ep.ports
    endpoints: ($ep.endpoints? | default [] | get addresses | flatten)
    age: ($ep.metadata.creationTimestamp? | helpers fmtage)
  }
}
