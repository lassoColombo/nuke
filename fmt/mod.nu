use ./helpers.nu
use ./decorators.nu

export def main [
  resource_spec: record
  formatters: record
  --output(-o): string
  --decorators(-d): list = []
] {
  let content = $in
  if ($output == full) { return $content } 

  let many = $content.kind | str ends-with "List"
  let getformatter = {|group version name|
    $in | get -o $group | get -o $version | get -o $name
  }

  let fmtclosure = $formatters
  | get -o $resource_spec.group
  | get -o $resource_spec.version
  | get -o $resource_spec.name
  | default {$formatters | get -o default}
  if ($fmtclosure | is-empty) {
    return $content
  }

  let output = if ($output | is-not-empty) { $output } else if $many { "compact" } else { "wide" }

  if not ($many) {
    let base = $content | do $fmtclosure $output
    $decorators | reduce --fold $base {|decorator acc| $acc | do $decorator $content}
  } else {
    $content.items | each {|obj|
      let base = $obj | do $fmtclosure $output
      $decorators | reduce --fold $base {|decorator acc| $acc | do $decorator $obj}
    }
  }
}
