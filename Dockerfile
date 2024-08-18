#FROM rust:1.76.0-bookworm as builder
#WORKDIR /app
#COPY Cargo.toml Cargo.lock ./
#RUN mkdir src && echo "fn main() {}" > src/main.rs
#RUN cargo build --release
#COPY . .
#RUN touch src/main.rs
#RUN cargo build --release
#RUN strip target/release/pved
#
#FROM debian:bookworm-20240311
#ENV GID=1000
#ENV SID=1000
#ENV USER=nonroot
#
#RUN groupadd -g $GID $USER \
#    && useradd -u $SID -g $GID -ms /bin/bash $USER
#WORKDIR /home/$USER
#RUN apt-get update && apt-get install --no-install-recommends -y openssl ca-certificates && apt-get clean autoclean && rm -rf /var/lib/{apt,dpkg,cache,log}/
#COPY --from=builder --chown=$USER:$USER /app/target/release/pved ./
#
#USER $USER
#ENTRYPOINT ["/bin/bash", "-c", "/home/$USER/pved"]

FROM debian:bookworm-20240311
ENV GID=1000
ENV SID=1000
ENV USER=nonroot

RUN groupadd -g $GID $USER \
    && useradd -u $SID -g $GID -ms /bin/bash $USER
WORKDIR /home/$USER
RUN apt-get update \
    && apt-get install --no-install-recommends -y openssl ca-certificates \
    && apt-get clean autoclean \
    && rm -rf /var/lib/{apt,dpkg,cache,log}/
COPY --chown=$USER:$USER /app/target/release/pved ./

USER $USER
ENTRYPOINT ["/bin/bash", "-c", "/home/$USER/pved"]