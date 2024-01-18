# bench
```shell
cargo bench
```

# local build & test rust only
```shell
cargo nextest run --all-features &&
cargo nextest run --examples --all-features &&
cargo test --doc --all-features &&
cargo  doc --all-features &&
cargo clippy --all-features -- --deny warnings
```

# Local build & test rust & python extension
* `links_bindings_python` will use `micromamba` env which has `python, maturin, pytest`
```shell
micromamba create --name links_build_env --yes python maturin pytest &&
micromamba run --name links_build_env cargo nextest run --all-features &&
micromamba run --name links_build_env cargo nextest run --examples --all-features && 
micromamba run --name links_build_env cargo test --doc --all-features &&
micromamba run --name links_build_env cargo clippy --all-features -- --deny warnings &&
micromamba run --name links_build_env cargo doc --all-features &&
micromamba run --name links_build_env --cwd ./bindings/python maturin develop &&
micromamba run --name links_build_env --cwd ./bindings/python pytest
```

# Regenerate `links_connect.pyi` file
```shell    
micromamba run --name links_build_env --cwd ./bindings/python/links_bindings_python pip install cogapp
micromamba run --name links_build_env --cwd ./bindings/python/links_bindings_python cog -r links_bindings_python.pyi
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
