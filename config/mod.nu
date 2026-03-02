use ./config-completers.nu

# Returns the path to the kubeconf file.
export def path [] {
  config-completers path
}

# Loads and returns the kubeconf as a record.
export def main [--kubeconfpath(-k): path] {
  config-completers load --kubeconfpath $kubeconfpath
}

# Opens the kubeconf file in your default editor.
export def edit [--kubeconfpath: path] {
  nu -c $"($env.EDITOR) ($kubeconfpath | default (path))"
}

# -------
#  get   
# -------

def resolve-selection [
  items: list
  explicit_name?: string
  --select-one
  --default-name-resolver: closure
] {
  if $select_one {
    if ($explicit_name | is-not-empty) {
      error make --unspanned {
        msg: "you can either specify a name or use a selection flag, not both."
      }
    }
  } else {
    if ($explicit_name | is-empty) { 
      return $items 
    }
  }

  let target_name = $explicit_name | default { do $default_name_resolver }

  if ($target_name | is-empty) {
    return $items
  }

  $items
  | where name == $target_name
  | first
}

def get-resources [
  resource_spec: record
  resourcename?: string
  --kubeconf(-K): record
  --current
  --context: string
  --current-resolver: closure
] {
  let cfg = if ($kubeconf | is-not-empty) { $kubeconf } else { main }

  if ($current and ($context | is-not-empty)) {
    error make --unspanned {
      msg: "you cannot use --current and --context together."
    }
  }

  let list = $cfg | get -o $resource_spec.plural

  mut effective_resolver = {||}
  if ($context | is-not-empty) {
    $effective_resolver = {||
      get-contexts $context -K $cfg 
      | get context
      | get $resource_spec.singular
    }
  } else {
    $effective_resolver = $current_resolver
  }

  let should_select = $current or ($context | is-not-empty)

  (
    resolve-selection $list $resourcename 
    --select-one=$should_select
    --default-name-resolver $effective_resolver
  )
}

# Returns all configured contexts, or a specific context if a name is provided.
export def get-contexts [
  context?: string@"config-completers context"
  --kubeconf(-K): record
  --current
] {
  let cfg = if ($kubeconf | is-not-empty) { $kubeconf } else { main }
  let list = $cfg | get -o contexts

  (
    resolve-selection $list $context 
    --select-one=$current 
    --default-name-resolver { $cfg.current-context }
  )
}

# Returns all configured clusters, or a specific cluster if a name is provided.
export def get-clusters [
  cluster?: string@"config-completers cluster"
  --context: string@"config-completers context" # Get the cluster of the given context.
  --current # Get the cluster of the current context.
  --kubeconf(-K): record
] {
  (
    get-resources {plural: clusters, singular: cluster} $cluster 
    --current=$current 
    --context $context
    --kubeconf $kubeconf 
    --current-resolver { (get-contexts --current -K $kubeconf).context.cluster }
  )
}

# Returns all configured users, or a specific user if a name is provided.
export def get-users [
  user?: string@"config-completers user" # The name of the user to get.
  --context: string@"config-completers context" # Get the user of the given context.
  --current # Get the user of the current context.
  --kubeconf(-K): record 
] {
  (
    get-resources {plural: users, singular: user} $user 
    --current=$current 
    --context $context
    --kubeconf $kubeconf 
    --current-resolver { (get-contexts --current -K $kubeconf).context.user }
  )
}

# Returns the current namespace from the kubeconf.
export def get-current-namespace [] {
  get-contexts --current 
  | get -o context.namespace 
  | default 'default'
}

