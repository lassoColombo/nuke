# Contributing to Nuke

Thank you for your interest in improving Nuke ❤️

Nuke interacts directly with the Kubernetes API server and its behavior depends heavily on cluster configuration, enabled APIs, authentication methods, and installed components.  
Providing reproducible environments is therefore essential for effective contributions.

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

Be respectful and constructive.  
Assume good intent.  
We are here to build useful software together.

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

### Authentication

How you authenticate to the cluster:

- Token
- Client certificates
- Exec plugin
- OIDC
- Other

### Description

Provide:

- Exact command executed
- Expected behavior
- Actual behavior
- Error messages (if any)

### Reproduction Steps

Provide a minimal set of steps to reproduce the issue on a fresh cluster.

If special configuration is required, include:

- Cluster configuration
- Installed components
- Resource manifests
- Required CRDs
- Namespace setup

---

## Reproducible Test Environments (KIND)

If the issue depends on cluster configuration, please provide instructions using KIND whenever possible.

KIND allows maintainers to reproduce issues quickly and consistently.

### Install KIND

https://kind.sigs.k8s.io/

### Create a Test Cluster

```bash
kind create cluster --name nuke-test
```

Verify access:

```bash
kubectl cluster-info
```

---

## Metrics Server Setup (Required for `nuke top`)

The Kubernetes Metrics Server is not installed by default in KIND.

Install it with:

```bash
kubectl apply -f https://github.com/kubernetes-sigs/metrics-server/releases/latest/download/components.yaml
```

KIND requires an additional patch to allow insecure TLS:

```bash
kubectl patch deployment metrics-server -n kube-system \
  --type='json' \
  -p='[
    {
      "op": "add",
      "path": "/spec/template/spec/containers/0/args/-",
      "value": "--kubelet-insecure-tls"
    }
  ]'
```

Wait for readiness:

```bash
kubectl rollout status deployment metrics-server -n kube-system
```

Verify:

```bash
kubectl top nodes
kubectl top pods
```

---

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

Example:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: example
spec:
  replicas: 1
  selector:
    matchLabels:
      app: example
  template:
    metadata:
      labels:
        app: example
    spec:
      containers:
      - name: example
        image: nginx
```

Apply with:

```bash
kubectl apply -f example.yaml
```

---

## Contributing Code

### General Guidelines

- Keep changes focused
- Prefer small, incremental pull requests
- Maintain Nushell-native style
- Avoid introducing external dependencies unless necessary

If your change alters behavior, update documentation accordingly.

---

## Adding or Improving Formatters

Formatters define how Kubernetes objects are presented.

Nuke uses three formatter categories:

- Resource formatters
- Rollout formatters
- Metric formatters

If a resource is not supported, you can implement a formatter.

### Workflow

1. Retrieve the raw object:

```nu
nuke show <resource> -o full
```

2. Design a structured output using Nushell pipelines

3. Implement the formatter closure

4. Test with real cluster data

5. Submit via pull request

Custom formatters can also be provided through environment variables:

- `NUKE_RESOURCE_FORMATTERS`
- `NUKE_ROLLOUT_FORMATTERS`
- `NUKE_METRIC_FORMATTERS`

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
