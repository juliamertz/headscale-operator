# User

The `User` resource creates an identity in your Headscale network. Users serve as the foundation for organizing devices and applying access control policies. Each device that joins your network is associated with a user, and ACL policies reference users to control network access.

## Example

```yaml
apiVersion: headscale.juliamertz.dev/v1
kind: User
metadata:
  name: kubernetes
spec:
  displayName: Kubernetes Service
  email: kubernetes@example.com
  headscaleRef:
    name: example
    namespace: headscale
```

## Fields

- `displayName`: Optional display name for the user
- `email`: Optional email address
- `pictureUrl`: Optional profile picture URL
- `headscaleRef`: Reference to the Headscale instance where the user should be created
