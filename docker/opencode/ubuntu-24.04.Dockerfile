# agent-zero + opencode — Ubuntu 24.04 LTS
#
# opencode npm package name: verify at https://opencode.ai before building.
#
# Build (from repo root — must have already built the base image):
#   docker build -t agent-zero-opencode:ubuntu-24.04 -f docker/opencode/ubuntu-24.04.Dockerfile .

FROM my-agent:ubuntu-24.04

# Install opencode CLI.
USER root
RUN npm install -g opencode-ai
USER ubuntu

CMD ["opencode"]
