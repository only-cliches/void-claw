# void-claw default image
#
# Uses the shared Ubuntu base (`void-claw-base:local`) so strict-network
# and proxy bootstrap behavior stays consistent with manager-launched images.
#
# Copy this file to create per-project variants, e.g.:
#   rust.dockerfile

FROM void-claw-base:local

USER root
RUN npm install -g @openai/codex @google/gemini-cli opencode-ai
USER ubuntu

RUN curl -fsSL https://claude.ai/install.sh | bash
ENV PATH="/home/ubuntu/.local/bin:${PATH}"

CMD ["bash"]
