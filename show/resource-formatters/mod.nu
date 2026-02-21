use ./api.nu
use ./apps.nu
use "../../fmt/helpers.nu"

def default-formatter [output?: string = compact] {
  $in | helpers meta base
}

export def main [] {
  {
    default: {|output?: string = compact| default-formatter $output }
    api: {
      v1: {
        configmaps: {| output?: string = compact| api configmaps v1 $output }
        events: {|output?: string = compact| api events v1 $output }
    #     limitranges: {|output?: string = compact| api limitranges v1 $output }
        namespaces: {|output?: string = compact| api namespaces v1 $output }
        nodes: {|output?: string = compact| api nodes v1 $output }
        pods: {|output?: string = compact| api pods v1 $output }
        podtemplates: {|output?: string = compact| api podtemplates v1 $output }
    #     resourcequotas: {|output?: string = compact| api resourcequotas v1 $output }
        secrets: {|output?: string = compact| api secrets v1 $output }
        serviceaccounts: {|output?: string = compact| api serviceaccounts v1 $output }
        services: {|output?: string = compact| api services v1 $output }
      }
    }
    apps: {
      v1: {
        # controllerrevisions: {| output?: string = compact | apps controllerrevisions v1 $output }
        daemonsets: {|output?: string = compact| apps daemonsets v1 $output }
        deployments: {|output?: string = compact| apps deployments v1 $output }
        # replicasets: {|output?: string = compact| apps replicasets v1 $output }
        statefulsets: {|output?: string = compact| apps statefulsets v1 $output }
      }
    }
  }
}
