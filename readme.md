# Kubernetes operator for [Headscale](https://github.com/juanfont/headscale)

## Apply CRD's

```sh
cargo run -- crd | kubectl apply -f-
```

## Todo

- [x] Headscale
- [x] Aclpolicy
- [x] PreauthKey
- [x] User
- [x] Tailscale sidecar
- [ ] Subnet router
- [ ] Api server proxy
- [ ] Figure out how to connect to headscale GRPC api
