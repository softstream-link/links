# Docker testing on Ubuntu
* To build 

```shell
docker build    --tag links_on_ubuntu_image \
                --build-arg RUN_UID=$(id -u) \
                --build-arg RUN_UNAME=$(whoami) \
                --build-arg RUN_GID=$(id -g) ./.devcontainer/ 
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

* To run shark

```shell
docker exec \
    --interactive --tty \
    links_on_ubuntu_pod \
    sudo tshark --interface lo -f "tcp port 8080"
```
