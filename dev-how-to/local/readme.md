# Run all tests

```shell
cargo nextest run --all-features
cargo nextest run --examples
cargo test --doc

cargo doc

# cargo bench --all-features
```

# Run all benchmarks
```shell
cargo bench --all-features
```

# Create docs
```shell
cargo doc
```

# Run Clippy
```
cargo clippy --all-features -- --deny warnings
```