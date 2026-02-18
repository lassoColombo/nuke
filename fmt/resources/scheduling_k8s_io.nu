export def "priorityclasses v1" [output?: string = compact] {
  let pc = $in
  {
    name: $pc.metadata.name
    value: $pc.value?
    preemption-policy: ($pc.preemptionPolicy? | default false)
    global-default: ($pc.globalDefault? | default false)
    age: ($pc.metadata.creationTimestamp? | helpers fmtage)
  }
}
