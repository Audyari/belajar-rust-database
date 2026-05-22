# Dockerfile
FROM alpine:latest

# Install CA certificates (buat koneksi HTTPS)
RUN apk add --no-cache ca-certificates

# Buat folder app
WORKDIR /app

# Copy binary Rust yang udah di-compile
COPY target/release/belajar-rust-datetime.exe /app/app

# Kasih permission execute
RUN chmod +x /app/app

# Jalankan binary
CMD ["/app/app"]