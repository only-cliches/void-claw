<div align="center">

# 🕵️ Void Claw 🛡️

**Zero-trust container manager for AI coding agents**

</div>

![Void Claw Demo showing launching an agent and the approval dialog.](https://github.com/only-cliches/void-claw/blob/main/demo.gif?raw=true)

AI coding agents are powerful, and by default, completely unconstrained. Give one your terminal and it has your machine: your files, your credentials, and your network. Void Claw enforces a zero-trust boundary around every agent session, running agents in isolated Docker containers with policy-enforced access to your code, your host, and the outside world. Nothing gets through without a rule that allows it.

## Key Features

* **Isolated Docker Environments**: Agents run in locked-down Docker containers, fully separated from your host system.
* **Zero-Trust Network Proxy**: A built-in MITM proxy intercepts all outbound HTTP and HTTPS traffic. Every request is evaluated against your policy: auto-allowed, denied, or escalated to you for approval in real time.
* **Controlled Host Execution (`hostdo`)**: Agents have no direct access to your machine. Instead, they request specific pre-approved host commands via `hostdo` (e.g. `cargo test`, `npm run build`). You approve or deny each class of command, once or permanently.
* **Interactive Terminal UI (TUI)**: Manage everything from a single terminal interface. View active containers, inspect logs, review and action pending network and host requests, and drop into a live terminal session when needed.
* **Flexible Workspace Syncing**: Void Claw creates a managed mirror of your project inside the container. Choose your sync strategy: push changes back automatically, pull host changes into the workspace, sync bidirectionally, or keep the workspace completely isolated.
* **Ready-to-Use Agent Profiles**: First-class support for Claude Code, OpenAI Codex, Google Gemini CLI, and Opencode, including automatic auth state mounting so agents don't need to re-authenticate on every launch.
* **OpenTelemetry Logging**: Export hostdo, proxy, and startup traces to your collector with configurable OTLP settings, while keeping local rotating logs on disk.

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

### 2. Add a Project

Add a project from within the TUI, or by adding a `[[projects]]` block to your `void-claw.toml`.

When a project is registered, Void Claw writes a `void-rules.toml` to the root of your project repository. This file defines the security policy for any agent operating in that codebase: which host commands it may request and which network destinations it may reach. Commit it to version control so your policy travels with the code.

### 3. Launch an Agent

From the TUI, use the arrow keys (or `j`/`k`) to navigate to a project and press `Enter` to launch an agent container.

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

Void Claw uses two files to separate concerns cleanly: one for your local environment, one for your project's security policy.

### `void-claw.toml` (Host Configuration)

Lives on your machine. Defines your environment:

* Workspace storage location
* Container profiles (which agent images to use)
* Registered projects and their paths
* Global network and execution defaults

### `void-rules.toml` (Project Security Policy)

Lives in your project repository. Defines what an agent is allowed to do:

* **`[hostdo]`**: Which host commands the agent may request. Commands can be set to `auto` (always run), `deny` (always block), or `prompt` (ask you each time). Aliases let you map simple agent-facing commands to complex host-side ones.
* **`[network]`**: Which domains the agent may reach. Common developer infrastructure (GitHub, npm, PyPI, crates.io) is pre-approved. Everything else defaults to `prompt` and you decide at runtime, with the option to persist your decision back to the policy file automatically.

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

## File Synchronization Modes

Void Claw creates a managed workspace mirror of your project inside the container, isolating the agent from your canonical source files. Configure the sync strategy per-project or globally under `[defaults.sync]`.

### Modes

* **`pushback` (Default)**: Agent changes are automatically synced back to your canonical project directory.
* **`pullthrough`**: Your host changes are synced into the agent's workspace. The agent's changes stay contained.
* **`bidirectional`**: Changes flow in both directions.
* **`workspace_only`**: The workspace is seeded once at container start with no further sync. Maximum isolation for experimentation.
* **`direct`**: Your canonical directory is bind-mounted directly into the container. Immediate, no mirroring. *Note: bypasses some safety features.*

### Conflict Resolution

When syncing back to your canonical directory, `conflict_policy` controls what happens when both sides have changed:

* **`preserve_canonical` (Default)**: Your host edits are never overwritten. Safest option.
* **`preserve_workspace`**: The agent's version always wins.

### Excluding Files

Void Claw is careful about what it exposes to agents:

* **`.gitignore` support**: Files ignored by git are not seeded into the workspace.
* **Global excludes**: Sensitive files (`.env`, `*.pem`, `.ssh`, `.aws`, `.claude`) are excluded by default and never synced.
* **Project excludes**: Add `exclude_patterns` in `void-claw.toml` or `void-rules.toml` to skip large build artifacts like `node_modules` or `target/`.

## Network & Proxy Control

Void Claw's built-in MITM proxy intercepts all outbound HTTP and HTTPS traffic from the agent container, giving you complete visibility and enforcement over external communication.

### How It Works

1. **Intercept**: All outbound requests from the container are routed through the Void Claw proxy.
2. **Evaluate**: The request is checked against your global config and the project's `void-rules.toml`.
3. **Enforce**:
   * **Auto-Allow**: Matches an `auto` rule, so the request proceeds immediately.
   * **Deny**: Matches a `deny` rule, so the request is blocked.
   * **Prompt**: No matching rule, so the TUI alerts you and asks: `Allow Once`, `Always Allow`, `Deny`, or `Always Deny`. Permanent decisions are written back to your policy file.

### Proxy Configuration

Under `[defaults.proxy]` in `void-claw.toml`:

* **`strict_network`**: Enables `NET_ADMIN` capabilities to enforce iptables rules inside the container, ensuring no traffic can bypass the proxy.
* **`proxy_port`**: The local port the proxy listens on (default: `8081`).

Example network policy in `void-rules.toml`:

```toml
[network]
default_policy = "prompt"

[[network.rules]]
host = "*.npmjs.org"
policy = "auto"

[[network.rules]]
host = "malicious-site.com"
policy = "deny"
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
