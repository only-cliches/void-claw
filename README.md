<div align="center">

# 🤖 Void Claw 🛡️

</div>

![Void Claw Demo](https://github.com/only-cliches/void-claw/blob/main/screenshot.png?raw=true)

This project is an agent workspace manager designed to safely expose filtered project workspaces to AI coding agents. 

AI coding assistants are incredibly powerful, but giving them unfettered access to your local machine, terminal, and network can be risky. Void Claw solves this by running your AI agents in isolated Docker containers while providing a secure bridge for them to interact with your code and the outside world—putting *you* in complete control.

## Key Features

* **Isolated Docker Environments**: Agents run inside secure Docker containers. 
* **Interactive Terminal UI (TUI)**: Manage everything from a terminal interface built with Alacritty and Ratatui. You can view active containers, inspect logs, and seamlessly drop into a terminal session.
* **Granular Network Filtering**: A built-in MITM (Man-in-the-Middle) proxy intercepts HTTP and HTTPS traffic from the container. You can auto-allow, permanently deny, or get prompted in the TUI to approve outbound network requests.
* **Safe Host Execution (`hostdo`)**: Agents cannot run arbitrary commands on your machine. Instead, they use a special `hostdo` command to request execution of specific, pre-approved commands (like `cargo test` or `npm run build`) on your host machine. You can approve or deny these requests directly in the TUI.
* **Flexible Workspace Syncing**: Void Claw can optionally synchronizes your actual project code (the "canonical" path) with the container's workspace. It supports multiple sync modes, including pushback, pullthrough, bidirectional, and direct mounting.
* **Ready-to-Use Agent Profiles**: Built-in support and Dockerfiles for popular agents including Claude Code, OpenAI Codex, Google Gemini CLI, and Opencode.

## Getting Started

### Prerequisites

Void Claw requires **Docker** to be installed and available in your system's `PATH`.

### Install

```bash
git clone https://github.com/only-cliches/void-claw
cd void-claw
cargo install --path .
```

### 1. Initialization

Generate your starter configuration file by just running `void-claw` from any directory.

```bash
void-claw
```
This will prompt you to create a `void-claw.toml` file in your current directory or in your home directory, populated with sensible defaults and a `docker_dir` that points at your current working directory's `docker/` folder when present.
If `./docker` does not exist, void-claw will instead use `~/.config/void-claw/docker` and create it on first run.
If that folder is missing files, the first run can offer to fetch the built-in Dockerfiles from GitHub and write the helper scripts needed for image builds.

### 2. Add a Project

You can easily add a new project from within the TUI, or by adding a `[[projects]]` block to your `void-claw.toml` file.

When a project is set up, Void Claw will place a `void-claw-rules.toml` file in the root of your project repository. This file is meant to be committed to version control and tells the agent exactly what commands it is allowed to run and what network domains it can reach.

### 3. Launch an agent!

Once in the TUI, use the arrow keys (or `j`/`k`) to navigate the sidebar, select a project, and press `Enter` to launch an AI agent container.

## Supported Agents

Void Claw is designed to be highly flexible. It comes with first-class support for several popular AI coding assistants, but it isn't limited to just those!

### Out-of-the-Box Support
Void Claw includes built-in configurations, Dockerfiles, and state-management for the following agents:

* **Claude Code** (`@anthropic-ai/claude-code`)
* **OpenAI Codex** (`@openai/codex`)
* **Google Gemini CLI** (`@google/gemini-cli`)
* **Opencode** (`opencode-ai`)

For these built-in agents, Void Claw automatically handles the heavy lifting. It automatically bind-mounts the agent's authentication and session state (e.g., `~/.claude`, `~/.gemini`, `~/.codex`) so you don't have to sign in every time you launch a new container.

### Bring Your Own Agent (BYOA)
Not seeing your favorite agent? No problem! **Any AI agent that can run inside a Docker container can be used with Void Claw.** You can define custom containers in your `void-claw.toml` file. By setting the agent type to `none`, Void Claw will skip the built-in config file injection, allowing you to mount your own custom Docker images, environment variables, and configuration files. As long as your agent can be packaged in Docker, Void Claw can secure it!

## How Configuration Works

Void Claw relies on two main configuration files to keep things organized and secure:

### `void-claw.toml` (The Global Config)
This file lives on your host machine and manages your overall environment. 
* Defines where your workspaces are stored.
* Lists your container profiles (which agent images to use).
* Registers your local projects and their paths.
* Configures global network and execution defaults.

### `void-claw-rules.toml` (The Project Policy)
This file lives in your project's repository. It dictates the specific rules of engagement for any AI operating in that codebase.
* **`[hostdo]`**: Defines what host-side commands the agent is allowed to execute. For example, you can allow `hostdo npm test` automatically, but require a manual prompt for `hostdo npm install`.
* **`[network]`**: Defines which domains the agent can talk to. By default, safe developer APIs (like GitHub or npm) are added to the default allow list, while unknown domains will trigger a prompt in your TUI.

## File Synchronization Modes

Void Claw supports several modes for synchronizing your "canonical" project code (the source of truth on your host) with the agent's workspace. Most of these modes involve creating a **managed mirror** of your project in a separate workspace directory. This isolation ensures that an agent cannot accidentally corrupt your original files without your knowledge or according to the rules you define.

You can configure the sync mode in your `void-claw.toml` under `[projects.sync]` or as a global default in `[defaults.sync]`.

### Supported Modes

* **`pushback` (Default)**: Changes made by the agent in the workspace are automatically synchronized back to your canonical project directory. This is the most common mode for active development.
* **`pullthrough`**: Changes you make in your canonical project directory are automatically synchronized into the agent's workspace. The agent's own changes are *not* pushed back.
* **`bidirectional`**: Changes are synchronized in both directions. If you edit a file on your host, the agent sees it; if the agent edits a file, your host version is updated.
* **`workspace_only`**: The workspace is "seeded" once from your canonical directory when the container starts, but no further synchronization occurs. This provides the highest level of isolation for experimentation.
* **`direct`**: The canonical project directory is bind-mounted **directly** into the container. No mirror is created, and all changes are immediate. *Note: This mode disables some safety features as it bypasses the managed workspace.*

### Conflict Resolution

When using sync modes that push changes back to the canonical directory (like `pushback` or `bidirectional`), Void Claw provides a `conflict_policy` setting to determine what happens if a file has been modified in both the host (canonical) and the workspace since the last sync.

* **`preserve_canonical` (Default)**: If the host version of a file is newer than the workspace version, the sync will **not** overwrite the host version. This is the safest setting, ensuring your manual edits on the host are never lost.
* **`preserve_workspace`**: The workspace version will always overwrite the host version, regardless of modification timestamps.

These can be configured in your `void-claw.toml` under `[defaults.sync]` or per-project.

### Excluding Files

Void Claw is designed to be efficient and secure by only syncing the files necessary for the agent to work.

*   **`.gitignore` Support**: Void Claw automatically detects and respects `.gitignore` files within your canonical project directory. Files ignored by git will not be seeded into the agent's workspace.
*   **Global Excludes**: The `void-claw.toml` file includes a `global_exclude_patterns` list (under `[defaults.sync]`) for files that should *never* be synced across any project (e.g., `.env`, `*.pem`, `.ssh`, `.aws`).
*   **Project-Specific Excludes**: You can add `exclude_patterns` to a specific project in `void-claw.toml` or directly in the project's `void-claw-rules.toml`. This is useful for ignoring large build artifacts or dependencies (like `node_modules` or `target/`) that the agent doesn't need to see.

## Network & Proxy Control

Void Claw includes a built-in MITM (Man-in-the-Middle) proxy that intercepts all outbound HTTP and HTTPS traffic from the agent's container. This gives you absolute visibility and control over the agent's external communication.

### How it Works

1.  **Intercept**: When an agent makes a network request, it is routed through the Void Claw proxy.
2.  **Evaluate**: The proxy checks the request against the rules defined in your global config and the project-specific `void-claw-rules.toml`.
3.  **Action**:
    *   **Auto-Allow**: If the domain/path matches an "auto" rule, the request proceeds immediately.
    *   **Deny**: If it matches a "deny" rule, the request is blocked.
    *   **Prompt (Default)**: If no rule matches, the TUI will flash and prompt you to `Allow Once`, `Always Allow` (persists to `void-claw-rules.toml`), `Deny`, or `Always Deny` (persist to `void-claw-rules.toml`).

### Configuration

You can customize the proxy behavior in `void-claw.toml` under `[defaults.proxy]`:

*   **`strict_network`**: When enabled, Void Claw uses `NET_ADMIN` capabilities to enforce strict iptables rules inside the container, ensuring *no* traffic can bypass the proxy or execution bridge.
*   **`proxy_port`**: Customize the local port the proxy listens on (default: `8081`).

Specific domain rules are managed in `void-claw-rules.toml`:

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

## Built-in Container Commands

Because Void Claw locks the AI agent inside an isolated container, it provides two special bridge scripts so the agent can safely interact with your host machine and the manager itself.

### `hostdo` (Host Execution Bridge)
The `hostdo` command routes execution requests from the isolated container back to the Void Claw host (your workstation). This allows the agent to trigger host-side actions (like compiling code or running tests) without giving raw SSH access to your machine.

* **Usage inside the container:** `hostdo <command> [args...]` (e.g., `hostdo cargo test`)
* **How it works:** When the agent runs `hostdo`, the request is sent to the void-claw manager. Based on your `void-claw-rules.toml` policy, the manager will either automatically run the command, silently deny it, or prompt you in the TUI to approve or deny the request. 
* **Aliases:** You can set up shortcuts in your config so the agent can run simple commands that expand into complex ones on the host (e.g., mapping `hostdo tests` to `cargo test --all`).

### `killme` (Container Exit Command)
The `killme` command is a simple utility that allows the agent to politely terminate its own session. 

* **Usage inside the container:** `killme`
* **How it works:** It sends a request to the Void Claw manager to cleanly stop the current container session.


## License
MIT
