FROM mcr.microsoft.com/devcontainers/base:bullseye

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends build-essential pkg-config libssl-dev binaryen \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*
