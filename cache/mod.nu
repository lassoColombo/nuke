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

export def raw-write [file: string, content: any --mod: int] {
  let cache_file = $"(basedir)/($file)"
  $content | save -f $cache_file
  if ($mod | is-not-empty) {
    chmod $mod $cache_file
  }
  $cache_file
}

export def write [file: string, content: any --mod: int, --raw] {
  let cache_file = $"(basedir)/($file)"
  {
    metadata: { last_update: (date now) }
    data: $content
  }
  | to yaml 
  | save -f $cache_file
  if ($mod | is-not-empty) {
    chmod $mod $cache_file
  }
  $cache_file
}

export def raw-read [file: string, --cutoff(-c): duration] {
  let cache_dir = basedir
  if not ($cache_dir | path exists) { mkdir $cache_dir }

  let cache_file = $"($cache_dir)/($file)"
  if not ($cache_file | path exists) { touch $cache_file }

  open -r $cache_file 
}

export def read [file: string, --cutoff(-c): duration] {
  let cache_dir = basedir
  if not ($cache_dir | path exists) { mkdir $cache_dir }

  let cache_file = $"($cache_dir)/($file)"
  if not ($cache_file | path exists) { touch $cache_file }

  let cache = open -r $cache_file 
  if ($cache | is-empty) {return $cache}

  let $cache = $cache | from yaml
  if ($cutoff | is-not-empty) and ((date now) - ($cache.metadata.last_update | into datetime)) > $cutoff {
    rm $cache_file
    return null
  }

  return $cache.data
}
