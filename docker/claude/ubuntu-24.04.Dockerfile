# agent-zero + Claude Code CLI — Ubuntu 24.04 LTS
#
# Build (from repo root — must have already built the base image):
#   docker build -t agent-zero-claude:ubuntu-24.04 -f docker/claude/ubuntu-24.04.Dockerfile .
#
# Or build both in one step:
#   docker build -t my-agent:ubuntu-24.04 -f docker/ubuntu-24.04.Dockerfile . \
#   && docker build -t agent-zero-claude:ubuntu-24.04 -f docker/claude/ubuntu-24.04.Dockerfile .

FROM my-agent:ubuntu-24.04

# Install Claude Code CLI.

USER ubuntu

RUN curl -fsSL https://claude.ai/install.sh | bash

# Ensure claude is on PATH for all shell types (login, non-login,
# non-interactive scripts).  The installer adds it to .bashrc, but
# that is only sourced by interactive bash shells.
ENV PATH="/home/ubuntu/.local/bin:${PATH}"

CMD ["claude"]
