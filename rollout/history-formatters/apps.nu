use "../../fmt/helpers.nu"

# Extract a normalised container list from a pod template spec
def "history fmt-containers" [] {
  $in.containers? | default [] | each {|c|
    {
      name:  $c.name
      image: $c.image
      ...($c.resources? | helpers resources base)
    }
  }
}

# Extract images list from a pod template spec
def "history fmt-images" [] {
  $in.containers? | default [] | get image
}

export def "deployments v1" [
  deployment: record # the parent Deployment object
  replicasets: list # all RS items in the namespace
  revision?: int # if set, return only this revision 
  output?: string = "compact"
] {
  let current_rev = $deployment.metadata.annotations?
    | default {}
    | get -o "deployment.kubernetes.io/revision"
    | default null
    | if ($in | is-not-empty) { $in | into int } else { null }

  # Filter to RSes owned by this Deployment
  let owned = $replicasets
    | where {|r|
      $r.metadata.ownerReferences?
      | default []
      | any {|o| $o.uid == $deployment.metadata.uid }
    }

  # Build history records
  let records = $owned | each {|rs|
      let rev_str = $rs.metadata.annotations?
        | default {}
        | get -o "deployment.kubernetes.io/revision"

      # Skip RSes that have no revision annotation (edge case: orphaned RSes)
      if ($rev_str | is-empty) { return null }

      let rev = ($rev_str | into int)
      let tpl = ($rs.spec.template? | default {})
      let tpl_meta = ($tpl.metadata? | default {})
      let tpl_spec = ($tpl.spec? | default {})
      let containers = ($tpl_spec | history fmt-containers)

      let change_cause = $tpl_meta.annotations?
        | default {}
        | get -o "kubernetes.io/change-cause"
        | if ($in | is-empty) {
          $rs.metadata.annotations?
          | default {}
          | get -o "kubernetes.io/change-cause"
        } else { $in }

      let compact = {
        created:     ($rs.metadata.creationTimestamp | helpers cvt-time)
        revision:    $rev
        current:     ($current_rev != null and $rev == $current_rev)
        changeCause: $change_cause
      }

      if $output != "wide" { $compact } else {
        $compact | merge {
          containers:    $containers
          replicaStatus: {
            desired:   ($rs.spec.replicas? | default 1)
            current:   ($rs.status.replicas? | default 0)
            ready:     ($rs.status.readyReplicas? | default 0)
            available: ($rs.status.availableReplicas? | default 0)
          }
        }
      }
    }
    | compact   # drop nulls from RSes without revision annotation
    | sort-by revision

  # Single-revision detail requested
  if ($revision | is-not-empty) and $revision != 0 {
    let match = ($records | where revision == $revision | first | default null)
    if ($match | is-empty) {
      error make --unspanned { msg: $"revision ($revision) not found in history" }
    }
    return $match
  }

  $records
}

export def "controllerrevisions v1" [
  owner: record # the parent DaemonSet or StatefulSet object
  controllerrevisions: list # all ControllerRevision items in the namespace
  revision?: int
  output?: string = "compact"
] {
  let kind = ($owner.kind? | default "")

  let current_rev_name = if $kind == "StatefulSet" {
      $owner.status.currentRevision? | default null
    } else if $kind == "DaemonSet" {
      null  # resolved below after sorting
    } else {
      null
    }

  # Filter to ControllerRevisions owned by this object
  let owned = $controllerrevisions
    | where {|cr|
      $cr.metadata.ownerReferences?
      | default []
      | any {|o| $o.uid == $owner.metadata.uid }
    }
    | sort-by revision

  # For DaemonSets: current = highest revision
  let max_rev = if ($owned | length) > 0 {
      $owned | last | get revision
    } else {
      null
    }

  let records = $owned
    | each {|cr|
      let change_cause = $cr.metadata.annotations?
        | default {}
        | get -o "kubernetes.io/change-cause"

      let data = ($cr.data? | default {})
      let tpl_spec = $data.spec?.template?.spec? | default {}
      let containers = ($tpl_spec | history fmt-containers)

      let is_current = (
        if $kind == "StatefulSet" {
          ($current_rev_name != null) and ($cr.metadata.name == $current_rev_name)
        } else {
          # DaemonSet: highest revision = current
          ($max_rev != null) and ($cr.revision == $max_rev)
        }
      )

      let compact = {
        created:     ($cr.metadata.creationTimestamp | helpers cvt-time)
        revision:    $cr.revision
        current:     $is_current
        changeCause: $change_cause
      }

      if $output != "wide" { $compact } else {
        $compact | merge {
          containers: $containers
        }
      }
    }

  if ($revision | is-not-empty) and $revision != 0 {
    let match = ($records | where revision == $revision | first | default null)
    if ($match | is-empty) {
      error make --unspanned { msg: $"revision ($revision) not found in history" }
    }
    return $match
  }

  $records
}

