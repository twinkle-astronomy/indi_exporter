FROM rust:1.72-buster as dev

RUN apt-get update && apt-get install -y --no-install-recommends \
    libcfitsio-dev \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir /app
ENV HOME="/app"
WORKDIR /app

#This user schenanigans allows for local development
ARG USER=app
ARG USER_ID=1000
ARG GROUP_ID=1000

RUN groupadd -g ${GROUP_ID} ${USER} && \
    useradd -l -u ${USER_ID} -g ${USER} ${USER}

RUN chown ${USER}:${USER} /app
USER ${USER}

FROM dev as tester
COPY . /app
RUN cargo build --release --all-targets

FROM tester as builder
COPY . /app
RUN cargo install --path .

FROM debian:bullseye-slim as release
COPY --from=builder /usr/local/cargo/bin/indi_exporter /usr/local/bin/

