use ./config.nu

def getmethods [] {
  {
    token: {|server path|
      http get $"($server)/($path)" --headers {
        Authorization: $"Bearer ($env.NUKE_AUTHENTICATION_TOKEN)"
      }
    }
    cert: {|server path|
      (curl -s
        --cert $env.NUKE_AUTHENTICATION_CERT
        --key $env.NUKE_AUTHENTICATION_KEY
        --cacert $env.NUKE_AUTHENTICATION_AUTHORITY
        $"($server)/($path)" | from json)
    }
  }
}

export def --env auth-reset [conf?] {
  let conf = if ($conf | is-not-empty) { $conf } else { config read } 

  let ctx = $conf.context
  let userconf = ($conf.config.users | where name == $ctx | first)

  let base_cache = $env.XDG_CACHE_HOME? | default [$env.HOME .cache] | append nuke | append auth
  let cache_dir = $base_cache | append $ctx | path join

  if not ($cache_dir | path exists) {
    mkdir $cache_dir
    chmod 700 $cache_dir
  }

  if ($env.NUKE_LAST_CONTEXT? | is-not-empty) and ($env.NUKE_LAST_CONTEXT != $ctx) {
    let old_dir = ($base_cache | append $env.NUKE_LAST_CONTEXT | path join)
    if ($old_dir | path exists) { rm -r $old_dir }
  }
  
  if ($userconf.user.token? | is-not-empty) {
    $env.NUKE_AUTHENTICATION_METHOD = 'token'
    $env.NUKE_AUTHENTICATION_TOKEN = $userconf.user.token
    try {hide-env NUKE_AUTHENTICATION_CERT}
    try {hide-env NUKE_AUTHENTICATION_KEY}
    try {hide-env NUKE_AUTHENTICATION_AUTHORITY}
    return
  }

  if ($userconf.user."client-certificate-data"? | is-not-empty) {
    try {hide-env NUKE_AUTHENTICATION_TOKEN}
    $env.NUKE_AUTHENTICATION_METHOD = 'cert'

    let cert_path = ($cache_dir | path join 'client-cert.pem')
    let key_path  = ($cache_dir | path join 'client-key.pem')
    let ca_path   = ($cache_dir | path join 'ca.pem')

    $userconf.user."client-certificate-data"
      | decode base64
      | save -f $cert_path
    chmod 600 $cert_path

    $userconf.user."client-key-data"
      | decode base64
      | save -f $key_path
    chmod 600 $key_path

    $conf.config.clusters
      | where name == $ctx
      | first
      | get cluster."certificate-authority-data"
      | decode base64
      | save -f $ca_path
    chmod 600 $ca_path

    $env.NUKE_AUTHENTICATION_CERT = $cert_path
    $env.NUKE_AUTHENTICATION_KEY = $key_path
    $env.NUKE_AUTHENTICATION_AUTHORITY = $ca_path

    return
  }

  try { hide-env NUKE_AUTHENTICATION_METHOD }
  error make { msg: 'current authentication method not supported' }
}

export def --env main [conf, path] {
  if ($env.NUKE_LAST_CONTEXT? | is-empty) {
    $env.NUKE_LAST_CONTEXT = $conf.context
  }

  if (($env.NUKE_LAST_CONTEXT != $conf.context) or
    ($env.NUKE_AUTHENTICATION_METHOD? | is-empty)) {
    auth-reset $conf
    $env.NUKE_LAST_CONTEXT = $conf.context
  }

  let getmethod = (getmethods | get $env.NUKE_AUTHENTICATION_METHOD)
  if ($getmethod | is-empty) {
    error make { msg: 'current authentication method not implemented' }
  }

  let server = ($conf.config.clusters
    | where name == $conf.context
    | first
    | get cluster.server)

  do $getmethod $server $path
}
