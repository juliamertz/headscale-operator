# PreauthKey

The `PreauthKey` resource generates authentication keys that allow devices to join your Headscale network without manual approval. When you need to automate device onboarding—whether for Kubernetes pods, virtual machines, or physical devices—preauth keys eliminate the need to manually approve each connection. The operator stores these keys in Kubernetes Secrets, making them easy to reference in deployments and configuration.

## Example

```yaml
apiVersion: headscale.juliamertz.dev/v1
kind: PreauthKey
metadata:
  name: example
spec:
  ephemeral: true
  reusable: true
  expiration: 99999h
  targetSecret: tailscale-agent-auth
  user:
    name: kubernetes
    namespace: headscale
```

## Fields

- `ephemeral`: Whether the key creates ephemeral nodes that disappear when disconnected (default: false)
- `reusable`: Whether the key can be used multiple times (default: false)
- `expiration`: Key expiration time in Go duration format (default: "1h")
- `targetSecret`: Name of the Secret to store the key in (optional, auto-generated if not specified)
- `user`: Reference to the User resource for which to generate the key
