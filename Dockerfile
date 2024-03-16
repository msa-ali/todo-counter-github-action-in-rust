# syntax=docker/dockerfile:1
FROM rust:1.73-alpine as builder
RUN apk add --no-cache musl-dev
WORKDIR /usr/src/left-todo-action
COPY --link . .
RUN cargo install --path .

FROM scratch
COPY --link --from=builder /usr/local/cargo/bin/left-todo-action /usr/local/bin/left-todo-action
ENTRYPOINT ["/usr/local/bin/left-todo-action"]