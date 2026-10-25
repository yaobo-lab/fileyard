FROM crpi-p35inr9lrgirg3t0.cn-hangzhou.personal.cr.aliyuncs.com/yaobo-box/rust-alpine:1.1 AS builder
WORKDIR /app
COPY . ./
RUN cargo build --release -p app

FROM crpi-p35inr9lrgirg3t0.cn-hangzhou.personal.cr.aliyuncs.com/yaobo-box/alpine:3.22.1

WORKDIR /app

RUN mkdir -p /app/etc /app/logs /app/uploads

COPY --from=builder /app/target/release/app /app/app
COPY etc/ /app/etc/

EXPOSE 3000 1883

CMD ["/app/app"]