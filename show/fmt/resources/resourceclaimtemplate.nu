export def main [output?: string = compact] {
  let rct = $in

  let requests = ($rct.spec.spec.devices.requests? | default [])

  let device_classes = (
    $requests
    | each {|r|
      $r.exactly?.deviceClassName?
    }
    | where $it != null
    | uniq
  )

  let res = {
    name: $rct.metadata.name
    namespace: $rct.metadata.namespace?
    requests: ($requests | length)
    deviceClasses: $device_classes
    age: ($rct.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert generation ($rct.metadata.generation?)
  | upsert template (
    $requests
    | each {|r|
      {
        name: $r.name?
        deviceClass: $r.exactly?.deviceClassName?
        allocationMode: ($r.exactly?.allocationMode? | default "ExactCount")
        count: ($r.exactly?.count? | default 1)
        capacity: ($r.exactly?.capacity?.requests? | default {})
      }
    }
  )
}
