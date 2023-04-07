FROM rust:latest

# Install docker
RUN curl -sSL https://get.docker.com/ | sh

# Setup ports
ENV PORT=8080
EXPOSE 8080

# Copy project
WORKDIR /usr/src/app
COPY . .

# Install the App
RUN cargo install --path .

# Run the App
CMD ["compiler"]
