FROM docker.io/library/rust:1.76-slim-bookworm AS build
WORKDIR /app

COPY . .
RUN cargo build --release --locked


FROM docker.io/library/debian:bookworm-slim

EXPOSE 8080
ENV LISTEN_ADDR=0.0.0.0:8080
CMD ["/usr/local/bin/feedfilter"]

COPY --from=build /app/target/release/feedfilter /usr/local/bin/feedfilter
