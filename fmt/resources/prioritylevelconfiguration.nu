export def main [output?: string = compact] {
  let plc = $in

  let limited = ($plc.spec.limited? | default {})
  let queuing = ($limited.limitResponse?.queuing? | default {})

  let res = {
    name: $plc.metadata.name
    type: ($plc.spec.type?)
    concurrencyShares: ($limited.nominalConcurrencyShares? | default 0)
    handSize: ($queuing.handSize? | default 0)
    queues: ($queuing.queues? | default 0)
    queueLengthLimit: ($queuing.queueLengthLimit? | default 0)
    age: ($plc.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  $res
  | upsert generation ($plc.metadata.generation?)
  | upsert queuing (
      if ($queuing | is-empty) {
        null
      } else {
        {
          handSize: $queuing.handSize?
          queues: $queuing.queues?
          queueLengthLimit: $queuing.queueLengthLimit?
        }
      }
    )
  | upsert lendablePercent ($limited.lendablePercent? | default null)
  | upsert limitResponseType ($limited.limitResponse.type? | default null)
  | upsert created (
      if ($plc.metadata.creationTimestamp? | is-not-empty) {
        $plc.metadata.creationTimestamp | into datetime
      } else {
        null
      }
    )
}
