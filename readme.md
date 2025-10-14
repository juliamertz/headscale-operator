# Kubernetes operator for [Headscale](https://github.com/juanfont/headscale)

## Apply CRD's

```sh
cargo run -- crd | kubectl apply -f-
```

## Todo

- [x] Headscale statefulset
- [x] Aclpolicy
- [x] Preauth keys
- [ ] Subnet router
- [ ] Tailscale sidecar
- [ ] Api server proxy

