# Docker testing on Ubuntu
* To build 

```shell
docker build    --tag links_on_ubuntu_image \
                --build-arg RUN_UID=$(id -u) \
                --build-arg RUN_UNAME=$(whoami) \
                --build-arg RUN_GID=$(id -g) ./dev-how-to/ubuntu-pod
```

* To run
```shell
# the cap-add are required for tshark to see eth0 and other network interfaces
docker run \
    --rm --interactive --tty \
    --user "$(id -u)":"$(id -g)" \
    --volume "$(pwd)"/..:/home/$(whoami)/dev \
    --workdir /home/$(whoami)/dev \
    --name links_on_ubuntu_pod \
    --cap-add=NET_RAW --cap-add=NET_ADMIN -it \
    links_on_ubuntu_image
```

* To install nextest
```shell
docker exec \
    --interactive --tty \
    links_on_ubuntu_pod \
    bash -c "curl -LsSf https://get.nexte.st/latest/linux-arm | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin"
```

* To run tests
```shell
docker exec \
    --interactive --tty \
    links_on_ubuntu_pod \
    bash -c " \
    rustup default nightly;  \
    pushd links; \
    cargo nextest run --all-features; \
    cargo nextest run --examples; \
    cargo test --doc; \
    "
```
* To run bench mark
```shell
docker exec \
    --interactive --tty \
    links_on_ubuntu_pod \
    bash -c " \
    rustup default nightly;  \
    pushd links; \
    cargo bench --all-features ;\
    "
```
* To run shark

```shell
docker exec \
    --interactive --tty \
    links_on_ubuntu_pod \
    sudo tshark --interface lo -f "tcp port 8080"
```
