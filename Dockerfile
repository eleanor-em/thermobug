FROM debian:buster-slim
RUN apt update \
  && apt install -y libssl-dev \
  && rm -rf /var/lib/apt/lists/*
EXPOSE 8111
RUN ["touch", ".env"]
COPY ./target/release/thermobug /usr/local/bin/thermobug
CMD ["thermobug"]
