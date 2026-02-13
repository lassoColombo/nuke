export def main [output?: string = compact] {
  let sc = $in

  let ready_cond = ($sc.status.conditions? | where type == Ready | first | default {})
  let res = {
    name: $sc.metadata.name
    cidrs: ($sc.spec.cidrs? | default [])
    ready: ($ready_cond.status?)
  }

  if ($output | is-empty) or $output == compact {
    reutrn $res
  } 
  $res
  | upsert message ($ready_cond.message?)
  | upsert reason ($ready_cond.reason?)
  | upsert finalizers ($sc.metadata.finalizers?)
  | upsert age ($sc.metadata.creationTimestamp?)
}
