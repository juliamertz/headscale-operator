# Headscale Operator

A Kubernetes operator for managing Headscale instances and related resources. The operator provides Custom Resource Definitions (CRDs) to declaratively manage Headscale deployments, users, preauth keys, and access control policies.

## Overview

The Headscale operator automates the deployment and management of Headscale, a self-hosted implementation of the Tailscale control server. It handles the creation of Headscale StatefulSets, user management, preauth key generation, ACL policy configuration, and provides a mutating admission webhook for injecting Tailscale sidecars into pods.

## Custom Resources

The operator provides four Custom Resource Definitions:

- **[Headscale](docs/headscale.md)**: Manages Headscale instance deployments
- **[User](docs/user.md)**: Creates and manages users in Headscale instances
- **[PreauthKey](docs/preauth-key.md)**: Generates authentication keys for users
- **[ACLPolicy](docs/acl-policy.md)**: Manages access control rules

Additionally, the operator provides a **[Tailscale sidecar injection](docs/tailscale-sidecar.md)** feature via a mutating admission webhook.

## Architecture

### Config Manager

Each Headscale StatefulSet includes a config-manager sidecar that:
- Watches the ACL ConfigMap for changes
- Writes ACL files to a shared emptyDir volume
- Sends SIGHUP signals to the Headscale process to trigger configuration reloads

The config-manager runs with minimal RBAC permissions, only requiring read access to the specific ACL ConfigMap.

### Resource Lifecycle

All resources use finalizers to ensure proper cleanup:
- Headscale: Removes StatefulSet, Service, ConfigMaps, Secrets, and RBAC resources
- User: Destroys the user in Headscale before deletion
- PreauthKey: Revokes the key in Headscale before deletion
- ACLPolicy: Removes the policy from the ACL ConfigMap
