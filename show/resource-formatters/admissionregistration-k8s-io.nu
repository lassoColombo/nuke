use "../../fmt/helpers.nu"

# -----------------------
# MutatingWebhookConfigurations
# -----------------------

export def "mutatingwebhookconfigurations v1" [output?: string = compact] {
  let m = $in

  let hooks = ($m.webhooks? | default [])

  let base = (
    $m
    | helpers meta base
    | merge {
        webhooks: ($hooks | length)
      }
  )

  if $output == "compact" {
    return $base
  }

  let spec = (
    $hooks
    | each {|w|
        {
          name: $w.name
          failurePolicy: ($w.failurePolicy? | default "Fail")
          matchPolicy: ($w.matchPolicy? | default "Equivalent")
          sideEffects: $w.sideEffects?
          admissionReviewVersions: ($w.admissionReviewVersions? | default [])
          rules: ($w.rules? | default [])
        }
      }
  )

  $base | merge {
    owner: ($m | helpers meta owner)
    webhooksSpec: $spec
  }
}


# -----------------------
# ValidatingWebhookConfigurations
# -----------------------

export def "validatingwebhookconfigurations v1" [output?: string = compact] {
  let v = $in

  let hooks = ($v.webhooks? | default [])

  let base = (
    $v
    | helpers meta base
    | merge {
        webhooks: ($hooks | length)
      }
  )

  if $output == "compact" {
    return $base
  }

  let spec = (
    $hooks
    | each {|w|
        {
          name: $w.name
          failurePolicy: ($w.failurePolicy? | default "Fail")
          matchPolicy: ($w.matchPolicy? | default "Equivalent")
          sideEffects: $w.sideEffects?
          admissionReviewVersions: ($w.admissionReviewVersions? | default [])
          rules: ($w.rules? | default [])
        }
      }
  )

  $base | merge {
    owner: ($v | helpers meta owner)
    webhooksSpec: $spec
  }
}


# -----------------------
# ValidatingAdmissionPolicies
# -----------------------

export def "validatingadmissionpolicies v1" [output?: string = compact] {
  let p = $in

  let validations = (
    $p.spec.validations?
    | default []
  )

  let base = (
    $p
    | helpers meta base
    | merge {
        validations: ($validations | length)
        failurePolicy: ($p.spec.failurePolicy? | default "Fail")
      }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {

    matchConstraints: ($p.spec.matchConstraints? | default {})

    validationsSpec: (
      $validations
      | each {|v|
          {
            expression: $v.expression
            message: $v.message?
            reason: $v.reason?
          }
        }
    )

    auditAnnotations: ($p.spec.auditAnnotations? | default [])

    variables: ($p.spec.variables? | default [])

    owner: ($p | helpers meta owner)
  }
}


# -----------------------
# ValidatingAdmissionPolicyBindings
# -----------------------

export def "validatingadmissionpolicybindings v1" [output?: string = compact] {
  let b = $in

  let policy = $b.spec.policyName?

  let base = (
    $b
    | helpers meta base
    | merge {
        policy: $policy
      }
  )

  if $output == "compact" {
    return $base
  }

  $base | merge {

    matchResources: ($b.spec.matchResources? | default {})

    validationActions: ($b.spec.validationActions? | default [])

    paramRef: $b.spec.paramRef?

    owner: ($b | helpers meta owner)
  }
}
