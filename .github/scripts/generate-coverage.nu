use /home/runner/work/nuke/nuke/fmt/mod.nu

let res = open .github/kube-resources/resources.yaml
| where {'get' in ($in.verbs? | default [])}

let formatters = mod supported-formatters
let unwilling_to_support = open .github/kube-resources/unwilling_to_support.yaml

let total = $res | length
let supported_formatters = $formatters | where {|formatter| $formatter in $res.singularName}
let supported_number = $supported_formatters | length
let not_yet_supported_number = $total - $supported_number
let unwilling_to_support_formatters = $unwilling_to_support | where {|formatter| $formatter in $res.singularName}
let unwilling_to_support_number = $unwilling_to_support_formatters | length

let graphbody = open -r .github/templates/readme/resource-coverage.mmd
| str replace ___SUPPORTED___ ($supported_number | into string)
| str replace ___UNSUPPORTED___ ($not_yet_supported_number | into string)
| str replace ___UNWILLING___ ($unwilling_to_support_number | into string)
let graph = $"```mermaid\n($graphbody)\n```"

let coverage = $res | group-by group --to-table | each {|group|
  let supported_formatters = $formatters | where {|formatter| $formatter in $group.items.singularName}
  let unwilling_to_support_formatters = $unwilling_to_support | where {|formatter| $formatter in $group.items.singularName}

  let t = $group.items | each {|item|
    {
      resource: $item.name
      status: (
        if ($item.names | any {$in in $supported_formatters}) { 
          "🟢" 
        } else if ($item.names | any {$in in $unwilling_to_support_formatters}) {
          "🔴"
        } else {
          "⚪"
        }
      )
    }
  } | to md
  $"### ($group.group)\n\n($t)"
} | str join "\n\n"

$"# Coverage\n\n($graph)\n\n($coverage)"
| save -f .doc/resource-coverage/coverage.md
