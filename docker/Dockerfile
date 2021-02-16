FROM rust:alpine
WORKDIR /usr/src/thermobug
COPY .. .
RUN cargo install --path .alpine
CMD ["thermobug"]