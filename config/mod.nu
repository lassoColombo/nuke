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

# -----------
#  current   
# -----------

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

# ------------
#  contexts   
# ------------

def context-completer [] { main | get contexts.name }
# list configured contexts
export def get-contexts [
  context?: string@context-completer
] {
  let contexts = main | get -o contexts
  if ($context | is-empty) {
    return $contexts
  }
  $contexts 
  | where name == $context 
  | first
}

export def delete-context [
  context: string@context-completer,
] {
  let conf = main
  $conf 
  | upsert contexts (
    $conf
    | get -o contexts
    | default []
    | where name != $context
  )
  | to yaml
  | save -f (path)
}

# ------------
#  clusters   
# ------------

def cluster-completer [] { main | get clusters.name }
# list configured clusters
export def get-clusters [
  cluster?: string@cluster-completer,
] {
  let clusters = main | get -o clusters
  if ($cluster | is-empty) {
    return $clusters
  }
  $clusters 
  | where name == $cluster 
  | first
}

export def delete-cluster [
  cluster: string@cluster-completer,
] {
  let conf = main
  $conf 
  | upsert clusters (
    $conf
    | get -o clusters
    | default []
    | where name != $cluster
  )
  | to yaml
  | save -f (path)
}

# ---------
#  users   
# ---------

def user-completer [] { main | get users.name }
# list configured users
export def get-users [
  user?: string@user-completer,
] {
  let users = main | get -o users
  if ($user | is-empty) {
    return $users
  }
  $users 
  | where name == $user 
  | first
}

export def delete-user [
  user: string@user-completer,
] {
  let conf = main
  $conf 
  | upsert users (
    $conf
    | get -o users
    | default []
    | where name != $user
  )
  | to yaml
  | save -f (path)
}
