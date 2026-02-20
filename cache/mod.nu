use "../config/config-completers.nu"

export def basedir [] {
  [
    (
      $env.XDG_CACHE_HOME? | default (
        [$env.HOME .cache] | path join
      )
    )
    nuke
  ] 
  | path join
}

export def write [file: string, content: any] {
  let cache_file = $"(basedir)/($file).yaml"
  {
    metadata: {
      last_update: (date now)
    }
    data: $content
  }
  | to yaml 
  | save -f $cache_file
}

export def read [file: string, --cutoff(-c): duration = 0sec] {
  let cache_dir = basedir
  if not ($cache_dir | path exists) {
    mkdir $cache_dir
  }
  let cache_file = $"($cache_dir)/($file).yaml"
  if not ($cache_file | path exists) {
    touch $cache_file
  }

  let cache = open $cache_file
  if ($cache | is-empty) {
    return $cache
  }
  if ((date now) - ($cache.metadata.last_update | into datetime)) > $cutoff {
    return null
  }
  return $cache.data
}

export def clean [
  cluster?: string@"config-completers cluster"
] {
  let cache_dir = [
    (basedir)
    (if ($cluster | is-not-empty) {$"($cluster).*.yaml"} else null)
  ] 
  | where {$in | is-not-empty}
  | path join
  | into glob

  rm -r $cache_dir
}
