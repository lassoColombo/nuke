use ./config-completers.nu

# returns the path to the kubeconfig file.
export def path [] {
  config-completers path
}

# loads and returns the kubeconfig as a record.
export def main [] {
  config-completers load
}

# opens the kubeconfig file in your default editor.
export def edit [] {
  nu -c $"($env.EDITOR) (path)"
}

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
  context?: string@"config-completers context"
  --current
] {
  _get 'contexts' $context --current=$current
}

# get configured clusters
# returns all configured clusters, or a specific cluster if a name is provided.
export def get-clusters [
  cluster?: string@"config-completers cluster"
  --current
] {
  _get 'clusters' $cluster --current=$current
}

# get configured users
# returns all configured users, or a specific user if a name is provided.
export def get-users [
  user?: string@"config-completers user"
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

