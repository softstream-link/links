# Docker testing on Ubuntu
## To build 

```shell
docker build    --tag links_on_ubuntu_image \
                --build-arg RUN_UID=$(id -u) \
                --build-arg RUN_UNAME=$(whoami) \
                --build-arg RUN_GID=$(id -g) ./readme/dev-how-to/ubuntu-pod
```

## To run
```shell
# the cap-add are required for tshark to see eth0 and other network interfaces
docker run \
    --rm --interactive --tty \
    --user "$(id -u)":"$(id -g)" \
    --volume "$(pwd)":/home/$(whoami)/links \
    --workdir /home/$(whoami)/links \
    --name links_on_ubuntu_pod \
    --cap-add=NET_RAW --cap-add=NET_ADMIN -it \
    links_on_ubuntu_image
```

## To run tests
```shell
docker exec \
    --interactive --tty \
    links_on_ubuntu_pod \
    bash -c " \
    rustup default stable ; \
    pushd links ; \
    cargo nextest run --all-features ; \
    cargo nextest run --examples --all-features ; \
    cargo test --doc --all-features; \
    cargo doc --all-features; \
    "
```
## To run specific test 
```shell
docker exec \
    --interactive --tty \
    links_on_ubuntu_pod \
    bash -c " \
    rustup default stable;  \
    pushd links; \
    cargo nextest run --all-features -- test_svc_clt_connected_spawned_recver_ref
    "
```

## To run bench mark
```shell
docker exec \
    --interactive --tty \
    links_on_ubuntu_pod \
    bash -c " \
    rustup default stable;  \
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
