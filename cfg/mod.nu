# returns the path to the kubeconfig file.
export def path [] {
  if ($env.KUBECONFIG? | is-not-empty) {
     $env.KUBECONFIG
  } else {
    [$env.HOME .kube config] | path join
  }
}

# loads and returns the kubeconfig as a record.
export def main [] {
  open -r (path) | from yaml
}

# returns the current context from the kubeconfig.
export def current-context [conf?] {
  let conf = if ($conf | is-not-empty) {$conf} else {main}
  $conf.contexts
  | where name == $conf.current-context 
  | if ($in | is-empty) {[{}]} else {$in}
  | first 
}

# returns the current namespace from the kubeconfig.
export def current-namespace [conf?] {
  let conf = if ($conf | is-not-empty) {$conf} else {main}
  $conf.contexts 
  | where name == $conf.current-context 
  | if ($in | is-empty) {[{}]} else {$in}
  | first 
  | get -o context.namespace
  | default 'default'
}

# opens the kubeconfig file in your default editor.
export def edit [] {
  nu -c $"($env.EDITOR) (path)"
}
