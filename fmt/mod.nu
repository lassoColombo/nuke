use ./helpers.nu
use ./decorators.nu
use ./formatters.nu

export def supported-outputs [] { [ full wide compact ] }

# lists supported resource formatters
export def supported-formatters [] {
  formatters | transpose formatter closure | get formatter
}

export def main [
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
  let kind = (if $many { $content.kind | str replace --regex 'List$' '' } else { $content.kind }) | str downcase
  let fmtclosure = $env.NUKE_FORMATTERS? 
  | default {}
  | get -o $kind 
  | default {(
    if ($content.kind | str ends-with "RolloutStatus") {
      formatters rollout-status
    } else if ($content.kind | str ends-with "RolloutHistory") {
      formatters rollout-history
    } else if ($content.kind | str ends-with "Metrics") or ($content.kind | str ends-with "MetricsList") {
      formatters metrics
    } else {
      formatters
    }
    | get -o $kind
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
