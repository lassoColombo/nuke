export use ./config
use ./show
use ./config/config-completers.nu
use ./show/show-completers.nu

use ./api
export module ./http-get
export module ./rollout
export module ./top

# Gets kubernetes resources.
export alias get = show
# Lists all API resources available in the cluster.
export alias api-resources = api resources
# Lists all API versions available in the cluster.
export alias api-versions = api versions

# Switches the current namespace in your kubeconfig.
export def "config switch-namespace" [
  namespace?:string@"show-completers namespace" # Target namespace.
] {
  let update = {|namespace|
    let configuration = config
    $configuration | update contexts (
      $configuration.contexts | each {|c|
        if ($c.name != $configuration.current-context) {
          $c
        } else {
          $c | update context ($c.context | upsert namespace $namespace)
        }
      }
    )
    | to yaml | save -f (config path)
    print $"(ansi cyan)switched to namespace ($namespace)(ansi reset)"
  }

  if ($namespace | is-not-empty) {
    do $update $namespace
    return
  }

  let namespace = show-completers namespace | input list --fuzzy 'choose namespace: '
  if ( $namespace | is-empty ) {return}
  do $update $namespace
}

# Switches the current context in your kubeconfig.
export def "config switch-context" [
  context?: string@"config-completers context" # Target context.
] {
  let update = {|context|
    config | upsert current-context $context | to yaml | save -f (config path)
    print $"(ansi cyan)switched to context ($context)(ansi reset)"
  }
  if ($context | is-not-empty) {
    do $update $context
    return
  }

  let context = config-completers context | input list --fuzzy 'choose context: '
  if ( $context | is-empty ) {return}
  do $update $context
}
