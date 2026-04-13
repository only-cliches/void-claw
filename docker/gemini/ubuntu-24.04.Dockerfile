# void-claw + Google Gemini CLI — Ubuntu 24.04 LTS
#
# Build (from repo root — must have already built the base image):
#   docker build -t void-claw-gemini:ubuntu-24.04 -f docker/gemini/ubuntu-24.04.Dockerfile .

FROM my-agent:ubuntu-24.04

# Install Google Gemini CLI.
USER root
RUN npm install -g @google/gemini-cli
USER ubuntu

CMD ["gemini"]
