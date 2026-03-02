export def context [context?: string] { 
  if ($context | is-empty) {
    return (load | get contexts.name)
  } 
  mut prev = $context | parse --regex '(?P<word>\S+)' | get word

  let idx = $prev | enumerate | where {$in.item in ['-k', '--kubeconfpath']} | get index
  let kubeconfpath = if ($idx | is-not-empty) {
    $prev | get (($idx | first) + 1)
  } else {
    ''
  }

  load --kubeconfpath $kubeconfpath | get contexts.name 
}

export def context-key [] {[cluster namespace user]}
export def cluster [context?: string] {
  if ($context | is-empty) {
    return (load | get clusters.name )
  } 

  mut prev = $context | parse --regex '(?P<word>\S+)' | get word

  let idx = $prev | enumerate | where {$in.item in ['-k', '--kubeconfpath']} | get index
  let kubeconfpath = if ($idx | is-not-empty) {
    $prev | get (($idx | first) + 1)
  } else {
    ''
  }

  load --kubeconfpath $kubeconfpath | get clusters.name 
}

export def cluster-key [] {[server]}
export def user [context?: string] {
  if ($context | is-empty) {
    return (load | get users.name )
  } 

  mut prev = $context | parse --regex '(?P<word>\S+)' | get word

  let idx = $prev | enumerate | where {$in.item in ['-k', '--kubeconfpath']} | get index
  let kubeconfpath = if ($idx | is-not-empty) {
    $prev | get (($idx | first) + 1)
  } else {
    ''
  }

  load --kubeconfpath $kubeconfpath | get users.name 
}

export def user-key [] {[token]}

# returns the path to the kubeconfig file.
export def path [] {
  if ($env.KUBECONFIG? | is-not-empty) {
     $env.KUBECONFIG
  } else {
    [$env.HOME .kube config] | path join
  }
}

# loads and returns the kubeconfig as a record.
export def load [--kubeconfpath: path] {
  let p = if ($kubeconfpath | is-empty) or $kubeconfpath == '' {path} else {$kubeconfpath}
  open -r $p | from yaml
}
