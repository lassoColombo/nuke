export def write [file: string, content: any] {
  let cache_file = $"($env.XDG_CACHE_HOME? | default $'($env.HOME)/.cache')/nuke/($file).yaml"
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
  let cache_dir = $"($env.XDG_CACHE_HOME? | default $'($env.HOME)/.cache')/nuke"
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

export def clean [] {
  let cache_dir = $"($env.XDG_CACHE_HOME? | default $'($env.HOME)/.cache')/nuke"
  rm -r $cache_dir
}
