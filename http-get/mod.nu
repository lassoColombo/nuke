use "../config"

def getmethods [] {
  {
    token: {|path|
      (
        curl 
        --insecure
        --silent
        --show-error
        --fail-with-body
        --connect-timeout 5
        --max-time 15
        --retry 3
        --retry-all-errors
        -H $"Authorization: Bearer ($env.NUKE_AUTHENTICATION_TOKEN)"
        $path
      )
    }
    cert: {|path|
      (
        curl
        --insecure
        --silent
        --show-error
        --fail-with-body
        --retry 3
        --retry-all-errors
        --cert $env.NUKE_AUTHENTICATION_CERT
        --key $env.NUKE_AUTHENTICATION_KEY
        --cacert $env.NUKE_AUTHENTICATION_AUTHORITY
        $path
      )
    }
  }
}

def --env get-http-method [conf] {
  let ctx = $conf.current-context
  let userconf = config get-users $ctx

  let cache_dir = cache basedir | append auth | path join

  if not ($cache_dir | path exists) {
    mkdir $cache_dir
    chmod 700 $cache_dir
  }

  if ($userconf.user.token? | is-not-empty) {
    return {|path|
      (
        curl 
        --insecure
        --silent
        --show-error
        --fail-with-body
        --connect-timeout 5
        --max-time 15
        --retry 3
        --retry-all-errors
        -H $"Authorization: Bearer ($userconf.user.token)"
        $path
      )
    }
  }

  if ($userconf.user."client-certificate-data"? | is-not-empty) {
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

    config get-clusters $ctx
    | get cluster."certificate-authority-data"
    | decode base64
    | save -f $ca_path
    chmod 600 $ca_path

    return {|path|
      (
        curl
        --insecure
        --silent
        --show-error
        --fail-with-body
        --retry 3
        --retry-all-errors
        --cert $cert_path
        --key $key_path
        --cacert $ca_path
        $path
      )
    }
  }

  error make { msg: 'current authentication method not supported' }
}

# performs an authenticated http GET request to the kubernetes api server
export def --env main [
  url_spec,
  conf?,
  --context(-c): string
  --cluster(-C): string
  --watch(-w)
] {
  mut conf = if ($conf | is-not-empty) {$conf} else {config}
  if ($context | is-not-empty) {
    $conf.current-context = $context
  } else if ($cluster | is-not-empty) { 
    $conf.current-context = $cluster
  }

  let getmethod = get-http-method $conf
  mut spec = config get-clusters --current
  | get cluster.server
  | url parse

  $spec.scheme = $url_spec.scheme? | default $spec.scheme?
  $spec.username = $url_spec.username? | default $spec.username?
  $spec.password = $url_spec.password? | default $spec.password?
  $spec.host = $url_spec.host? | default $spec.host?
  $spec.port = $url_spec.port? | default $spec.port?
  $spec.path = ([($spec.path? | default '') ($url_spec.path? | default '')] | path join)
  $spec.query = ([($url_spec.query? | default '') ($spec.query? | default '')] | path join)
  $spec.fragment = $url_spec.fragment? | default $spec.fragment?
  $spec.params = $spec.params? | default [] | append ($url_spec.params? | default [])

  let path = $spec | url join

  if not $watch {
    return (do $getmethod $path | from json)
  }

  do $getmethod $path
}
