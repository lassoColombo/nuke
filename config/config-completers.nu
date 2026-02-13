export def context [] { load | get contexts.name }
export def context-key [] {[cluster namespace user]}
export def cluster [] { load | get clusters.name }
export def cluster-key [] {[server]}
export def user [] { load | get users.name }
export def user-key [] {[token]}

# returns the path to the kubeconfig file.
export def path [] {
  if ($env.KUBECONFIG? | is-not-empty) {
     $env.KUBECONFIG
  } else {
    [$env.HOME .kube config] | path join
  }
}

# loads and returns the kubeconfig as a record.
export def load [] {
  open -r (path) | from yaml
}
