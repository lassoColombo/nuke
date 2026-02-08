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
  let context = $configuration.contexts | where {|c|
    $c.name == $configuration.current-context
  } | first
  let cluster = $configuration.clusters | where {|c|
    $c.name == $configuration.current-context
  } | first
  let user = $configuration.users | where {|c|
    $c.name == $configuration.current-context
  } | first
  {
    context: $context
    cluster: $cluster
    user: $user
  }
}

export def current-namespace [conf?] {
  let conf = if ($conf | is-not-empty) {$conf} else {cfg show}
  $conf.contexts 
  | where name == $conf.current-context 
  | first 
  | get -o context.namespace
  | default 'default'
}

export def edit [] {
  nu -c $"($env.EDITOR) (path)"
}

