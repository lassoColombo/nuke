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
  $configuration.contexts | where {|c|
    $c.name == $configuration.current-context
  }
  | first
}


export def edit [] {
  nu -c $"($env.EDITOR) (path)"
}

