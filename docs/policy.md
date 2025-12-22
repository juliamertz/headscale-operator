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
    namespace: default
  groups:
    group:admins:
      - julia
      - admin
  tagOwners:
    tag:kubernetes:
      - kubernetes
  acls:
    - action: accept
      src: ['kubernetes@']
      dst: ['homelab@:80,443']
    - action: accept
      src: ['julia@']
      dst: ['*:*']
    - action: accept
      src: ['group:admins']
      dst: ['tag:kubernetes:*']
```

## Fields

- `headscaleRef`: Reference to the Headscale instance this policy applies to (required)
  - `name`: Name of the Headscale resource
  - `namespace`: Namespace of the Headscale resource (optional, defaults to the same namespace as the Policy)
- `hosts`: Map of hostname aliases to IP addresses or CIDR ranges (required)
- `acls`: Array of access control rules defining allowed/denied traffic (required)
  - `action`: Either `accept` or `deny`
  - `src`: Array of source identifiers (users, groups, tags, or IPs)
  - `dst`: Array of destination identifiers (users, groups, tags, IPs, or ports)
- `groups`: Map of group names to arrays of user identifiers (optional)
- `tagOwners`: Map of tag names to arrays of user identifiers that can own devices with those tags (optional)
