use "../config/config-completers.nu"
use "../config"
use "../cache"
use std/log

def cache-material [
  data: string,
  --decode-base64,
  --mod: int = 600
] {
  let cache_basedir = 'auth'
  let file = $data | hash sha256
  let cached = cache raw-read $cache_basedir $file

  if ($cached | is-not-empty) {
    return ([(cache basedir) $cache_basedir $file] | path join)
  } 

  let content = if $decode_base64 {
    $data | decode base64 | decode
  } else {
    $data
  }
  cache raw-write $cache_basedir $file $content --mod $mod
}

def prepare-auth [kubeconf, clusterconf, userconf] {
  if ($userconf.user.token? | is-not-empty) {
    return [ -H $"Authorization: Bearer ($userconf.user.token)" ] 
  }

  if ($userconf.user.tokenFile? | is-not-empty) {
    return [ -H $"Authorization: Bearer (open $userconf.user.tokenFile | str trim)" ]
  }

  if ($userconf.user.username? | is-not-empty) {
    return [ -u $"($userconf.user.username):($userconf.user.password)" ]
  }

  mut cert_args = []

  if ($userconf.user."client-certificate-data"? | is-not-empty) {
    let cert_path = cache-material $userconf.user."client-certificate-data" --decode-base64
    $cert_args = $cert_args | append [ --cert $cert_path ]
  }

  if ($userconf.user."client-key-data"? | is-not-empty) {
    let key_path = cache-material $userconf.user."client-key-data" --decode-base64
    $cert_args = $cert_args | append [ --key $key_path ]
  }

  if ($userconf.user."client-certificate"? | is-not-empty) {
    $cert_args = $cert_args | append [ --cert $userconf.user."client-certificate" ]
  }

  if ($userconf.user."client-key"? | is-not-empty) {
    $cert_args = $cert_args | append [ --key $userconf.user."client-key" ]
  }

  $cert_args
}

def build-curl-args [
  path: string, 
  --headers(-H): record, 
  --kubeconf(-K): record, 
  --raw] {
  let ctx = $kubeconf.current-context
  let userconf = config get-users --context $ctx --kubeconf $kubeconf
  let clusterconf = config get-clusters --context $ctx --kubeconf $kubeconf

  let ca_path = (
    if ($clusterconf.cluster."certificate-authority-data"? | is-not-empty) {
      cache-material $clusterconf.cluster."certificate-authority-data" --decode-base64
    } else if ($clusterconf.cluster."certificate-authority"? | is-not-empty) {
      $clusterconf.cluster."certificate-authority"
    } else {
      null
    }
  )
  
  let base_headers = $headers | transpose k v | each {|h|
    [-H $"($h.k): ($h.v)"] 
  } | flatten

  let curl_args = [
    --silent
    --show-error
    ...(if $raw {[null]} else {[--write-out "\n%{http_code}"]} )
    --retry 3
    --retry-all-errors
    ...$base_headers
  ] 
  | compact
  | append ( # proxy args
    if ($clusterconf.cluster."proxy-url"? | is-empty) { null } else {
      [ "--proxy", $clusterconf.cluster."proxy-url" ]
    }
  ) 
  | append ( # ssl verificarion args
    if ($ca_path | is-not-empty) {
      [ --cacert $ca_path ] 
    } else if ($clusterconf.cluster."insecure-skip-tls-verify"? | default false) {
      [ --insecure ] 
    } else {
      null
    }
  )
  | append ( # user impersonation
    if ($userconf.user.as? | is-empty) { null } else {
      [ -H $"Impersonate-User: ($userconf.user.as)" ]
    }
  )
  | append ( # group impersonation
    if ($userconf.user."as-groups"? | is-not-empty) {
      $userconf.user."as-groups"? | default [] | each {|g|
        [ -H $"Impersonate-Group: ($g)" ]
      } | flatten
    }
  )
  | append (prepare-auth $kubeconf $clusterconf $userconf)
  | append $path

  log debug $path
  $curl_args
}

# Performs an authenticated http GET request to the kubernetes api server.
export def main [
  spec,
  --headers(-H): record = {}
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

  let spec = if ($spec | describe) == string { {path: $spec} } else { $spec }
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
  if $raw {
    let args = build-curl-args $path -K $kubeconf -H $headers --raw
    let getmethod = {curl ...$args}
    return (do $getmethod $path)
  }

  let args = build-curl-args $path -K $kubeconf -H $headers
  let getmethod = {curl ...$args}
  let response = (do $getmethod $path | lines)
  let status = ($response | last | into int)
  let body = ($response | drop | str join "\n" | from json)

  if $status != 200 { error make --unspanned {msg: $body.message} }
  $body
}
