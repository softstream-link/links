# bench
```shell
cargo bench
```

# local build & test rust only
```shell
cargo nextest run --all-features &&
cargo nextest run --examples --all-features &&
cargo test --doc --all-features &&
cargo doc --all-features &&
cargo clippy --all-features -- --deny warnings
```

# Local build & test rust & python extension
* `links_bindings_python` will use `micromamba` env which has `python, maturin, pytest`
```shell
if [ -d links_bindings_pythons ] ; then CWD="./../.." ; else CWD="."; fi ; echo ${CWD}
cd ${CWD} ; cd bindings/python/ 
micromamba create --name links_build_env --yes python maturin pytest &&
micromamba run --name links_build_env cargo nextest run --all-features &&
micromamba run --name links_build_env cargo nextest run --examples --all-features && 
micromamba run --name links_build_env cargo test --doc --all-features &&
micromamba run --name links_build_env cargo clippy --all-features -- --deny warnings &&
micromamba run --name links_build_env cargo doc --all-features &&
micromamba run --name links_build_env maturin develop &&
micromamba run --name links_build_env pytest
```

# Regenerate `links_connect.pyi` file
```shell    
if [ -d links_bindings_pythons ] ; then CWD="./../.." ; else CWD="."; fi ; echo ${CWD}
cd ${CWD} ; cd bindings/python/
micromamba run --name links_build_env pip install cogapp
micromamba run --name links_build_env cog -r ./links_bindings_python/links_bindings_python.pyi
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
