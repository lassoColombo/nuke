use ./config-completers.nu

# Returns the path to the kubeconfig file.
export def path [] {
  config-completers path
}

# Loads and returns the kubeconfig as a record.
export def main [] {
  config-completers load
}

# Opens the kubeconfig file in your default editor.
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
  let l = $cfg | get -o $resource

  if $current {
    if ($resourcename | is-not-empty) {
      error make --unspanned {
        msg: $"you can either specify a name from the ($resource) list or --current, not both."
      }
    }
  } else {
    if ($resourcename | is-empty) {return $l}
  }

  let resourcename = $resourcename | default $cfg.current-context
  if ($resourcename | is-empty) {
    return $l
  }
  $l
  | where name == $resourcename
  | first
}

# Returns all configured contexts, or a specific context if a name is provided.
export def get-contexts [
  context?: string@"config-completers context" # The context to get.
  --current # Get current context.
] {
  _get 'contexts' $context --current=$current
}

# Returns all configured clusters, or a specific cluster if a name is provided.
export def get-clusters [
  cluster?: string@"config-completers cluster" # The cluster to get.
  --current # Get current cluster
] {
  _get 'clusters' $cluster --current=$current
}

# Returns all configured users, or a specific user if a name is provided.
export def get-users [
  user?: string@"config-completers user" # The user to get.
  --current # Get current user.
] {
  _get 'users' $user --current=$current
}

# Returns the current namespace from the kubeconfig.
export def get-current-namespace [] {
  get-contexts --current 
  | get -o context.namespace 
  | default 'default'
}

