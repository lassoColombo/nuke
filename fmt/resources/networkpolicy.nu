export def main [output?: string = compact] {
  let np = $in
  {
    name: $np.metadata.name
    selector: $np.spec.podSelector.matchLabels
    ingress: ($np.spec.ingress? | default [] | length)
    egress: ($np.spec.egress? | default [] | length)
    age: ($np.metadata.creationTimestamp? | helpers fmtage)
  }
}
