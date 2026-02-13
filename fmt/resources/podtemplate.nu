export def main [output?: string = compact] {
  let pt = $in
  {
    name: $pt.metadata.name
    containers: ( $pt.template.spec.containers | select -o name image )
    pod-labels: ( $pt.template.metadata.labels?)
    restart-policy: ( $pt.template.spec.restartPolicy )
  }
}
