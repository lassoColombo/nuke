use ./helpers.nu
use ./decorators.nu
use ./resource-formatters
use ./rollout-formatters
use ./metric-formatters

# lists supported resource formatters
export def supported-formatters [] {
  formatters | transpose formatter closure | get formatter
}

export def main [
  resource_spec: record
  --output(-o): string
  --suffix(-s): string
  --decorators(-d): list<string> = []
] {
  let c = $in
  let content = $c | update kind (
    if ($suffix | is-not-empty) {$"($c.kind)($suffix)"} else {$c.kind}
  )

  if ($output == full) { return $content } 

  let many = $content.kind | str ends-with "List"
  let fmtclosure = $env.NUKE_FORMATTERS? 
  | default {}
  | get -o $resource_spec.group 
  | get -o $resource_spec.version 
  | get -o $resource_spec.name 
  | default {(
    if ($content.kind | str ends-with "RolloutStatus") {
      rollout-formatters
    } else if ($content.kind | str ends-with "Metrics") or ($content.kind | str ends-with "MetricsList") {
      metric-formatters
    } else {
      resource-formatters
    }
    | get -o $resource_spec.group
    | get -o $resource_spec.version
    | get -o $resource_spec.name
  )}

  if ($fmtclosure | is-empty) {
    return $content
  } 
  let output = if ($output | is-not-empty) { $output } else if $many { "compact" } else { "wide" }
  let decoratorclosures = $decorators | default [] | each {|decorator| decorators | get $decorator}

  if not ($many) {
    let base = $content | do $fmtclosure $output
    $decoratorclosures | reduce --fold $base {|closure acc| $acc | do $closure $content}
  } else {
    $content.items | each {|obj|
      let base = $obj | do $fmtclosure $output
      $decoratorclosures | reduce --fold $base {|closure acc| $acc | do $closure $obj}
    }
  }
}
