use /home/runner/work/nuke/nuke/fmt/mod.nu

let res = open .github/kube-resources/resources.yaml
| where {'get' in ($in.verbs? | default [])}

let formatters = mod supported-formatters
let unwilling_to_support = open .github/kube-resources/unwilling_to_support.yaml

let coverage = $res | group-by group --to-table | each {|group|
  let total = $group.items | length
  let supported_formatters = $formatters | where {|formatter| $formatter in $group.items.singularName}
  let supported_number = $supported_formatters | length
  let not_yet_supported_number = $total - $supported_number
  let unwilling_to_support_formatters = $unwilling_to_support | where {|formatter| $formatter in $group.items.singularName}
  let unwilling_to_support_number = $unwilling_to_support_formatters | length

  let graphbody = open -r .github/templates/readme/resource-coverage.mmd
  | str replace ___SUPPORTED___ ($supported_number | into string)
  | str replace ___UNSUPPORTED___ ($not_yet_supported_number | into string)
  | str replace ___UNWILLING___ ($unwilling_to_support_number | into string)
  let graph = $"```mermaid\n($graphbody)\n```"

  let statusbody = $group.items | each {|item|
    if ($item.names | any {$in in $supported_formatters}) {
      $"| ($item.name)     | 🟢     |"
    } else if ($item.names | any {$in in $unwilling_to_support_formatters}) {
      $"| ($item.name)     | 🔴    |"
    } else {
      $"| ($item.name)     | ⚪     |"
    }
  } | str join "\n"
  let status = $"| Resource       | Status |\n|-----------------|--------|\n($statusbody)"

  $"### ($group.group)\n\n($graph)\n\n($status)"
} | str join "\n"

$"# Coverage\n\n($coverage)"
| save -f .doc/resource-coverage/coverage.md 


