export def main [output?: string = compact] {
  let svc = $in
  {
    name: $svc.metadata.name
    type: $svc.spec.type
    clusterIP: $svc.spec.clusterIP?
    age: ($svc.metadata.creationTimestamp? | helpers fmtage)
    ports: ($svc.spec.ports? | default [] | select -o protocol port targetPort)
    selector: ($svc.spec.selector? | default {} | transpose key value)
  }
}
