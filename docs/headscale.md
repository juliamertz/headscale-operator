# Headscale

The `Headscale` resource deploys a Headscale control server in your Kubernetes cluster. Instead of using Tailscale's hosted service, this lets you run your own control plane with full control over your network topology, user management, and access policies.

## Example

```yaml
apiVersion: headscale.juliamertz.dev/v1
kind: Headscale
metadata:
  name: example
spec:
  tls:
    existingSecret: headscale-tls
  config:
    server_url: https://headscale.domain.com:30443
    tls_cert_path: /etc/headscale/tls/tls.crt
    tls_key_path: /etc/headscale/tls/tls.key
    # ... additional Headscale configuration
  deployment:
    image: headscale/headscale:v0.27.1
    env:
      - name: HEADSCALE_DATABASE_TYPE
        value: postgres
      # ... additional environment variables
```

## Configuration

The `spec.config` field accepts a JSON object that is passed directly to Headscale. See the [Headscale documentation](https://github.com/juanfont/headscale) for available configuration options.

### TLS Configuration

TLS certificates must be provided via an existing Secret. Specify the secret name in `spec.tls.existingSecret`. The Secret should contain `tls.crt` and `tls.key` keys.

### Database Configuration

Database connection details are configured via environment variables in `spec.deployment.env`. The operator does not manage database instances; you must provide an external database.
