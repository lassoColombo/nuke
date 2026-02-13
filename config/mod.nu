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

# -----------
#  current   
# -----------

# returns the current context from the kubeconfig.
export def get-current-context [conf?] {
  let conf = if ($conf | is-not-empty) {$conf} else {main}
  $conf.contexts
  | where name == $conf.current-context 
  | if ($in | is-empty) {[{}]} else {$in}
  | first 
}

# returns the current namespace from the kubeconfig.
export def get-current-namespace [conf?] {
  let conf = if ($conf | is-not-empty) {$conf} else {main}
  $conf.contexts 
  | where name == $conf.current-context 
  | if ($in | is-empty) {[{}]} else {$in}
  | first 
  | get -o context.namespace
  | default 'default'
}

# -------
#  get   
# -------
def _get [
  resource: string
  resourcename?: string
] {
  let l = main | get -o $resource
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
] {
  _get 'contexts' $context
}

# get configured clusters
# returns all configured clusters, or a specific cluster if a name is provided.
export def get-clusters [
  cluster?: string@cluster-completer,
] {
  _get 'clusters' $cluster
}

# get configured users
# returns all configured users, or a specific user if a name is provided.
export def get-users [
  user?: string@user-completer,
] {
  _get 'users' $user
}
