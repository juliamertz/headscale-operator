# Headscale Operator

A Kubernetes operator for managing Headscale instances and related resources. The operator provides Custom Resource Definitions (CRDs) to declaratively manage Headscale deployments, users, preauth keys, and access control policies.

## Installation

The Headscale operator can be installed using Helm from the chart repository:

> [!WARNING]
> This operator is still in development and CRD's are likely to change, install at your own peril 

```bash
helm repo add headscale-operator https://charts.juliamertz.dev
helm repo update
helm install headscale-operator headscale-operator/headscale-operator \
    --version v0.0.2 \
    --namespace headscale-operator \
    --create-namespace \
    --set crds.install=true
```

## Custom Resources

The operator provides four Custom Resource Definitions:

- **[Headscale](docs/headscale.md)**: Manages Headscale instance deployments
- **[User](docs/user.md)**: Creates and manages users in Headscale instances
- **[PreauthKey](docs/preauth-key.md)**: Generates authentication keys for users
- **[Policy](docs/policy.md)**: Manages access control rules

Additionally, the operator provides a **[Tailscale sidecar injection](docs/tailscale-sidecar.md)** feature via a mutating admission webhook.
