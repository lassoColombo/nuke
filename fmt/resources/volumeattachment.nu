export def main [output?: string = compact] {
  let va = $in

  let source = ($va.spec.source? | default {})
  let attach_error = ($va.status.attachError? | default {})

  let res = {
    name: $va.metadata.name
    pv: ($source.persistentVolumeName?)
    node: ($va.spec.nodeName?)
    attached: ($va.status.attached? | default false)
    age: ($va.metadata.creationTimestamp? | helpers fmtage)
  }

  if ($output | is-empty) or $output == compact {
    return $res
  }

  $res
  | upsert attacher ($va.spec.attacher?)
  | upsert source (
    if ($source | is-empty) {
      null
    } else {
      $source
    }
  )
  | upsert attachError (
    if ($attach_error | is-empty) {
      null
    } else {
      {
        message: $attach_error.message?
        time: (
          if ($attach_error.time? | is-not-empty) {
            $attach_error.time | into datetime
          } else {
            null
          }
        )
      }
    }
  )
  | upsert created (
    if ($va.metadata.creationTimestamp? | is-not-empty) {
      $va.metadata.creationTimestamp | into datetime
    } else {
      null
    }
  )
}
