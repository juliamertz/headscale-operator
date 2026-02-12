# Tailscale Sidecar Injection

The operator includes a mutating admission webhook that automatically injects Tailscale sidecars into pods. This makes your Kubernetes services reachable through your Headscale network without modifying pod specifications or manually adding sidecar containers. Annotate a pod to enable injection, and the webhook adds the Tailscale container with the necessary configuration.

## Example

```yaml
apiVersion: v1
kind: Pod
metadata:
  annotations:
    headscale.juliamertz.dev/tailscale-inject-sidecar: "true"
    headscale.juliamertz.dev/tailscale-auth-secret: tailscale-agent-auth
    headscale.juliamertz.dev/tailscale-extra-args: --accept-routes --advertise-tags=tag:sidecar
    headscale.juliamertz.dev/tailscale-image: ghcr.io/tailscale/tailscale:v1.92.4
spec:
  containers:
    - name: app
      image: nginx:1.14.2
```

## Annotations

- `headscale.juliamertz.dev/tailscale-inject-sidecar`: Set to `"true"` to enable injection (required)
- `headscale.juliamertz.dev/tailscale-auth-secret`: Name of the Secret containing the preauth key (required)
- `headscale.juliamertz.dev/tailscale-extra-args`: Additional arguments to pass to Tailscale (optional)
- `headscale.juliamertz.dev/tailscale-image`: Tailscale container image (optional, defaults to `ghcr.io/tailscale/tailscale:v1.92.4`)
- `headscale.juliamertz.dev/tailscale-resources`: Tailscale container resources (stringified json resource requirements)
