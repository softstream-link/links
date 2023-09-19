FROM mcr.microsoft.com/devcontainers/rust:latest

RUN rustup toolchain install nightly

RUN apt update && apt install -y wireshark tshark && apt clean