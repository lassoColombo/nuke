use ./helpers.nu
use ./decorators.nu
use ./formatters.nu

export def supported-outputs [] { [ full wide compact ] }

# lists supported resource formatters
export def supported-formatters [] {
  formatters | transpose formatter closure | get formatter
}

export def resource [
  --output(-o): string
  --decorators(-d): list<string> = []
] {
  let content = $in
  if ($output == full) {
    return $content
  } 
  let closures = if ($content.kind | str ends-with "Metrics") or ($content.kind | str ends-with "MetricsList") {
    formatters metrics
  } else {
    formatters
  }
  let many = $content.kind | str ends-with "List"
  let kind = (if $many { $content.kind | str replace --regex 'List$' '' } else { $content.kind }) | str downcase
  let fmtclosure = $closures | get -o $kind

  if ($fmtclosure | is-empty) {
    $content
  } else {
    let output = if ($output | is-not-empty) { $output } else if $many { "compact" } else { "wide" }

    let closures = $decorators | each {|decorator| decorators | get $decorator}

    if not ($many) {
      let base = $content | do $fmtclosure $output
      $closures | reduce --fold $base {|closure acc| $acc | do $closure $content}
    } else {
      $content.items | each {|obj|
        let base = $obj | do $fmtclosure $output
        $closures | reduce --fold $base {|closure acc| $acc | do $closure $obj}
      }
    }
  }
}
