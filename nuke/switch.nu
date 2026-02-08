use ./show.nu
use ./cfg.nu

def context-completer [] {
  cfg show | get contexts.name
}

def namespace-completer [] {
  show ns | get name
}

export def namespace [namespace?:string@namespace-completer] {
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

export def --env context [context?: string@context-completer] {
  let update = {|context|
    cfg show | update current-context $context | to yaml | save -f (cfg path)
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
