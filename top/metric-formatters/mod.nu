use ./api.nu

export def main [] {
  {
    metrics.k8s.io: {
      v1beta1: {
        nodes: {| output?: string = compact | api nodes v1 $output }
        pods: {| output?: string = compact | api pods v1 $output }
      }
    }
  }
}
