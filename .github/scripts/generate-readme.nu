use /home/runner/work/nuke/nuke/fmt/mod.nu

let res = open .github/kube-resources/resources.yaml
| where {'get' in ($in.verbs? | default [])}

let formatters = mod supported-formatters
let unwilling_to_support = open .github/kube-resources/unwilling_to_support.yaml

let total = $res | length
let supported_number = $formatters | length
let not_yet_supported_number = $total - $supported_number
let unwilling_to_support_number = $unwilling_to_support | length

let coverage = open -r .github/templates/readme/resource-coverage.mmd
| str replace ___SUPPORTED___ ($supported_number | into string)
| str replace ___UNSUPPORTED___ ($not_yet_supported_number | into string)
| str replace ___UNWILLING___ ($unwilling_to_support_number | into string)

let statusbody = $res
| sort-by name
| each {|resource|
  if ($resource.names | any {$in in $formatters}) {
    $"| ($resource.name)     | 🟢     |"
  } else if ($resource.names | any {$in in $unwilling_to_support}) {
    $"| ($resource.name)     | 🔴    |"
  } else {
    $"| ($resource.name)     | ⚪     |"
  }

} | str join $"\n"

let status = $"| Resource       | Status |\n|-----------------|--------|\n($statusbody)"

open -r .github/templates/readme/readme.md 
| str replace ___COVERAGE_PLACEHOLDER___ $coverage
| str replace ___LIST_PLACEHOLDER___ $status
| save -f README.md
