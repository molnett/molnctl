FROM ubuntu:22.04

# Install packages
RUN apt-get update && \
    apt-get install -y curl wget git && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy source code
COPY . .

# Build steps
RUN echo "Building application..." && \
    sleep 2 && \
    echo "Compilation step 1..." && \
    sleep 1 && \
    echo "Compilation step 2..." && \
    sleep 1 && \
    echo "Build complete!"

# Expose port
EXPOSE 8080

# Set entrypoint
ENTRYPOINT ["echo", "Complex build test completed!"]