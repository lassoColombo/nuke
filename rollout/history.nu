use "../config"
use "../api"
use "../show/get-resource.nu"
use "../show/show-completers.nu"
use "../fmt/fmt-completers.nu"
use "../fmt/helpers.nu"
use ./history-formatters

def fmt [
  resource: record
  obj: record
  parents: record
  --revision(-r): int
  --output(-o): string
] {
  let revision = $revision | default 0
  let formatter = $env.NUKE_ROLLOUTHISTORY_FORMATTERS?.apps?.v1? 
  | default {} 
  | get -o $resource.name
  | default {(history-formatters).apps.v1 | get -o $resource.name}
  do $formatter $obj ($parents.items | default []) $revision $output
}

export def main [
  resource: string@"api get-resource-completer"
  resourcename?: string@"show-completers resourcename"
  --namespace (-n): string@"show-completers namespace"
  --selector  (-l): string
  --revision  (-r): int            # Show detail for a specific revision
  --output    (-o): string@"fmt-completers output-no-wide"  # compact | wide (default: wide if --revision set, else compact)
  --kubeconf  (-K): record
  --kubeconfpath (-k): path
  --context   (-c): string@"config-completers context"
  --cluster   (-C): string@"config-completers cluster"
] {
  let kubeconf = if ($kubeconf | is-not-empty) { $kubeconf } else {
    config -k $kubeconfpath
  }

  # Resolve effective output mode:
  #   explicit --output flag  →  use it
  #   --revision passed       →  wide  (single-revision detail)
  #   otherwise               →  compact (list view)
  let output = if ($output | is-not-empty) {
    $output 
  } else if ($revision | is-not-empty) {
    "wide" 
  } else { 
    "compact" 
  }
  let resource = (api resolve-resource $resource)

  let obj = (
    get-resource $resource $resourcename
    -n $namespace
    -l $selector
    -K $kubeconf
    -c $context
    -C $cluster
  )

  # Normalise: if get-resource returned a List, take items; if single object, wrap
  let objects = if ($obj.items? | is-not-empty) { $obj.items } else { [$obj] }

  $objects | each {|o|
    match $o.kind? {
      "Deployment" => {
        let rs_resource = {
          name: "replicasets"
          group: "apps"
          version: "v1"
          namespaced: true
        }
        let rs = (
          get-resource $rs_resource
          -n ($o.metadata.namespace? | default $namespace)
          -K $kubeconf
          -c $context
          -C $cluster
        ) 
        fmt $resource $o $rs -r $revision -o $output
      }

      "DaemonSet" | "StatefulSet" => {
        let cr_resource = {
          name: "controllerrevisions"
          group: "apps"
          version: "v1"
          namespaced: true
        }
        let cr = (
          get-resource $cr_resource
          -n ($o.metadata.namespace? | default $namespace)
          -K $kubeconf
          -c $context
          -C $cluster
        )
        fmt {name: controllerrevisions} $o $cr -r $revision -o $output
      }

      _ => { error make --unspanned { msg: $"rollout history is not supported for ($o.kind? | default 'unknown') resources" } }
    }
  }
  | if ($objects | length) == 1 { first } else { $in }
}

