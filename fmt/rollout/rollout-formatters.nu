use ./apps.nu

export def main [] {
  {
    apps: {
      v1: {
        daemonsets: {|output?: string = compact| apps daemonsets v1 $output}
        deployments: {|output?: string = compact| apps deployments v1 $output}
        replicasets: {|output?: string = compact| apps replicasets v1 $output}
        statefulsets: {|output?: string = compact| apps statefulsets v1 $output}
      }
    }
  }
}
