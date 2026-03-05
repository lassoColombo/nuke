use "../config"
use "../cache"
use "../http-get"

def discovery-cache-root [kubeconf: record, ctx: string] {
  let url = (config get-clusters --context $ctx --kubeconf $kubeconf).cluster.server | url parse
  let path = $url.path | str replace --all / _
  [discovery $url.host $path] | path join
}

def discover-aggregated [
  --kubeconf(-K): record
  --context(-c): string
  --cluster(-C): string
] {

  let ctx = ($context | default $kubeconf.current-context)
  let root = discovery-cache-root $kubeconf $ctx

  let agg = try {
    http-get { path: apis } -K $kubeconf -c $context -C $cluster -H {
      Accept: "application/json;v=v2;g=apidiscovery.k8s.io;as=APIGroupDiscoveryList"
    }
  } catch {
    null
  }
  if ($agg | is-empty) or $agg.kind != "APIGroupDiscoveryList" {
    return null
  }

  # -------------------------
  # write resources cache
  # -------------------------

  $agg.items | each {|item|
    let group = $item.metadata.name

    $item.versions 
    | where {$in.resources? | is-not-empty}
    | each {|version|
      let dir = [$root $group $version.version] | path join
      let normalized = $version.resources | each {|r|
        {
          name: $r.resource
          singularName: $r.singularResource
          kind: $r.responseKind.kind?
          group: $r.responseKind.group?
          version: $r.responseKind.version?
          scope: $r.scope?
          shortNames: $r.shortNames?
          verbs: $r.verbs?
        }
      }

      cache write $dir serverresources.json {
        resources: $normalized
      } --mod 640
    }
  }

  # -------------------------
  # build servergroups.json
  # -------------------------

  let groups = (
    $agg.items | each {|item|

      let versions = (
        $item.versions | each {|v|
          {
            groupVersion: $"($item.metadata.name)/($v.version)"
            version: $v.version
          }
        }
      )

      # choose preferred version
      let preferred = (
        $versions
        | where {|v|
          (
            not ($v.version | str contains "alpha")
            and not ($v.version | str contains "beta")
          )
        }
        | first
        | default ($versions | first)
      )

      {
        name: $item.metadata.name
        versions: $versions
        preferredVersion: $preferred
      }
    }
  )

  cache write $root servergroups.json { groups: $groups } --mod 640

  discover-core -K $kubeconf -c $context -C $cluster

  $groups
}

def discover-classic [
  --kubeconf(-K): record
  --context(-c): string
  --cluster(-C): string
] {

  print $"(ansi cyan)manual api discovery, might take a while..."

  let ctx = ($context | default $kubeconf.current-context)
  let root = discovery-cache-root $kubeconf $ctx

  # -------------------------
  # fetch /apis
  # -------------------------

  let api_groups = http-get {path: apis} -K $kubeconf -c $context -C $cluster

  # -------------------------
  # fetch preferred version resources
  # -------------------------

  let groups = (
    $api_groups.groups
    | each {|group|

      let group_name = $group.name
      let preferred = $group.preferredVersion

      let gv = $preferred.groupVersion
      let version = $preferred.version

      let dir = [$root $group_name $version] | path join

      let r = http-get {path: $"apis/($gv)"} -K $kubeconf -c $context -C $cluster

      cache write $dir serverresources.json $r --mod 640

      {
        name: $group_name
        versions: [
          {
            groupVersion: $gv
            version: $version
          }
        ]
        preferredVersion: {
          groupVersion: $gv
          version: $version
        }
      }
    }
  )

  # -------------------------
  # write servergroups.json
  # -------------------------

  cache write $root servergroups.json { groups: $groups } --mod 640

  # -------------------------
  # discover core
  # -------------------------

  discover-core -K $kubeconf -c $context -C $cluster

  $groups
}

def discover-core [
  --kubeconf(-K): record
  --context(-c): string
  --cluster(-C): string
] {
  let ctx = ($context | default $kubeconf.current-context)
  let root = discovery-cache-root $kubeconf $ctx

  let core = http-get {path: api} -K $kubeconf -c $context -C $cluster
  cache write $root core.json $core --mod 640

  $core.versions | each {|version|
    let dir = [$root api $version] | path join
    cache write --mod 640 $dir serverresources.json (
      http-get {path: $"api/($version)"} -K $kubeconf -c $context -C $cluster
    )
  }
}

def discover-api [
  --kubeconf(-K): record
  --context(-c): string
  --cluster(-C): string
] {
  let agg = discover-aggregated -K $kubeconf -c $context -C $cluster
  if ($agg | is-not-empty) {
    return $agg
  }
  discover-classic -K $kubeconf -c $context -C $cluster
}

export def build-index [content] {

  # ---------------------------------------
  # flatten all resources
  # ---------------------------------------

  let flat = (
    $content | each {|group|
      $group.versions
      | each {|version|

        $version.resources
        | default []
        | each {|r|

          if ($r.name? | is-empty) {
            null
          } else {
            $r
            | upsert group $group.name
            | upsert version $version.version
          }
        }
      }
      | flatten
    }
    | flatten
    | compact
  )

  # ---------------------------------------
  # group by canonical name
  # ---------------------------------------

  let by_name = ($flat | group-by name)

  # ---------------------------------------
  # alias resolution
  # ---------------------------------------

  let by_alias = (
    $flat
    | reduce -f {} {|r, acc|

      let canonical = $r.name
      let short = ($r.shortNames? | default [])
      let singular = ($r.singularName? | default "")
      let kind = ($r.kind? | default "")

      mut a = $acc

      for s in $short {
        $a = ($a | upsert ($s | str downcase) $canonical)
      }

      if ($singular | is-not-empty) {
        $a = ($a | upsert ($singular | str downcase) $canonical)
      }

      if ($kind | is-not-empty) {
        $a = ($a | upsert ($kind | str downcase) $canonical)
      }

      $a
    }
  )

  # ---------------------------------------
  # preferred versions per group
  # ---------------------------------------

  let preferred = (
    $content
    | reduce -f {} {|g, acc|

      let pref = ($g.preferredVersion?.version? | default null)

      if ($pref | is-empty) {
        $acc
      } else {
        $acc | upsert $g.name $pref
      }
    }
  )

  {
    flat: $flat
    by_name: $by_name
    by_alias: $by_alias
    preferred: $preferred
  }
}

export def load [
  --kubeconf(-K): record
  --context(-c): string
  --cluster(-C): string
] {

  let ctx = ($context | default $kubeconf.current-context)
  let root = discovery-cache-root $kubeconf $ctx

  mut groups = cache read $root servergroups.json -c 10min

  if ($groups | is-empty) {
    discover-api -K $kubeconf -c $context -C $cluster
    $groups = cache read $root servergroups.json -c 10min
  }

  let noncore = (
    $groups.groups
    | each {|group|

      let versions = (
        $group.versions
        | each {|v|

          let dir = [$root $group.name $v.version] | path join
          let resources = cache read $dir serverresources.json -c 10min

          if ($resources | is-empty) {
            null
          } else {
            {
              version: $v.version
              groupVersion: $v.groupVersion
              resources: $resources.resources
            }
          }
        }
        | compact
      )

      if ($versions | is-empty) {
        null
      } else {
        {
          name: $group.name
          versions: $versions
        }
      }
    }
    | compact
  )

  # -------------------------
  # load core api
  # -------------------------

  let core = cache read $root core.json -c 10min

  let core_group = (
    if ($core | is-empty) {
      null
    } else {
      {
        name: "api"
        versions: (
          $core.versions
          | each {|v|

            let dir = [$root api $v] | path join
            let r = cache read $dir serverresources.json -c 10min

            if ($r | is-empty) {
              null
            } else {
              {
                version: $v
                groupVersion: $"api/($v)"
                resources: $r.resources
              }
            }
          }
          | compact
        )
      }
    }
  )

  if ($core_group | is-empty) {
    $noncore
  } else {
    $noncore | append $core_group
  }
}

export def resource-index [
  --kubeconf(-K): record
  --kubeconfpath(-k): path
  --context(-c): string
  --cluster(-C): string
] {

  let kubeconf = if ($kubeconf | is-not-empty) {
    $kubeconf
  } else {
    config --kubeconfpath $kubeconfpath
  }

  let content = load -K $kubeconf -c $context -C $cluster
  build-index $content
}
