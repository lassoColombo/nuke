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

# opens the kubeconfig file in your default editor.
export def edit [] {
  nu -c $"($env.EDITOR) (path)"
}

# --------------
#  completers   
# --------------
def context-completer [] { main | get contexts.name }
def context-key-completer [] {[cluster namespace user]}
def cluster-completer [] { main | get clusters.name }
def cluster-key-completer [] {[server]}
def user-completer [] { main | get users.name }
def user-key-completer [] {[token]}


# -------
#  get   
# -------
def _get [
  resource: string
  resourcename?: string
  --current
] {
  let cfg = main
  let resourcename = $resourcename | default $cfg.current-context
  let l = $cfg | get -o $resource
  if ($resourcename | is-empty) {
    return $l
  }
  $l
  | where name == $resourcename
  | first
}

# get configured contexts
# returns all configured contexts, or a specific context if a name is provided.
export def get-contexts [
  context?: string@context-completer
  --current
] {
  _get 'contexts' $context --current=$current
}

# get configured clusters
# returns all configured clusters, or a specific cluster if a name is provided.
export def get-clusters [
  cluster?: string@cluster-completer,
  --current
] {
  _get 'clusters' $cluster --current=$current
}

# get configured users
# returns all configured users, or a specific user if a name is provided.
export def get-users [
  user?: string@user-completer,
  --current
] {
  _get 'users' $user --current=$current
}

# returns the current namespace from the kubeconfig.
export def get-current-namespace [conf?] {
  get-contexts --current 
  | get -o context.namespace 
  | default 'default'
}

