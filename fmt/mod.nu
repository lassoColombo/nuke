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
  let getformatter = {|group version name|
    $in | get -o $group | get -o $version | get -o $name
  }
  let fmtclosure = $env.NUKE_FORMATTERS? 
  | default {}
  | do $getformatter $resource_spec.group $resource_spec.version $resource_spec.name 
  | default {(
    if ($content.kind | str ends-with "RolloutStatus") {
      rollout-formatters
      | do $getformatter $resource_spec.group $resource_spec.version $resource_spec.name 
      | default {rollout-formatters | get -o default}
    } else if ($content.kind | str ends-with "Metrics") or ($content.kind | str ends-with "MetricsList") {
      metric-formatters
      | do $getformatter $resource_spec.group $resource_spec.version $resource_spec.name 
      | default {metric-formatters | get -o default}
    } else {
      let res = resource-formatters
      | do $getformatter $resource_spec.group $resource_spec.version $resource_spec.name 
      | default {resource-formatters | get -o default}
      $res
    }
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
