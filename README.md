<div align="center">

# 🕵️ Void Claw 🛡️

**Zero-trust container manager for AI coding agents**

Open-source, local-first alternative inspired by [Coder Agent Firewall](https://coder.com/docs/ai-coder/agent-firewall).

</div>

![Void Claw Demo showing launching an agent and the approval dialog.](https://github.com/only-cliches/void-claw/blob/main/demo.gif?raw=true)

AI coding agents are powerful, and by default, completely unconstrained. Give one your terminal and it has your machine: your files, your credentials, and your network. Void Claw enforces a zero-trust boundary around every agent session, running agents in isolated Docker containers with policy-enforced access to your code, your host, and the outside world. Nothing gets through without a rule that allows it.

## Key Features

* **Isolated Docker Environments**: Agents run in locked-down Docker containers, fully separated from your host system.
* **Zero-Trust Network Proxy**: A built-in MITM proxy intercepts all outbound HTTP and HTTPS traffic. Every request is evaluated against your policy: auto-allowed, denied, or escalated to you for approval in real time.
* **Controlled Host Execution (`hostdo`)**: Agents have no direct access to your machine. Instead, they request specific pre-approved host commands via `hostdo` (e.g. `cargo test`, `npm run build`). You approve or deny each class of command, once or permanently.
* **Interactive Terminal UI (TUI)**: Manage everything from a single terminal interface. View active containers, inspect logs, review and action pending network and host requests, and drop into a live terminal session when needed.
* **Ready-to-Use Agent Profiles**: First-class support for Claude Code, OpenAI Codex, Google Gemini CLI, and Opencode, including automatic auth state mounting so agents don't need to re-authenticate on every launch.
* **OpenTelemetry Logging**: Export hostdo, proxy, and startup traces to your collector with configurable OTLP settings, while keeping local rotating logs on disk.
* **Keyboard-First Navigation**: The app is a terminal-native TUI with keyboard-friendly navigation and controls across workspaces, sessions, prompts, and settings.

## Getting Started

### Prerequisites

Void Claw requires
1. [Docker](https://www.docker.com/get-started/) to be installed and available in your system's `PATH`
2. The [Rust programming language](https://rust-lang.org/tools/install/) to be insatlled.

### Install

```bash
git clone https://github.com/only-cliches/void-claw
cd void-claw
cargo install --path .
```

### 1. Initialization

Run `void-claw` from any directory to generate your starter configuration:

```bash
void-claw
```

This will prompt you to create a `void-claw.toml` file, populated with sensible defaults. Void Claw will use `./docker` as your Docker build directory if it exists, or fall back to `~/.config/void-claw/docker` and create it on first run. If the built-in Dockerfiles are missing, Void Claw will offer to fetch them from GitHub automatically.

### 2. Add a Workspace

Add a workspace from within the TUI, or by adding a `[[workspaces]]` block to your `void-claw.toml` (legacy `[[projects]]` still works).

When a workspace is registered, Void Claw writes a `void-rules.toml` to the root of your repository. This file defines the security policy for any agent operating in that codebase: which host commands it may request and which network destinations it may reach. Commit it to version control so your policy travels with the code.

### 3. Launch an Agent

From the TUI, use the arrow keys (or `j`/`k`) to navigate to a workspace and press `Enter` to launch an agent container.

## Supported Agents

Void Claw is designed to be flexible. It ships with first-class support for the most popular AI coding assistants, but any agent that runs in Docker will work.

### Out-of-the-Box Support

* **Claude Code** (`@anthropic-ai/claude-code`)
* **OpenAI Codex** (`@openai/codex`)
* **Google Gemini CLI** (`@google/gemini-cli`)
* **Opencode** (`opencode-ai`)

For these agents, Void Claw automatically bind-mounts authentication and session state (e.g. `~/.claude`, `~/.gemini`) so agents authenticate once and stay authenticated across container restarts.

### Bring Your Own Agent (BYOA)

Any agent that can run in a Docker container works with Void Claw. Define custom containers in `void-claw.toml` and set the agent type to `none` to skip built-in config injection, then mount your own images, environment variables, and configuration files.

## Configuration

Void Claw uses two files to separate concerns cleanly: one for your local environment, one for your workspace's security policy.

### `void-claw.toml` (Host Configuration)

Lives on your machine. Defines your environment:

* Container profiles (which agent images to use)
* Registered workspaces and their paths
* Global network and execution defaults

### `void-rules.toml` (Workspace Security Policy)

Lives in your repository. Defines what an agent is allowed to do:

* **`[hostdo]`**: Which host commands the agent may request. Commands can be set to `auto` (always run), `deny` (always block), or `prompt` (ask you each time). Aliases let you map simple agent-facing commands to complex host-side ones.
* **`[network]`**: Coder-style allowlist rules for outbound traffic (`method=... domain=... path=...`). If a request does not match an allowlist rule, it is denied.

## Logging

Void Claw writes daily rotating logs to the directory configured under `[logging].log_dir` in `void-claw.toml`. The default is `~/.local/share/void-claw`, which also holds runtime state and the local CA material used by the proxy.

On first startup, Void Claw also generates a stable `instance_id` and writes it back into `[logging]`. That value is exported as `service.instance.id` so a collector can distinguish logs and traces from different installations.

If you want OpenTelemetry export, enable `[logging.otlp]` in `void-claw.toml` or your own config:

* **Endpoint**: OTLP collector URL, such as `http://localhost:4317` for gRPC or `http://localhost:4318/v1/traces` for HTTP/protobuf.
* **Protocol**: `grpc` or `http`.
* **Level**: `approvals` for prompt-related spans, `all` for the full hostdo/proxy flow, or `none` to disable export.

Example:

```toml
[logging]
log_dir = "~/.local/share/void-claw"

[logging.otlp]
endpoint = "http://localhost:4317"
protocol = "grpc"
level = "approvals"
```

## Workspace Mounting

Void Claw runs workspaces in direct mode: each container mounts the canonical repository directory directly. There is no workspace mirroring or sync workflow.

## Network & Proxy Control

Void Claw's built-in MITM proxy intercepts all outbound HTTP and HTTPS traffic from the agent container, giving you complete visibility and enforcement over external communication.

### How It Works

1. **Intercept**: All outbound requests from the container are routed through the Void Claw proxy.
2. **Evaluate**: The request is checked against your global config and the workspace's `void-rules.toml`.
3. **Enforce**:
   * **Allow**: Request matches a `[network].allowlist` expression.
   * **Deny**: No allowlist match (deny by default).

### Proxy Configuration

Under `[defaults.proxy]` in `void-claw.toml`:

* **`strict_network`**: Enables `NET_ADMIN` capabilities to enforce iptables rules inside the container, ensuring no traffic can bypass the proxy.
* **`proxy_port`**: The local port the proxy listens on (default: `8081`).

Example network policy in `void-rules.toml`:

```toml
[network]
allowlist = [
  "domain=*.npmjs.org",
  "method=GET domain=api.github.com path=/repos/*",
]
```

## Agent Commands

Because agents run in an isolated container with no direct access to your machine, Void Claw provides two bridge commands for controlled interaction with the host.

### `hostdo` (Host Execution Bridge)

Lets an agent request execution of specific commands on your host machine, without raw shell access.

* **Usage inside container:** `hostdo <command> [args...]` (e.g. `hostdo cargo test`)
* **How it works:** The request is routed to the Void Claw manager. Based on your `void-rules.toml` policy, it is automatically executed, silently denied, or escalated to you in the TUI.
* **Aliases:** Map simple agent-facing commands to complex host-side ones (e.g. `hostdo tests` to `cargo test --all`).

### `killme` (Container Exit)

Lets an agent cleanly terminate its own container.

* **Usage inside container:** `killme`
* **How it works:** Sends a clean shutdown request to the Void Claw manager.

## License
MIT
