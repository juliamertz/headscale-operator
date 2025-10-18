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
- [ ] Subnet router
- [ ] Api server proxy
- [ ] Tailscale sidecar
- [ ] Figure out how to connect to headscale GRPC api
