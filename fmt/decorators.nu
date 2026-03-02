def _decorators [] {
  {
    namespace: {|obj|
      $in | upsert namespace ($obj.metadata.namespace)
    }
    labels: {|obj|
      $in | upsert labels ($obj.metadata.labels?)
    }
    annotations: {|obj|
      $in | upsert annotations ($obj.metadata.annotations?)
    }
    conditions: {|obj|
      $in | upsert conditions (
        $obj.status.conditions?
        | default []
        | select -o type lastTransitionTime reason message
        | sort-by lastTransitionTime
        | each {|condition|
          $condition | update lastTransitionTime (
            $condition.lastTransitionTime | into datetime
          )
        }
        | group-by type --to-table
        | reduce -f {} {|group acc|
          let g = $group | update items (
            $group.items | reject -o type
          )
          $acc | merge {
            $g.type: ($g.items | sort-by lastTransitionTime)
          }
        }
      )
    }
    '': {$in}
  }
}

export def main [
  --namespace
  --labels
  --annotations
  --conditions
] {
  [
    (if $namespace {(_decorators).namespace} else {null})
    (if $labels {(_decorators).labels} else {null})
    (if $annotations {(_decorators).annotations} else {null})
    (if $conditions {(_decorators).conditions} else {null})
  ]
  | compact
}
