use "../../fmt/helpers.nu"

# -----------------------
# SelfSubjectReview
# -----------------------

export def "selfsubjectreviews v1" [output?: string = compact] {
  let r = $in

  let status = ($r.status? | default {})

  let base = (
    $r
    | helpers meta base
    | merge {
        authenticated: ($status.authenticated? | default false)
      }
  )

  if $output == "compact" {
    return $base
  }

  let user = ($status.userInfo? | default {})

  $base | merge {

    user: {
      username: $user.username?
      uid: $user.uid?
      groups: ($user.groups? | default [])
      extra: ($user.extra? | default {})
    }

    audiences: ($status.audiences? | default [])

    error: $status.error?
  }
}

# -----------------------
# TokenReview
# -----------------------

export def "tokenreviews v1" [output?: string = compact] {
  let tr = $in

  let status = ($tr.status? | default {})

  let base = (
    $tr
    | helpers meta base
    | merge {
        authenticated: ($status.authenticated? | default false)
      }
  )

  if $output == "compact" {
    return $base
  }

  let user = ($status.user? | default {})

  $base | merge {

    user: {
      username: $user.username?
      uid: $user.uid?
      groups: ($user.groups? | default [])
      extra: ($user.extra? | default {})
    }

    audiences: ($status.audiences? | default [])

    error: $status.error?
  }
}
