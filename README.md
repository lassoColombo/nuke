# Nuke: Nushell Kubernetes Integration

<p align="center">
  <img src=".doc/cover.png" alt="Nuke – A Nushell-native kubectl toolkit" width="100%" style="border-radius: 16px; box-shadow: 0 6px 24px rgba(0,0,0,0.25);" />
</p>

---

## Why?
Interacting with kubernetes looks too much like this:
```nu
# does not actually work
kubectl get po | detect columns --guess | update AGE {|po| $po.AGE | str replace 's' 'sec' | str replace 'm' 'min' | str replace 'h' 'hour' | str replace 'd' 'day' | into datetime }
| sort-by AGE
```

I wish i could just:
```nu
kubectl get po | sort-by AGE
```

Don't you?

---

## So What?
Nuke re-implements some of kubectl commands.  
It talks directly with the kube-apiserver to retrieve structured objects and typed data, so we can run things like:
```nu
nuke top po | sort-by memory
nuke get po -o wide | group-by node
```

- Nuke does not aim to reimplement all of kubectl. It focuses on the commands that wuould benefit from nushell's structured data.
- Nuke tries to mimick kubectl syntax to recreate a familiar environment. No need to learn a new tool.
- Nuke uses your kubeconfig as configuration. No additional setup is required.
- Nuke tries to adhere to kubectl semantics, integrating it with richer data.

### Implemented Commands

| Nuke Command | kubectl Equivalent | 
| --- | --- | 
| nuke get | kubectl get | 
| nuke http-get | kubectl get --raw |
| nuke api-resources | kubectl api-resources |
| nuke api-versions | kubectl api-versions |
| nuke rollout status | kubectl rollout status |
| nuke rollout history | kubectl rollout history |
| nuke top | kubectl top |
| nuke config | kubectl config |

---

## How Nuke Works

1. Reads your kubeconfig
2. Authenticates against the API server
3. Performs HTTP requests directly
4. Applies resource-specific formatter
5. Returns structured Nushell data

If no formatter is implemented, a default formatter is used.

### Authentication

Supported authentication methods:

- Bearer token authentication - as defined in the kubeconfig
- client certificate (mTLS) - as defined in the kubeconfig

Planned support:

- exec-plugins

# Installation

```nu
# Clone this repository into one of your NU_LIB_DIRS:
let nuke_basedir = ([($env.NU_LIB_DIRS | first) nuke] | path join)
git clone git@github.com:lassoColombo/nuke.git $nuke_basedir

# Verify installation:
use nuke
nuke api-resources
```

### Dependencies

- `curl` — used for HTTP communication with the API server

---

# Formatters

Commands that retrieve objects support three formats:

| Format | Description |
| --- | --- |
| compact | minimal view (Default for lists). |
| wide | extended attributes (Default for single objects). |
| full | the complete object from the API |


> Nuke is currently under active development, so not all resources have a dedicated formatter yet.  
> When a specific formatter isn’t available, Nuke automatically falls back to the default formatter.

### Custom Formatters

You can override formatters or implement new ones using environment variables:

- `NUKE_RESOURCE_FORMATTERS`
- `NUKE_ROLLOUTSTATUS_FORMATTERS`
- `NUKE_METRIC_FORMATTERS`

Example:

```nu
$env.NUKE_RESOURCE_FORMATTERS = {
  apps: { # api group (see 'nuke api-versions -o wide | get name')
    v1: { # api version (see 'nuke api-versions')
      deployments: {|output?: string = compact| # resource object (see 'nuke api-resources | get name')
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

# Nuke Http-Get
The http-get method performs an authenticated request to the kube API-server and returns the result as structured data without performing any additional parsing.

The request url can be specified as a string or as a record as expected by [url-join](https://www.nushell.sh/commands/docs/url_join.html).

```nu
# get pods
nuke http-get api/v1/namespaces/<namespace>/pods 

# get pods by label
nuke http-get {
  path: api/v1/namespaces/<namespace>/pods 
  params: [
    {key: labelSelector, value: 'my-label in (my-value-1, my-value-2)'}
  ]
}

# get aggregated api discovery
nuke http-get apis -H {
   Accept: "application/json;v=v2;g=apidiscovery.k8s.io;as=APIGroupDiscoveryList"
}
```

---

# Kubeconfig

Nuke provides utilities to manage your kubectl configuration, and to help you switch context swiftly.

### Context Switching
Context switching takes inspiration from [kubectx](https://github.com/ahmetb/kubectx) and [kubens](https://github.com/ahmetb/kubectx):
```nu
nuke config switch-namespace monitoring # explicit switch
nuke config switch-context # interactive switch - triggers input list
```

### Configuration Utilities
Nuke provides structured access to your kubeconfig data:
```nu
nuke config # returns the kubeconfig
nuke config path # get the path to the current detected kubeconfig
nuke config get-contexts # get all the contexts
nuke config get-contexts --current # get the current context
nuke config get-current-namespace # get the current namespace
nuke config get-clusters --current # get the current cluster
nuke config get-users --context k8s-001 # get the user of context k8s-001
nuke config get-cluster --context k8s-qa # get the cluster of context k8s-qa
```

---

## Roadmap

- Improve coverage of built-in resource formatters
- Implement `nuke describe` command
- Additional authentication methods
  - Exec plugins
- Watch functionality

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
