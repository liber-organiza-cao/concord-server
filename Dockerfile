FROM rust:alpine AS build

RUN apk add musl-dev
RUN mkdir -p /source
WORKDIR /source
COPY . .

RUN cargo build --release --target=x86_64-unknown-linux-musl

FROM scratch

COPY --from=build /source/target/x86_64-unknown-linux-musl/release/concord-server /bin/concord-server

CMD ["/bin/concord-server"]