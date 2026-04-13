# void-claw + OpenAI Codex CLI — Ubuntu 24.04 LTS
#
# Build (from repo root — must have already built the base image):
#   docker build -t void-claw-codex:ubuntu-24.04 -f docker/codex/ubuntu-24.04.Dockerfile .

FROM my-agent:ubuntu-24.04

# Install Codex plus an explicit arch alias package.
# The generic package provides the `codex` bin, while the alias package
# guarantees the platform payload exists even if optional dependency
# resolution is skipped during global install.
USER root
RUN set -eu; \
    case "$(dpkg --print-architecture)" in \
        amd64) codex_payload_alias="@openai/codex-linux-x64@npm:@openai/codex@linux-x64" ;; \
        arm64) codex_payload_alias="@openai/codex-linux-arm64@npm:@openai/codex@linux-arm64" ;; \
        *) echo "unsupported architecture for Codex" >&2; exit 1 ;; \
    esac; \
    npm install -g @openai/codex "$codex_payload_alias"
USER ubuntu

CMD ["codex"]
