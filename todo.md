# Void Claw Roadmap & TODO

## New Features

### [ ] Flood Protection & Bulk Deny
Harden the manager/TUI against containers flooding `hostdo` or network approval requests.

**Goals:**
* Add configurable flood thresholds + actions (e.g. deny new, cooldown deny, pause container, kill container, notify only).
* Add "mass deny" actions to clear queued `hostdo` and network requests (optionally scoped to a specific container/session).
* Enforce caps before enqueueing into TUI pending lists to prevent unbounded growth.

**Proposed Config:**
* Add a `[flood]` (or `[security.flood]`) section to `void-claw.toml`.
* Separate knobs for `hostdo` vs `network` (different legitimate volumes).
* Threshold ideas:
  * `max_pending_exec`, `max_pending_net` (TUI-visible backlog limits)
  * `max_inflight_exec`, `max_inflight_net` (limit concurrent requests waiting on approval)
  * `max_per_sec_exec`, `max_per_sec_net` (rate limit prompt-generating operations; token bucket)
  * `cooldown_secs` (deny-new duration when tripped)
* Action ideas (configurable): `deny_new`, `cooldown_deny`, `pause_container`, `kill_container`, `notify_only`.

**UX / Safety Notes:**
* Prefer "mass deny" over "mass allow" to avoid thundering-herd releases (many network connections / host commands at once).
* Show a flood indicator/banner in the TUI: counts + rate + current action being applied (e.g. deny_new for N seconds).
* Consider per-session host-side exec concurrency limits so even approved/auto flows can’t spawn unlimited processes.

### [ ] Agent-to-Agent Spawning (Orchestration)
Enable an agent running inside a Void Claw container to programmatically spawn additional agents to perform sub-tasks.

**Core Concept:**
A new container-side utility (e.g., `spawn-agent`) that communicates with the Void Claw manager to launch new containers.

**Technical Requirements:**
*   **Bridge Implementation**: Create a `spawn-agent` script (similar to `hostdo`) that sends a launch request to the manager's server.
*   **Security & Permissions**:
    *   Add a `[spawning]` section to `void-rules.toml` to permit/deny this action.
    *   Implement "fork bomb" protection (max nesting depth, max concurrent sub-agents).
*   **Workspace Management**:
    *   Support launching into the `current` workspace (requires handling file concurrency).
    *   Support launching into a `different` project workspace (requires cross-project permission).
*   **TUI Updates**:
    *   Visualize child agents in the sidebar (e.g., a nested tree view).
    *   Ensure the developer can still approve/deny network and `hostdo` requests from sub-agents.
*   **Execution API**: Define a CLI-like interface for the agent:
    `spawn-agent --project <name> --agent <type> --cmd "the task description"`

---


1. Running tests/builds from other Docker containers

Currently, hostdo acts as a bridge to run commands directly on the host machine. To run them in a container instead, we could expand the /exec protocol:

* How it would work: You could use a command like hostdo --image node:20 npm test.
* Manager Side: The manager would see the --image flag and, instead of running the command on the host, it would execute docker run --rm -v <workspace>:/workspace ... node:20 npm test.
* Benefits: This keeps the build/test environment isolated from the host and allows you to use different versions of tools (e.g., Node.js 18 vs 20) without changing the host or the main agent container.
* Implementation: Since these are typically short-lived and non-interactive, they wouldn't need a TUI tab; the manager would just stream the output back to the calling hostdo process.

2. Spawning new agent containers with a prompt

This is essentially "agent orchestration." An agent in one container can "fork" another agent to help with a sub-task.

* How it would work: A command like hostdo spawn-agent --profile claude --prompt "Analyze the logs in /workspace/logs and summarize the errors.".
* Manager Side:
    1. The manager receives the spawn request.
    2. It sends a message to the TUI to open a new session using the specified profile.
    3. The TUI calls its existing spawn logic to create the new container and terminal tab.
    4. Once the container is up, the TUI automatically injects the initial prompt followed by a newline into the new session's PTY.
* Killme expansion: You mentioned killme. Perhaps a killme --next "Task finished, now do X" could be used to terminate the current container and immediately trigger a new one with a follow-up task, though simply having spawn-agent available inside the
    container covers most "handoff" scenarios.

Architectural Considerations for Discussion:

1. Approval Flow: Should spawning a new agent require manual approval in the TUI (like hostdo commands do)? This would be consistent with the "Human-in-the-loop" philosophy of the project.
2. Initial Prompt Injection: How do we know when the agent inside the new container is ready to receive the prompt? Simply writing to the PTY immediately after spawn usually works, but it can be racey if the shell/agent is still booting.
3. Communication Bridge: Would it be better to have a dedicated spawn-agent script in docker/scripts/ alongside hostdo and killme, or should we consolidate these into a single "void-claw-bridge" tool?

What are your thoughts on these approaches? Does this align with how you were imagining using these features?


*Last Updated: 2026-04-12*
