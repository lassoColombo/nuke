use "../show"
use "../cfg"

def context-completer [] {
  cfg show | get contexts.name
}

def namespace-completer [] {
  show ns | get name
}

# switch the current namespace in your kubeconfig.
def set-namespace [
  namespace?:string@namespace-completer # target namespace
] {
  let update = {|namespace|
    let configuration = cfg show
    $configuration | update contexts (
      $configuration.contexts | each {|c|
        if ($c.name != $configuration.current-context) {
          $c
        } else {
          $c | update context ($c.context | upsert namespace $namespace)
        }
      }
    )
    | to yaml | save -f (cfg path)
    print $"(ansi cyan)switched to namespace ($namespace)(ansi reset)"
  }

  if ($namespace | is-not-empty) {
    do $update $namespace
    return
  }

  let namespace = namespace-completer | input list --fuzzy 'choose namespace: '
  if ( $namespace | is-empty ) {return}
  do $update $namespace
}

# switch the current context in your kubeconfig.
def --env set-context [context?: string@context-completer] {
  let update = {|context|
    cfg show | upsert current-context $context | to yaml | save -f (cfg path)
    print $"(ansi cyan)switched to context ($context)(ansi reset)"
  }
  if ($context | is-not-empty) {
    do $update $context
    return
  }

  let context = context-completer | input list --fuzzy 'choose context: '
  if ( $context | is-empty ) {return}
  do $update $context
}

export def main [
  context?: string@context-completer
  --namespace(-n): string@namespace-completer
] {
  if ($context | is-not-empty) {
    set-context $context
  }
  if ($namespace | is-not-empty) {
    set-namespace $namespace
  }
}
