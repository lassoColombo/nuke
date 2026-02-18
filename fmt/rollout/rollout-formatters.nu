use ./apps-v1.nu

export def main [] {
  {
    apps: {
      v1: {
        daemonsets: {|output?: string = compact| apps-v1 daemonsets v1 $output}
        deployments: {|output?: string = compact| apps-v1 deployments v1 $output}
        replicasets: {|output?: string = compact| apps-v1 replicasets v1 $output}
        statefulsets: {|output?: string = compact| apps-v1 statefulsets v1 $output}
      }
    }
  }
}
