use "../../fmt/helpers.nu"

# -----------------------
# PriorityClasses
# -----------------------

export def "priorityclasses v1" [output?: string = compact] {
  let pc = $in

  let base = (
    $pc
    | helpers meta base
    | merge {
        value: ($pc.value? | default 0)
        globalDefault: ($pc.globalDefault? | default false)
      }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {
    description: $pc.description?
    preemptionPolicy: ($pc.preemptionPolicy? | default "PreemptLowerPriority")
    owner: ($pc | helpers meta owner)
  }
}
