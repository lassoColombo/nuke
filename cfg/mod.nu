# returns the path to the kubeconfig file.
export def path [] {
  if ($env.KUBECONFIG? | is-not-empty) {
     $env.KUBECONFIG
  } else {
    [$env.HOME .kube config] | path join
  }
}

# loads and returns the kubeconfig as a record.
export def show [] {
  open -r (path) | from yaml
}

# returns the current namespace from the kubeconfig.
export def current-namespace [conf?] {
  let conf = if ($conf | is-not-empty) {$conf} else {show}
  $conf.contexts 
  | where name == $conf.current-context 
  | first 
  | get -o context.namespace
  | default 'default'
}

# returns the current context name from the kubeconfig.
export def current-context [conf?] {
  if ($conf | is-not-empty) {$conf} else {show}
  | get -o current-context
}

# opens the kubeconfig file in your default editor.
export def edit [] {
  nu -c $"($env.EDITOR) (path)"
}
