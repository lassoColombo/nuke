use "../helpers.nu"

export def "resource-formatter" [output?: string = compact] {
  let obj = $in
  let meta = $obj | helpers meta base
  {
    ...$meta
  }
}
