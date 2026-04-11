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
    *   Add a `[spawning]` section to `void-claw-rules.toml` to permit/deny this action.
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
*Last Updated: 2026-04-11*
