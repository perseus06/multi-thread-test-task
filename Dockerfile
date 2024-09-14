# Dockerfile
FROM rust:latest

WORKDIR /app

# Copy the source code into the Docker container
COPY . .

# Build the Rust application
RUN cargo build --release

# Command to run the application
CMD ["./target/release/rust-image-server"]
