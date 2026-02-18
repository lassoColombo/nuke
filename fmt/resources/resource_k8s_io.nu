export def "deviceclasses v1" [output: string = compact] {
  let dc = $in

  let selectors = ($dc.spec.selectors? | default [])

  let selector_types = (
    $selectors
    | each {|s|
      $s | columns | first
    }
    | uniq
  )

  let res = {
    name: $dc.metadata.name
    resource: ($dc.spec.extendedResourceName?)
    selectors: ($selectors | length)
    selector-types: $selector_types
    age: ($dc.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  } 
  $res
  | upsert generation ($dc.metadata.generation?)
  | upsert selectors (
    $selectors
    | each {|s|
      if ($s.cel? != null) {
        {
          type: "cel"
          expression: $s.cel.expression?
        }
      } else {
        {
          type: ($s | columns | first)
          value: ($s | values | first)
        }
      }
    }
  )
}

export def "resourceclaimtemplates v1" [output?: string = compact] {
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
