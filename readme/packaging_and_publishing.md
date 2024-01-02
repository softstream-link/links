# bench
```shell
cargo bench
```

# local build
```shell
cargo nextest run --all-features &&
cargo nextest run --examples --all-features &&
cargo test --doc --all-features &&
cargo --config 'env.RUSTDOCFLAGS="-D warnings"' doc --all-features &&
cargo clippy --all-features -- --deny warnings
```

# bench nonblocking only 
```shell
pushd nonblocking &&
cargo bench --all-features &&
popd
```

# test book nonblocking
```shell
mdbook test
```
```shell
mdbook serve --open
```
