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
if [ -d links_bindings_pythons ] ; then CWD="./../.." ; else CWD="."; fi ; echo ${CWD} ;cd ${CWD}
micromamba create --name links_build_env --yes maturin &&
micromamba run --name links_build_env --cwd ./bindings/python cargo nextest run --all-features &&
micromamba run --name links_build_env --cwd ./bindings/python cargo nextest run --examples --all-features && 
micromamba run --name links_build_env --cwd ./bindings/python cargo test --doc --all-features &&
micromamba run --name links_build_env --cwd ./bindings/python cargo clippy --all-features -- --deny warnings &&
micromamba run --name links_build_env --cwd ./bindings/python cargo doc --all-features &&
micromamba run --name links_build_env --cwd ./bindings/python maturin develop --extras test &&
micromamba run --name links_build_env --cwd ./bindings/python pytest
```

# Testing release extension
* test with minimum python version `3.11`
* NOTE: must have `links_build_env` already created from prior step
```shell
if [ -d links_bindings_pythons ] ; then CWD="./../.." ; else CWD="."; fi ; echo ${CWD} ;cd ${CWD}
micromamba create --name links_test_env --yes python=3.11 pytest &&
(rm -f ./target/wheels/*.whl || true) &&
micromamba run --name links_build_env --cwd ./bindings/python maturin build --release &&
micromamba run --name links_test_env  pip install --ignore-installed ./target/wheels/*.whl &&
micromamba run --name links_test_env  --cwd ./bindings/python pytest
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
