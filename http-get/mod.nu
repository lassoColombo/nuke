use "../config/config-completers.nu"
use "../config"

def --env get-http-method [kubeconf] {
  let ctx = $kubeconf.current-context
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

# Performs an authenticated http GET request to the kubernetes api server.
export def --env main [
  spec,
  --kubeconf(-K): record # The configuration to use (defaults to kubeconfig).
  --context(-c): string@"config-completers context" # The context to use in the configuration (defaults to current).
  --cluster(-C): string@"config-completers cluster" # The cluster to use in the configuration (defaults to current).
  --raw(-r) # Return the result as raw stream of bytes.
] {
  mut kubeconf = if ($kubeconf | is-not-empty) {$kubeconf} else {config}

  if ($context | is-not-empty) {
    $kubeconf.current-context = $context
  } else if ($cluster | is-not-empty) { 
    $kubeconf.current-context = $cluster
  }

  let getmethod = get-http-method $kubeconf

  mut default_spec = $kubeconf.clusters 
  | where {$in.name == $kubeconf.current-context}
  | first
  | get cluster.server
  | url parse

  $default_spec.scheme = $spec.scheme? | default $default_spec.scheme?
  $default_spec.username = $spec.username? | default $default_spec.username?
  $default_spec.password = $spec.password? | default $default_spec.password?
  $default_spec.host = $spec.host? | default $default_spec.host?
  $default_spec.port = $spec.port? | default $default_spec.port?
  $default_spec.path = ([($default_spec.path? | default '') ($spec.path? | default '')] | path join)
  $default_spec.query = ([($spec.query? | default '') ($default_spec.query? | default '')] | path join)
  $default_spec.fragment = $spec.fragment? | default $default_spec.fragment?
  $default_spec.params = $default_spec.params? | default [] | append ($spec.params? | default [])

  let path = $default_spec | url join

  if not $raw {
    return (do $getmethod $path | from json)
  }

  do $getmethod $path
}
