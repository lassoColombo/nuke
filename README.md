# Nuke

A Nushell-native Kubernetes toolkit

<p align="center">
  <img src=".doc/cover.png" alt="Nuke – A Nushell-native kubectl toolkit" width="100%" style="border-radius: 16px; box-shadow: 0 6px 24px rgba(0,0,0,0.25);" />
</p>

---

## Overview

Have you ever done something like
```nu
kubectl get po | detect columns --guess
kubectl get po -o yaml | from yaml
```
hoping you could simply query and inspect data from the kube API-server?

Maybe you have something like this in your config
```nu
def kk [resource resourcename --namespace: string] {
    kubectl get $resource $resourcename --namespace $namespace | detect columns --guess 
    | update AGE {|p| $p.AGE | str replace s sec | str replace m min | str replace h hr | str replace d day | into datetime }
    | sort-by AGE
}
```
And you wish you could just `kubectl get po | sort-by age`? Me too.  

That's why i created Nuke (that and to learn about kubernetes).  
Nuke talks directly to the Kubernetes API server and returns **structured, typed, queryable objects**, so you can just
```nu
nuke get po | where restarts != 0 | sort-by restarts
nuke get po --all -l 'run in (production, quality)' -o wide | group-by node
```

> Nuke tries to adhere the semantics of kubectl, integrating it with richer data.
> At the same time it allows you to create custom formatters for your favourite resources and commands.

> Nuke does not aim to reimplement all of kubectl.  
> Instead, it focuses on those commands where Nushell’s structured data provides the most value.

> All flags, resources and resource names support autocompletion.

---

## Implemented Commands

| Nuke Command           | kubectl Equivalent        |
|------------------------|---------------------------|
| `nuke get`             | `kubectl get`             |
| `nuke api-resources`   | `kubectl api-resources`   |
| `nuke api-versions`    | `kubectl api-versions`    |
| `nuke rollout status`  | `kubectl rollout status`  |
| `nuke rollout history` | `kubectl rollout history` |
| `nuke top`             | `kubectl top`             |
| `nuke config`          | `kubectl config`          |

---

## Installation

Clone this repository into one of your Nushell library directories (`$env.NU_LIB_DIRS`):
```nu
let random_lib_dir = ([($env.NU_LIB_DIRS | first) nuke] | path join)
git clone git@github.com:lassoColombo/nuke.git $random_lib_dir
```

Verify installation:
```nu
use nuke
nuke api-resources
```

---

## Output Formats

Commands that retrieve objects support three formats:

| Format      | Description                                                   |
|-------------|---------------------------------------------------------------|
| `compact`   | Similar to `kubectl get` |
| `wide`      | Similar to `kubectl describe`                              |
| `full`      | Complete object returned by the Kubernetes API server         |


The compact format is the default when retrieving a list of objects, while wide is the default for single objects.

---

## How Nuke Works

1. Reads your kubeconfig
2. Authenticates against the API server
3. Performs HTTP requests directly
4. Applies resource-specific formatter
5. Returns structured Nushell data

If no formatter exists, a default formatter is used.

---

## Custom Formatters

Formatters control how Kubernetes objects are displayed.

You can override or define custom formatters using environment variables:

- `NUKE_RESOURCE_FORMATTERS`
- `NUKE_ROLLOUT_FORMATTERS`
- `NUKE_METRIC_FORMATTERS`

Example:

```nu
$env.NUKE_RESOURCE_FORMATTERS = {
  apps: {
    v1: {
      deployments: {|output?: string = compact|
        let obj = $in
        let res = {
          name: $obj.metadata.name
          namespace: $obj.metadata.namespace
          containers: ($obj.spec.template.spec.containers | length)
        }
        if $output == compact {
          return ($res
            | insert containers ($obj.spec.template.spec.containers | length)
          )
        }
        $res 
        | insert containers $obj.spec.template.spec.containers
      }
    }
  }
}
```

---

## Config Module

Nuke provides utilities to manage kubeconfig contexts and namespaces.

##### Switch namespace

```nu
nuke config switch-namespace my-namespace
# If no argument is provided, you will be prompted to choose one in the builtin fuzzyfinder
```

##### Switch context

```nu
nuke config switch-context my-context
# If no argument is provided, you will be prompted to choose one in the builtin fuzzyfinder
```

##### Edit kubeconfig

```nu
nuke config edit
```

##### Additional helpers:
```nu
nuke config get-contexts
nuke config get-clusters
nuke config get-users
nuke config
```

---

## Configuration

Nuke uses your existing Kubernetes configuration:

```
$env.KUBECONFIG  (defaults to ~/.kube/config)
```

No additional setup required.

### Cache Location

Nuke follows the XDG Base Directory Specification:

```
$XDG_CACHE_HOME or ~/.cache
```

---

## Authentication

Supported authentication methods:

- Token-based credentials
- Client certificate authentication

Planned support:

- OIDC
- Exec plugins

---

## Dependencies

External tools required:

- `curl` — used for HTTP communication with the API server

Nuke otherwise aims to remain fully Nushell-native.

---

## Updating

Pull the latest changes:

```nu
cd ([($env.NU_LIB_DIRS | first) nuke] | path join)
git pull
```

---

## Contributing

Contributions, bug reports, and feature requests are welcome.

Before opening an issue or pull request, please read: [CONTRIBUTING.md](CONTRIBUTING.md)

The contributing guide includes:

- Development setup
- How to reproduce bugs
- KIND cluster configuration
- Metrics server setup
- Formatter development guidelines

---

## Roadmap

- Improve coverage of built-in resource formatters
- Additional authentication methods
  - OIDC
  - Exec plugins
- Watch functionality
