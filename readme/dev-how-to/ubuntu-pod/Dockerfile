FROM mcr.microsoft.com/devcontainers/rust:latest

ARG RUN_UID
ARG RUN_UNAME
ARG RUN_GID


RUN rustup toolchain install stable
RUN rustup toolchain install nightly

RUN apt update && apt install -y wireshark tshark && apt clean

RUN curl -LsSf https://get.nexte.st/latest/linux-arm | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin

RUN useradd --no-log-init \
                --create-home \
                --shell /bin/bash \
                --uid ${RUN_UID}  \
                --gid ${RUN_GID} \
                ${RUN_UNAME}

RUN usermod --append --groups sudo ${RUN_UNAME}
RUN chmod a+r /etc/sudoers
RUN mkdir -p /home/${RUN_UNAME}/.local/bin
RUN export HOME=/home/${RUN_UNAME}; export CONDA_FORGE_YES="yes"; curl -L micro.mamba.pm/install.sh | bash
RUN chown -R ${RUN_UNAME}:${RUN_GID} /home/${RUN_UNAME}/micromamba

RUN echo "\
${RUN_UNAME} ALL = NOPASSWD : ALL \
" >> /etc/sudoers


RUN echo " \
# user alias \
alias ll='ls -latr' \
" >> /home/${RUN_UNAME}/.bashrc
