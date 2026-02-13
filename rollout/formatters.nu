use "../fmt/helpers.nu"

export def main [
] {
  {
    status: {rollout_status}
    history: {|output?: string = compact| 
      # todo
    }
  }
}

def rollout_status [] {
  let obj = $in

  let evaluators = {
    Deployment: {deployment_status}
    StatefulSet: {statefulset_status}
  }

  let evaluator = ($evaluators | get -o $obj.kind)

  if ($evaluator | is-empty) {
    error make {
      msg: $"rollout status not supported for kind ($obj.kind)"
    }
  }

  $obj | do $evaluator
}
