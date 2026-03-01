# Contributing to Nuke

Thank you for your interest in improving Nuke ❤️

Nuke interacts directly with the Kubernetes API server and its behavior depends heavily on cluster configuration, enabled APIs, authentication methods, and installed components.  
Providing reproducible environments is important for effective contributions.

This guide explains how to report issues, reproduce bugs, and contribute new features.

---

## Table of Contents

- Code of Conduct
- Development Setup
- Reporting Bugs
- Reproducible Test Environments (KIND)
- Metrics Server Setup
- Providing Test Resources
- Contributing Code
- Adding or Improving Formatters
- Pull Request Guidelines

---

## Code of Conduct

Assume good intent, intend good.   
Be responsible and open to discussion.

---

## Development Setup

### Requirements

- Nushell
- Git
- curl
- kubectl (recommended for cluster setup)
- Access to a Kubernetes cluster

### Clone and Load Nuke


```nu
# Clone into one of your Nushell library directories:
git clone git@github.com:lassoColombo/nuke.git ([($env.NU_LIB_DIRS | first) nuke] | path join)
# Verify installation:
use nuke
nuke api-resources
```

---

## Reporting Bugs

Include the following information:

### Environment

- Nuke version or commit hash
- Nushell version
- Kubernetes version (`kubectl version`)
- Cluster type (kind, k3s, EKS, GKE, etc.)
- Operating system

### Description

Provide:

- Exact command executed
- Expected behavior
- Actual behavior
- Error messages (if any)

Some bugs may be hard to diagnose as they depend on both the configuration of the cluster and its state.  
We try to use kind as a common base to ensure reproducibility.

## Reproducible Test Environments (KIND)

If the issue depends on cluster configuration, please provide instructions using KIND whenever possible.

### Install KIND

https://kind.sigs.k8s.io/

### Create a Test Cluster

```bash
kind create cluster --name nuke-test
kubectl cluster-info
```

---

### Metrics Server Setup (Required for `nuke top`)

The Kubernetes Metrics Server is not installed by default in KIND.
Install it as follows (KIND requires an additional patch to allow insecure TLS):

```yaml
# kustomization.yaml
# kubectl apply -k .
resources:
- https://github.com/kubernetes-sigs/metrics-server/releases/download/v0.8.0/components.yaml

apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
patches:
- patch: |-
    - op: add
      path: /spec/template/spec/containers/0/args/-
      value: --kubelet-insecure-tls
  target:
    group: apps
    kind: Deployment
    name: metrics-server
    namespace: kube-system
    version: v1' | save -f kustomization.yaml
```

## Providing Test Resources

If the issue involves specific workloads, include the manifests needed to reproduce it.

Examples:

- Pod definitions
- Deployments
- StatefulSets
- Custom Resources
- Services
- Namespaces
- RBAC configuration

Provide them inline or as files in the issue.

---

## Contributing Code

### General Guidelines

- Keep changes focused
- Prefer small, incremental pull requests
- Maintain Nushell-native style
- Avoid introducing external dependencies unless necessary
- Adhere to kubectl semantics and syntax as much as possible

If your change alters behavior, update documentation accordingly.

---

## Adding or Improving Formatters

Formatters define how Kubernetes objects are presented.
If a resource is not supported, you can implement a formatter.

### Manual Workflow

Suppose you want to write a formatter for `admissionregistration.k8s.io/v1/validatingadmissionpolicies`.

1. write a closure with the following signature:
    ```nu
    let formatter = {|output?: string = compact|
        # formatters should support compact and wide output.
        let obj = $in
        let res = {
            name: $obj.metadata.name
        }
        if $output == compact {
            return $res
        }
        return ($res | merge {
            created: $obj.metadata.creationTimestamp
        })
    }
    ```

2. test your formatter untill you are happy
    ```nu
    nuke show validatingadmissionpolicies -o full | do $formatter compact
    nuke show validatingadmissionpolicies <my-policy> -o full | do $formatter wide
    ```

### Env Variable Workflow

You can set your custom formatters using the following env variables:

- `NUKE_RESOURCE_FORMATTERS`- get command
- `NUKE_ROLLOUTSTATUS_FORMATTERS` - rollout status command
- `NUKE_METRIC_FORMATTERS` - top command

So, you can set the environment variable for validatingadmissionpolicies as follows:
```nu
$env.NUKE_RESOURCE_FORMATTERS = {
    admissionregistration.k8s.io: {
        v1: {
            validatingadmissionpolicies: {|output?: string = compact|
                # your formatter here
            }
        }
    }
}
```

And test your formatter:
```nu
nuke show validatingadmissionpolicies -o full | do $formatter compact
nuke show validatingadmissionpolicies <my-policy> -o full | do $formatter wide
```

### Helpers

When writing a formatter you should make use of the `fmt/helpers.nu` module, which contains common functions used by most of the formatters.  
These functions provide a common implementation for duplicated logic, and is used to provide a consistent formatting and extraction of information from the response.  

You can import these functions when testing and implementing your formatter:
```nu 
use nuke/fmt/helpers.nu
```

### Pull Request

Formatters are located in:
- show/resource-formatters
- rollout/rollout-formatters
- top/metric-formatters

Here each group has a dedicated file containing the formatters for the resources in the group, in all their versions. Formatters must be called as `resourcename <version>`.

So, Suppose you want to improve the formatter for apps/v1/deployments. You wuould open show/resource-formatters/apps.nu, and edit the function `deployments v1`.  
Suppose you want instead to add a new formatter for `admissionregistration.k8s.io/v1/validatingadmissionpolicies`.  You wuould open `show/resource-formatters/admissionregistration_k8s_io.nu`, and add a new function called `validatingadmissionpolicies v1`. Then you wuould open `show/resource-formatters/mod.nu` and add a new entry in the formatters table:
```nu
{
  admissionregistration.k8s.io: {
        v1: {
            validatingadmissionpolicies: {|output?: string = compact| validatingadmissionpolicies v1 $output}
        }
    }
}
```


Then you can open a pull request following the [general guidelines](#pull-request-guidelines)

---

## Pull Request Guidelines

Before submitting a PR:

- Ensure the code loads without errors
- Test against a real cluster
- Confirm no regressions in existing commands
- Update relevant documentation
- Describe the change clearly

PR descriptions should include:

- Motivation
- Approach taken
- Testing performed
- Any limitations or follow-ups

---

## When Reproduction Is Not Possible

If you cannot provide a reproducible environment:

- Explain why
- Provide as much context as possible
- Include logs or screenshots if helpful

Issues without reproducible steps may be difficult to address.

---

## Questions and Discussions

If you are unsure how to approach a contribution, feel free to open an issue to discuss it before starting implementation.

---

Thank you for helping make Nuke better 
