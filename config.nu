export def read [] {
  let confpath = $env.KUBECONFIG? | default $"($env.HOME)/.kube/config"
  let conf = open $confpath | from yaml
  let context = $conf | get current-context
  let namespace = $conf.contexts 
  | where name == $context 
  | first 
  | get -o context.namespace 
  | default 'default'
  {
    config: $conf
    context: $context
    namespace: $namespace
  }
}
