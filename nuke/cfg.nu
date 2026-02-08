export def path [] {
  if ($env.KUBECONFIG? | is-not-empty) {
     $env.KUBECONFIG
  } else {
    [$env.HOME .kube config] | path join
  }
}

export def show [--current] {
  let configuration = open -r (path) | from yaml
  if not $current {
    return $configuration
  }
}

export def current-namespace [conf?] {
  let conf = if ($conf | is-not-empty) {$conf} else {show}
  $conf.contexts 
  | where name == $conf.current-context 
  | first 
  | get -o context.namespace
  | default 'default'
}

export def current-context [conf?] {
  if ($conf | is-not-empty) {$conf} else {show}
  | get -o current-context
}

export def edit [] {
  nu -c $"($env.EDITOR) (path)"
}

