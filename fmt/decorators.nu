export def main [] {
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
      )
    }
    '': {$in}
  }
}

