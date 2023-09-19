# Docker testing on Ubuntu
* To build 

```shell
docker build --tag links_on_ubuntu ./.devcontainer/
```

* To run
```shell
# the cap-add are required for tshark to see eth0 and other network interfaces
docker run --name links_on_ubuntu_pod --cap-add=NET_RAW --cap-add=NET_ADMIN -it  links_on_ubuntu
```