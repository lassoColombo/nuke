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

export def raw-write [dir: path, file: string, content: any --mod: int] {
  let base = [(basedir) $dir] | path join
  if not ($base | path exists) { mkdir $base }

  let cache_file = [$base $file] | path join
  $content | save -f $cache_file

  if ($mod | is-not-empty) {
    chmod $mod $cache_file
  }
  $cache_file
}

export def write [dir: path file: string, content: any --mod: int, --raw] {
  let base = [(basedir) $dir] | path join
  if not ($base | path exists) { mkdir $base }
  let cache_file = [$base $file] | path join
  {
    metadata: { last_update: (date now) }
    data: $content
  }
  | to json
  | save -f $cache_file

  if ($mod | is-not-empty) {
    chmod $mod $cache_file
  }
  $cache_file
}

export def raw-read [dir: path file: string, --cutoff(-c): duration] {
  let base = [(basedir) $dir] | path join
  if not ($base | path exists) { mkdir $base }

  let cache_file = [$base $file] | path join
  if not ($cache_file | path exists) { touch $cache_file }

  open -r $cache_file 
}

export def read [dir: path, file: string, --cutoff(-c): duration] {
  let base = [(basedir) $dir] | path join
  if not ($base | path exists) { mkdir $base }

  let cache_file = [$base $file] | path join
  if not ($cache_file | path exists) { return null }

  let cache = open -r $cache_file
  if ($cache | is-empty) { return null }

  let cache = $cache | from json

  if ($cutoff | is-not-empty) and ((date now) - ($cache.metadata.last_update | into datetime)) > $cutoff {
    rm $cache_file
    return null
  }

  $cache.data
}
