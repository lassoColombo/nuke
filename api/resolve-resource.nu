use "../api"

export def main [
  resourcename: string
  resources: list
] {

  if ($resources | length) == 0 {
    error make {
      msg: "run 'nuke api resources' to get the full list"
      label: {
        text: $'($resourcename) is not a resource from the cluster'
        span: (metadata $resourcename).span
      }
    }
  }

  if ($resources | length) == 1 {
    return ($resources)
  }

  let groups = (api groups)

  
  let annotated = ($resources | each { |r|
      let group_info = ($groups | where name == $r.group | default [null] | first)
      let is_preferred = if $group_info != null and $group_info.preferredVersion?.version? != null {
          $r.version == $group_info.preferredVersion.version
      } else {
          false
      }
      $r | insert is_preferred $is_preferred
  })

  
  let preferred = ($annotated | where is_preferred == true)
  if ($preferred | length) == 1 {
      return ($preferred)
  }
  let annotated = if ($preferred | length) > 0 { $preferred } else { $annotated }

  
  let non_core = ($annotated | where group != "api")
  if ($non_core | length) == 1 {
      return ($non_core)
  }
  let annotated = if ($non_core | length) > 0 { $non_core } else { $annotated }

  
  let stable = ($annotated | where ($it.version | str contains "alpha") == false and ($it.version | str contains "beta") == false)
  if ($stable | length) == 1 {
      return ($stable )
  }
  let annotated = if ($stable | length) > 0 { $stable } else { $annotated }

  
  $annotated
  | sort-by group version
  | first
}

