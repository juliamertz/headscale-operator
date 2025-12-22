# Policy

The `Policy` resource defines which users can communicate with each other and on which ports. Without ACL policies, all devices in your Headscale network can reach each other. Policies let you segment your network, restrict access between teams or environments, and limit service-to-service communication to only the ports that are necessary.

## Example

```yaml
apiVersion: headscale.juliamertz.dev/v1alpha1
kind: Policy
metadata:
  name: example
spec:
  headscaleRef:
    name: example
  rules:
    - action: accept
      src: ['kubernetes@']
      dst: ['homelab@:80,443']
    - action: accept
      src: ['julia@']
      dst: ['*:*']
```

## Fields

- `headscaleRef`: Reference to the Headscale instance to apply the policy to
- `rules`: Array of access control rules

### Rule Fields

- `action`: Either `accept` or `deny`
- `src`: Array of source user or group identifiers (e.g., `['user@', 'group:admins']`)
- `dst`: Array of destination identifiers with optional ports (e.g., `['user@:80,443', '*:*']`)
