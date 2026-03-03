use "../config/config-completers.nu"
use "../config"

def get-http-method [kubeconf] {
  let ctx = $kubeconf.current-context
  let userconf = config get-users --context $ctx --kubeconf $kubeconf
  let clusterconf = config get-clusters --context $ctx --kubeconf $kubeconf

  let cache_dir = cache basedir | append auth | path join

  if not ($cache_dir | path exists) {
    mkdir $cache_dir
    chmod 700 $cache_dir
  }

  # -----------------------------
  # Resolve CA
  # -----------------------------

  let ca_path = (
    if ($clusterconf.cluster."certificate-authority-data"? | is-not-empty) {
      let p = ($cache_dir | path join 'ca.pem')
      $clusterconf.cluster."certificate-authority-data"
      | decode base64
      | save -f $p
      chmod 600 $p
      $p
    } else if ($clusterconf.cluster."certificate-authority"? | is-not-empty) {
      $clusterconf.cluster."certificate-authority"
    } else {
      null
    }
  )

  let base_args = [
    --silent
    --show-error
    --fail-with-body
    --retry 3
    --retry-all-errors
  ]
  let ca_args = if ($ca_path | is-not-empty) { [ --cacert $ca_path ] } else { [] }
  let impersonation = impersonation-headers $userconf
  # -----------------------------
  # 1. Bearer token (inline)
  # -----------------------------

  if ($userconf.user.token? | is-not-empty) {
    return {|path|
      let args = (
        $base_args
        | append $ca_args
        | append [
          -H $"Authorization: Bearer ($userconf.user.token)"
        ]
        | append $impersonation
        | append $path
      )
      curl ...$args
    }
  }

  # -----------------------------
  # 2. Bearer token (tokenFile)
  # -----------------------------

  if ($userconf.user.tokenFile? | is-not-empty) {
    let token = (open $userconf.user.tokenFile | str trim)

    return {|path|
      curl ...(
        $base_args
        | append $ca_args
        | append [ -H $"Authorization: Bearer ($token)" ]
        | append $impersonation
        | append $path
      )
    }
  }

  # -----------------------------
  # 3. Client cert (embedded data)
  # -----------------------------

  if ($userconf.user."client-certificate-data"? | is-not-empty) {
    let cert_path = ($cache_dir | path join 'client-cert.pem')
    let key_path  = ($cache_dir | path join 'client-key.pem')

    $userconf.user."client-certificate-data"
    | decode base64
    | save -f $cert_path
    chmod 600 $cert_path

    $userconf.user."client-key-data"
    | decode base64
    | save -f $key_path
    chmod 600 $key_path

    return {|path|
      curl ...(
        $base_args
        | append $ca_args
        | append [ --cert $cert_path --key $key_path ]
        | append $impersonation
        | append $path
      )
    }
  }

  # -----------------------------
  # 4. Client cert (file paths)
  # -----------------------------

  if ($userconf.user."client-certificate"? | is-not-empty) {
    return {|path|
      curl ...(
        $base_args
        | append $ca_args
        | append [
          --cert $userconf.user."client-certificate"
          --key  $userconf.user."client-key"
        ]
        | append $impersonation
        | append [$path]
      )
    }
  }

  # -----------------------------
  # 5. Basic auth
  # -----------------------------

  if ($userconf.user.username? | is-not-empty) {
    return {|path|
      curl ...(
        $base_args
        | append $ca_args
        | append [
          -u $"($userconf.user.username):($userconf.user.password)"
        ]
        | append $impersonation
        | append [$path]
      )
    }
  }

  # -----------------------------
  # 6. Anonymous
  # -----------------------------

  if ($userconf.user | is-empty) {
    return {|path|
      curl ...(
        $base_args
        | append $ca_args
        | append [$path]
      )
    }
  }

  error make { msg: 'current authentication method not supported (exec/oidc not handled)' }
}

# ---------------------------------
# Impersonation helper
# ---------------------------------

def impersonation-headers [userconf] {
  mut headers = []

  if ($userconf.user.as? | is-not-empty) {
    $headers = ($headers | append [ -H $"Impersonate-User: ($userconf.user.as)" ])
  }

  if ($userconf.user."as-groups"? | is-not-empty) {
    for g in $userconf.user."as-groups" {
      $headers = ($headers | append [ -H $"Impersonate-Group: ($g)" ])
    }
  }

  $headers
}

# Performs an authenticated http GET request to the kubernetes api server.
export def main [
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

  mut default_spec = config get-clusters --context $kubeconf.current-context --kubeconf $kubeconf
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
