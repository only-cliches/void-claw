use super::{App, Focus, SidebarItem, restore_terminal_output};
use crate::activity::{Activity, ActivityEvent, ActivityKind, ActivityState};
use crate::ca::CaStore;
use crate::config::Config;
use crate::proxy::ProxyState;
use crate::shared_config::SharedConfig;
use crate::state::StateManager;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tokio::sync::mpsc;

#[test]
fn restore_terminal_output_emits_reset_sequences() {
    let mut buf = Vec::new();
    restore_terminal_output(&mut buf).expect("restore commands should serialize");
    let out = String::from_utf8_lossy(&buf);
    assert!(out.contains("\u{1b}[?1049l"), "missing leave alt-screen");
    assert!(out.contains("\u{1b}[?25h"), "missing show cursor");
    assert!(out.contains("\u{1b}[?1000l"), "missing disable mouse");
    assert!(
        out.contains("\u{1b}[?2004l"),
        "missing disable bracketed paste"
    );
    assert!(out.contains("\u{1b}[?7h"), "missing enable line wrap");
    assert!(out.contains("\u{1b}[0m"), "missing reset color");
}

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("harness-hat-{prefix}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn encode_sgr_mouse_click_down_left() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::empty(),
    };
    let bytes = super::app::encode_sgr_mouse(mouse).expect("encodes");
    assert_eq!(String::from_utf8_lossy(&bytes), "\u{1b}[<0;1;1M");
}

#[test]
fn encode_sgr_mouse_click_up_left() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 2,
        row: 3,
        modifiers: KeyModifiers::empty(),
    };
    let bytes = super::app::encode_sgr_mouse(mouse).expect("encodes");
    assert_eq!(String::from_utf8_lossy(&bytes), "\u{1b}[<0;3;4m");
}

#[test]
fn encode_sgr_mouse_drag_left() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 9,
        row: 8,
        modifiers: KeyModifiers::empty(),
    };
    let bytes = super::app::encode_sgr_mouse(mouse).expect("encodes");
    assert_eq!(String::from_utf8_lossy(&bytes), "\u{1b}[<32;10;9M");
}

#[test]
fn encode_sgr_mouse_scroll_down_with_shift() {
    let mouse = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 4,
        row: 5,
        modifiers: KeyModifiers::SHIFT,
    };
    let bytes = super::app::encode_sgr_mouse(mouse).expect("encodes");
    assert_eq!(String::from_utf8_lossy(&bytes), "\u{1b}[<69;5;6M");
}

#[test]
fn encode_sgr_mouse_ignores_move() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Moved,
        column: 1,
        row: 1,
        modifiers: KeyModifiers::empty(),
    };
    assert!(super::app::encode_sgr_mouse(mouse).is_none());
}

fn build_test_app() -> App {
    let root = unique_temp_dir("tui-build-flow");
    let global_rules_file = root.join("global-rules.toml");
    let workspace_root = root.join("workspace");
    let docker_dir = root.join("docker-root");
    let project_path = root.join("project-a");
    std::fs::create_dir_all(&workspace_root).expect("create workspace");
    std::fs::create_dir_all(&docker_dir).expect("create docker dir");
    std::fs::create_dir_all(&project_path).expect("create project path");

    let cfg_path = root.join("harness-hat.toml");
    let raw = format!(
        r#"
docker_dir = "{}"

[workspace]

[manager]
global_rules_file = "{}"

[[workspaces]]
name = "project-a"
canonical_path = "{}"

[container_profiles.test]
image = "missing-image"
"#,
        docker_dir.display(),
        global_rules_file.display(),
        project_path.display()
    );
    std::fs::write(&cfg_path, raw).expect("write test config");
    let config: Config = crate::config::load(&cfg_path).expect("load minimal config");
    let shared = SharedConfig::new(Arc::new(config));

    let (_exec_tx, exec_rx) = mpsc::channel(8);
    let (_stop_tx, stop_rx) = mpsc::channel(8);
    let (net_tx, net_rx) = mpsc::channel(8);
    let (activity_tx, activity_rx) = mpsc::unbounded_channel();
    let (_audit_tx, audit_rx) = mpsc::channel(8);

    let ca = Arc::new(CaStore::load_or_create(&root.join("ca")).expect("create CA"));
    let proxy_state =
        ProxyState::new(ca, shared.clone(), net_tx, activity_tx).expect("proxy state");
    let state = StateManager::open(&root.join("state")).expect("state manager");

    App::new(
        shared,
        root.join("config.toml"),
        "token".to_string(),
        crate::server::SessionRegistry::default(),
        exec_rx,
        stop_rx,
        net_rx,
        activity_rx,
        audit_rx,
        state,
        proxy_state,
        "127.0.0.1:0".to_string(),
        root.join("ca/ca.crt").display().to_string(),
    )
    .expect("App::new")
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn build_commands_use_configured_docker_root() {
    let docker_dir = std::path::Path::new("/tmp/harness-hat-docker-root");
    let (build_cmd, base_cmd) = App::build_commands_for(docker_dir, "harness-hat-codex:local");

    assert_eq!(
        build_cmd,
        vec![
            "build".to_string(),
            "-t".to_string(),
            "harness-hat-codex:local".to_string(),
            "-f".to_string(),
            "/tmp/harness-hat-docker-root/codex.dockerfile".to_string(),
            "/tmp/harness-hat-docker-root".to_string(),
        ]
    );
    assert_eq!(
        base_cmd,
        Some(vec![
            "build".to_string(),
            "-t".to_string(),
            "harness-hat-base:local".to_string(),
            "-f".to_string(),
            "/tmp/harness-hat-docker-root/harness-hat-base.dockerfile".to_string(),
            "/tmp/harness-hat-docker-root".to_string(),
        ])
    );
}

#[test]
fn build_commands_for_base_image_do_not_nest_base_build() {
    let docker_dir = std::path::Path::new("/tmp/harness-hat-docker-root");
    let (build_cmd, base_cmd) = App::build_commands_for(docker_dir, "harness-hat-base:local");

    assert_eq!(
        build_cmd,
        vec![
            "build".to_string(),
            "-t".to_string(),
            "harness-hat-base:local".to_string(),
            "-f".to_string(),
            "/tmp/harness-hat-docker-root/harness-hat-base.dockerfile".to_string(),
            "/tmp/harness-hat-docker-root".to_string(),
        ]
    );
    assert_eq!(base_cmd, None);
}

#[test]
fn preflight_missing_image_opens_image_build_pane() {
    let mut app = build_test_app();
    let proceed = app.preflight_image_or_prompt_build(0, 0, "missing-image:latest", |_| Ok(false));
    assert!(!proceed);
    assert_eq!(app.focus, Focus::ImageBuild);
    assert_eq!(app.build_project_idx, Some(0));
    assert_eq!(app.build_container_idx, Some(0));
    assert_eq!(app.build_cursor, 0);
}

#[test]
fn persist_exec_rule_updates_existing_timeout() {
    let mut app = build_test_app();
    let rules_path = app.config.get().workspaces[0]
        .canonical_path
        .join("harness-rules.toml");
    std::fs::write(
        &rules_path,
        r#"
[hostdo]
default_policy = "prompt"

[[hostdo.commands]]
argv = ["cargo", "test"]
cwd = "$WORKSPACE"
timeout_secs = 60
concurrency = "queue"
approval_mode = "auto"
"#,
    )
    .expect("write rules");

    app.persist_exec_rule(
        &rules_path,
        &["cargo".to_string(), "test".to_string()],
        None,
        300,
        "$WORKSPACE",
        crate::rules::ApprovalMode::Auto,
    )
    .expect("persist rule");

    let rules = crate::rules::load(&rules_path).expect("load rules");
    assert_eq!(rules.hostdo.commands.len(), 1);
    assert_eq!(rules.hostdo.commands[0].timeout_secs, 300);
}

#[test]
fn persist_network_rule_writes_denylist_entry() {
    let mut app = build_test_app();
    let rules_path = app.config.get().workspaces[0]
        .canonical_path
        .join("harness-rules.toml");
    std::fs::write(
        &rules_path,
        r#"
[network]
allowlist = ["domain=blocked.example.com"]
"#,
    )
    .expect("write rules");

    let updated_path = app
        .persist_network_rule(
            "blocked.example.com",
            crate::rules::NetworkPolicy::Deny,
            Some("project-a"),
        )
        .expect("persist network deny");

    assert_eq!(updated_path.as_deref(), Some(rules_path.as_path()));
    let rules = crate::rules::load(&rules_path).expect("load rules");
    assert!(
        rules
            .network
            .denylist
            .iter()
            .any(|r| r == "domain=blocked.example.com")
    );
    assert!(
        !rules
            .network
            .allowlist
            .iter()
            .any(|r| r == "domain=blocked.example.com")
    );

    let second_update = app
        .persist_network_rule(
            "blocked.example.com",
            crate::rules::NetworkPolicy::Deny,
            Some("project-a"),
        )
        .expect("persist duplicate network deny");
    assert!(second_update.is_none());
}

#[test]
fn activity_events_update_history_and_state() {
    let mut app = build_test_app();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let activity = Activity::new(
        "project-a".to_string(),
        Some("container-a".to_string()),
        ActivityKind::Hostdo {
            argv: vec!["cargo".into(), "test".into()],
            image: None,
            timeout_secs: 60,
        },
        ActivityState::PendingApproval,
        cancel_flag,
    );
    let id = activity.id.clone();

    app.apply_activity_event(ActivityEvent::Started(activity));
    app.apply_activity_event(ActivityEvent::Line {
        id: id.clone(),
        line: "stdout: compiling".to_string(),
    });
    app.apply_activity_event(ActivityEvent::Finished {
        id: id.clone(),
        state: ActivityState::Complete,
        status: Some("exit code 0".to_string()),
    });

    let activity = app.activity_by_id(&id).expect("activity exists");
    assert_eq!(activity.state, ActivityState::Complete);
    assert_eq!(activity.status.as_deref(), Some("exit code 0"));
    assert!(activity.finished_at.is_some());
    assert_eq!(
        activity.lines.back().map(String::as_str),
        Some("stdout: compiling")
    );
}

#[test]
fn hostdo_command_timer_starts_on_running_state() {
    let mut app = build_test_app();
    let activity = Activity::new(
        "project-a".to_string(),
        Some("container-a".to_string()),
        ActivityKind::Hostdo {
            argv: vec!["cargo".into(), "test".into()],
            image: Some("rust".into()),
            timeout_secs: 120,
        },
        ActivityState::Running,
        Arc::new(AtomicBool::new(false)),
    );
    let id = activity.id.clone();

    app.apply_activity_event(ActivityEvent::Started(activity));
    assert!(
        app.activity_by_id(&id)
            .expect("activity exists")
            .command_started_at
            .is_none()
    );

    app.apply_activity_event(ActivityEvent::State {
        id: id.clone(),
        state: ActivityState::PullingImage,
        status: Some("pulling Docker image 'rust'".to_string()),
    });
    assert!(
        app.activity_by_id(&id)
            .expect("activity exists")
            .command_elapsed_duration()
            .is_none()
    );

    app.apply_activity_event(ActivityEvent::State {
        id: id.clone(),
        state: ActivityState::Running,
        status: Some("running cargo test".to_string()),
    });
    assert!(
        app.activity_by_id(&id)
            .expect("activity exists")
            .command_started_at
            .is_some()
    );

    app.apply_activity_event(ActivityEvent::Finished {
        id: id.clone(),
        state: ActivityState::Complete,
        status: Some("exit code 0".to_string()),
    });
    let activity = app.activity_by_id(&id).expect("activity exists");
    assert!(activity.command_finished_at.is_some());
    assert!(activity.command_elapsed_duration().is_some());
}

#[test]
fn activity_cancel_marks_flag_and_pending_hostdo() {
    let mut app = build_test_app();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let activity = Activity::new(
        "project-a".to_string(),
        Some("container-a".to_string()),
        ActivityKind::Hostdo {
            argv: vec!["cargo".into(), "test".into()],
            image: None,
            timeout_secs: 60,
        },
        ActivityState::PendingApproval,
        cancel_flag.clone(),
    );
    let id = activity.id.clone();
    let (tx, _rx) = tokio::sync::oneshot::channel();

    app.apply_activity_event(ActivityEvent::Started(activity));
    app.pending_exec.push(crate::server::PendingItem {
        id: "pending-1".to_string(),
        activity_id: id.clone(),
        cancel_flag: cancel_flag.clone(),
        project: "project-a".to_string(),
        container_id: Some("container-a".to_string()),
        argv: vec!["cargo".into(), "test".into()],
        image: None,
        timeout_secs: 60,
        cwd: std::path::PathBuf::from("/workspace"),
        rule_cwd: std::path::PathBuf::from("/workspace"),
        matched_command: None,
        response_tx: Some(tx),
    });

    app.cancel_activity(&id);

    assert!(cancel_flag.load(std::sync::atomic::Ordering::SeqCst));
    assert!(app.pending_exec.is_empty());
    let activity = app.activity_by_id(&id).expect("activity exists");
    assert_eq!(activity.state, ActivityState::Cancelled);
}

#[test]
fn activity_container_identity_matching_accepts_id_prefixes_and_names() {
    assert!(App::container_identity_matches(
        "abcdef",
        "abcdef123456",
        "codex",
        "harness-codex"
    ));
    assert!(App::container_identity_matches(
        "codex",
        "abcdef123456",
        "codex",
        "harness-codex"
    ));
    assert!(!App::container_identity_matches(
        "other",
        "abcdef123456",
        "codex",
        "harness-codex"
    ));
    assert!(!App::container_identity_matches("anything", "", "", ""));
}

#[test]
fn terminal_activities_wait_to_fade_until_unselected() {
    let mut app = build_test_app();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let mut activity = Activity::new(
        "project-a".to_string(),
        Some("container-a".to_string()),
        ActivityKind::Network {
            method: "GET".to_string(),
            host: "example.com".to_string(),
            path: "/".to_string(),
            protocol: "http".to_string(),
            payload_preview: String::new(),
            payload_truncated: false,
            content_type: None,
            content_length: None,
        },
        ActivityState::Complete,
        cancel_flag,
    );
    activity.updated_at = std::time::Instant::now()
        - Duration::from_secs(crate::activity::ACTIVITY_TERMINAL_TTL_SECS + 1);
    let id = activity.id.clone();

    app.apply_activity_event(ActivityEvent::Started(activity));
    app.active_activity = Some(id.clone());
    app.focus = Focus::Activity;

    app.prune_terminal_activities();

    let activity = app.activity_by_id(&id).expect("selected activity is kept");
    assert!(activity.terminal_unselected_at.is_none());

    app.active_activity = None;
    app.focus = Focus::Sidebar;
    app.prune_terminal_activities();

    let activity = app
        .activity_by_id_mut(&id)
        .expect("activity starts fading after deselection");
    assert!(activity.terminal_unselected_at.is_some());
    activity.terminal_unselected_at = Some(
        std::time::Instant::now()
            - Duration::from_secs(crate::activity::ACTIVITY_TERMINAL_HIGHLIGHT_SECS + 1),
    );

    app.sidebar_idx = 0;
    app.refresh_terminal_activity_selection(&[SidebarItem::Activity(id.clone())]);

    let activity = app
        .activity_by_id(&id)
        .expect("highlighted activity resets fade");
    assert!(activity.terminal_unselected_at.is_none());

    app.refresh_terminal_activity_selection(&[SidebarItem::Launch(0)]);
    let activity = app
        .activity_by_id_mut(&id)
        .expect("activity restarts fading after highlight moves away");
    assert!(activity.terminal_unselected_at.is_some());
    activity.terminal_unselected_at = Some(
        std::time::Instant::now()
            - Duration::from_secs(crate::activity::ACTIVITY_TERMINAL_TTL_SECS + 1),
    );

    app.prune_terminal_activities();

    assert!(app.activity_by_id(&id).is_none());
}

#[test]
fn sidebar_selection_tracks_session_preview() {
    let mut app = build_test_app();
    let items = vec![
        SidebarItem::Workspace(0),
        SidebarItem::Launch(0),
        SidebarItem::Session(2),
    ];

    app.sidebar_idx = 2;
    app.update_sidebar_preview(&items);
    assert_eq!(app.preview_session, Some(2));

    app.sidebar_idx = 1;
    app.update_sidebar_preview(&items);
    assert_eq!(app.preview_session, None);
}

#[test]
fn sidebar_selection_preserves_item_when_activity_rows_shift_indices() {
    let mut app = build_test_app();
    let before = vec![
        SidebarItem::Workspace(0),
        SidebarItem::Session(0),
        SidebarItem::Launch(0),
        SidebarItem::Settings(0),
        SidebarItem::NewWorkspace,
    ];
    app.sidebar_idx = 3;
    let selected = app.selected_sidebar_item_from(&before);

    let after_insert = vec![
        SidebarItem::Workspace(0),
        SidebarItem::Session(0),
        SidebarItem::Activity("activity-1".to_string()),
        SidebarItem::Activity("activity-2".to_string()),
        SidebarItem::Launch(0),
        SidebarItem::Settings(0),
        SidebarItem::NewWorkspace,
    ];
    app.restore_sidebar_selection(selected.as_ref(), &after_insert);
    assert_eq!(app.sidebar_idx, 5);

    let after_remove = vec![
        SidebarItem::Workspace(0),
        SidebarItem::Session(0),
        SidebarItem::Launch(0),
        SidebarItem::Settings(0),
        SidebarItem::NewWorkspace,
    ];
    app.restore_sidebar_selection(selected.as_ref(), &after_remove);
    assert_eq!(app.sidebar_idx, 3);
}

#[test]
fn ctrl_g_toggles_terminal_fullscreen() {
    let mut app = build_test_app();
    app.focus = Focus::Terminal;
    app.active_session = Some(0);

    app.handle_terminal_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL));
    assert!(app.terminal_fullscreen);
    assert!(!app.log_fullscreen);

    app.handle_terminal_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL));
    assert!(!app.terminal_fullscreen);
}

#[test]
fn escape_returns_terminal_fullscreen_to_sidebar() {
    let mut app = build_test_app();
    app.focus = Focus::Terminal;
    app.active_session = Some(0);
    app.terminal_fullscreen = true;

    app.handle_terminal_key(key(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!app.terminal_fullscreen);
    assert_eq!(app.focus, Focus::Sidebar);
    assert!(!app.should_quit);
}

#[test]
fn escape_returns_terminal_to_sidebar_without_quitting() {
    let mut app = build_test_app();
    app.focus = Focus::Terminal;
    app.active_session = Some(0);

    app.handle_terminal_key(key(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!app.should_quit);
    assert_eq!(app.focus, Focus::Sidebar);
}

#[test]
fn image_build_status_bar_only_shows_live_shortcuts() {
    let mut app = build_test_app();
    app.focus = Focus::ImageBuild;

    let prompt_keys = super::render::status_bar_keys(&app);
    assert!(!prompt_keys.contains("[r]"));
    assert!(!prompt_keys.contains("[c]"));
    assert!(prompt_keys.contains("[↵/l]select"));
    assert!(prompt_keys.contains("[Esc/^B]back"));

    app.build_task = Some(super::BuildTaskState {
        label: "docker build".to_string(),
        shell_command: "docker build .".to_string(),
        cancel_flag: Arc::new(AtomicBool::new(false)),
    });

    let running_keys = super::render::status_bar_keys(&app);
    assert!(!running_keys.contains("[r]"));
    assert!(!running_keys.contains("[c]"));
    assert!(running_keys.contains("[^C]cancel build"));
    assert!(running_keys.contains("[Esc/^B]sidebar"));
}

#[test]
fn right_pane_gap_only_shows_for_docker_build_views() {
    let mut app = build_test_app();
    assert_eq!(super::render::right_pane_gap_width(&app), 0);

    app.focus = Focus::ImageBuild;
    assert_eq!(super::render::right_pane_gap_width(&app), 1);

    app.focus = Focus::Sidebar;
    app.build_project_idx = Some(0);
    app.build_container_idx = Some(0);
    app.build_task = Some(super::BuildTaskState {
        label: "docker build".to_string(),
        shell_command: "docker build .".to_string(),
        cancel_flag: Arc::new(AtomicBool::new(false)),
    });
    let items = app.sidebar_items();
    app.sidebar_idx = items
        .iter()
        .position(|item| matches!(item, SidebarItem::Build(_)))
        .expect("build row present");

    assert_eq!(super::render::right_pane_gap_width(&app), 1);
}

#[test]
fn removing_active_session_clears_terminal_fullscreen() {
    let mut app = build_test_app();
    app.active_session = Some(0);
    app.terminal_fullscreen = true;
    app.last_terminal_esc = Some(std::time::Instant::now());

    app.clear_terminal_fullscreen_for_removed_session(0);

    assert!(!app.terminal_fullscreen);
    assert!(app.last_terminal_esc.is_none());
}

#[cfg(unix)]
#[test]
fn termios_guard_only_restores_ixon() {
    use super::disable_xon_xoff_on_fd;

    fn get_termios(fd: i32) -> libc::termios {
        unsafe {
            let mut t = std::mem::MaybeUninit::<libc::termios>::uninit();
            assert_eq!(libc::tcgetattr(fd, t.as_mut_ptr()), 0);
            t.assume_init()
        }
    }

    fn set_termios(fd: i32, t: &libc::termios) {
        unsafe {
            assert_eq!(libc::tcsetattr(fd, libc::TCSANOW, t), 0);
        }
    }

    unsafe {
        let mut master: libc::c_int = 0;
        let mut slave: libc::c_int = 0;
        assert_eq!(
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut()
            ),
            0
        );

        // Ensure IXON is enabled so we can observe disable+restore.
        let mut t0 = get_termios(slave);
        t0.c_iflag |= libc::IXON;
        set_termios(slave, &t0);

        let echo_was_enabled = (t0.c_lflag & libc::ECHO) != 0;
        let expected_echo_enabled = !echo_was_enabled;

        {
            let _guard = disable_xon_xoff_on_fd(slave).expect("guard should be created for PTY");
            let t_mid = get_termios(slave);
            assert!((t_mid.c_iflag & libc::IXON) == 0);

            // Mutate an unrelated bit while guard is alive; the guard must not
            // overwrite it on drop.
            let mut t1 = t_mid;
            if echo_was_enabled {
                t1.c_lflag &= !libc::ECHO;
            } else {
                t1.c_lflag |= libc::ECHO;
            }
            set_termios(slave, &t1);
        }

        let t_after = get_termios(slave);
        assert!((t_after.c_iflag & libc::IXON) != 0);
        assert_eq!(
            (t_after.c_lflag & libc::ECHO) != 0,
            expected_echo_enabled,
            "TermiosGuard must not restore unrelated flags like ECHO"
        );

        let _ = libc::close(master);
        let _ = libc::close(slave);
    }
}

#[test]
fn sidebar_navigation_wraps_and_scrolls() {
    let mut app = build_test_app();
    // build_test_app only adds 1 project ("project-a")
    // sidebar_items() should return [Project(0), Launch(0), Settings(0), NewWorkspace]

    // Project rows are section headers: they render, but can't be selected/highlighted.
    app.sidebar_idx = 0;

    // Down -> Launch(0)
    app.handle_sidebar_key(key(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.sidebar_idx, 1);

    // Up -> Wrap to NewWorkspace (index 3), skipping Project(0)
    app.handle_sidebar_key(key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.sidebar_idx, 3);

    // Up -> Settings(0)
    app.handle_sidebar_key(key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.sidebar_idx, 2);

    // Down -> NewWorkspace
    app.handle_sidebar_key(key(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.sidebar_idx, 3);
}

#[test]
fn global_rules_external_edit_does_not_trigger_security_modal() {
    let mut app = build_test_app();
    let rules_path = app.config.get().manager.global_rules_file.clone();

    app.tick_base_rules_file_watch();
    std::fs::write(
        &rules_path,
        r#"
[hostdo]
default_policy = "prompt"
"#,
    )
    .expect("write base rules");

    std::thread::sleep(Duration::from_millis(800));
    app.tick_base_rules_file_watch();
    assert!(app.base_rules_changed.is_none());
}

#[test]
fn workspace_rules_external_edit_triggers_security_modal() {
    let mut app = build_test_app();
    let rules_path = app.config.get().workspaces[0]
        .canonical_path
        .join("harness-rules.toml");

    app.tick_base_rules_file_watch();
    std::fs::write(
        &rules_path,
        r#"
[hostdo]
default_policy = "prompt"
"#,
    )
    .expect("write workspace rules");

    std::thread::sleep(Duration::from_millis(800));
    app.tick_base_rules_file_watch();
    assert!(app.base_rules_changed.is_some());
    assert_eq!(
        app.base_rules_changed.as_ref().map(|s| s.path.clone()),
        Some(rules_path)
    );
}

#[test]
fn workspace_rules_created_by_cli_after_open_does_not_trigger_security_modal() {
    let mut app = build_test_app();
    let project_path = app.config.get().workspaces[0].canonical_path.clone();
    let rules_path = project_path.join("harness-rules.toml");

    assert!(!rules_path.exists());
    app.tick_base_rules_file_watch();

    let result = crate::agents::inject_agent_config(
        &crate::config::AgentKind::Codex,
        &project_path,
        &project_path,
        "project-a",
        true,
        std::path::Path::new("/workspace"),
        "http://127.0.0.1:0",
        "http://127.0.0.1:0",
        None,
    )
    .expect("inject agent config");
    let created = result.created_rules.expect("starter rules file created");
    assert_eq!(created.path, rules_path);
    app.record_completed_rules_internal_write(created.path, created.content);

    std::thread::sleep(Duration::from_millis(800));
    app.tick_base_rules_file_watch();
    assert!(app.base_rules_changed.is_none());
}

#[test]
fn workspace_rules_tampered_during_cli_create_still_triggers_security_modal() {
    let mut app = build_test_app();
    let project_path = app.config.get().workspaces[0].canonical_path.clone();
    let rules_path = project_path.join("harness-rules.toml");

    app.tick_base_rules_file_watch();

    let result = crate::agents::inject_agent_config(
        &crate::config::AgentKind::Codex,
        &project_path,
        &project_path,
        "project-a",
        true,
        std::path::Path::new("/workspace"),
        "http://127.0.0.1:0",
        "http://127.0.0.1:0",
        None,
    )
    .expect("inject agent config");
    let created = result.created_rules.expect("starter rules file created");
    std::fs::write(
        &rules_path,
        r#"
[hostdo]
default_policy = "deny"
"#,
    )
    .expect("tamper rules file");
    app.record_completed_rules_internal_write(created.path, created.content);

    std::thread::sleep(Duration::from_millis(800));
    app.tick_base_rules_file_watch();
    assert!(app.base_rules_changed.is_some());
    assert_eq!(
        app.base_rules_changed.as_ref().map(|s| s.path.clone()),
        Some(rules_path)
    );
}

#[test]
fn global_rules_internal_expected_write_is_not_alerted() {
    let mut app = build_test_app();
    let rules_path = app.config.get().manager.global_rules_file.clone();
    let new_content = r#"
[hostdo]
default_policy = "prompt"
"#
    .to_string();

    app.note_base_rules_internal_write(new_content.clone());
    std::fs::write(&rules_path, new_content).expect("write base rules");

    std::thread::sleep(Duration::from_millis(800));
    app.tick_base_rules_file_watch();
    assert!(app.base_rules_changed.is_none());
}

#[test]
fn global_rules_mismatched_write_is_not_alerted() {
    let mut app = build_test_app();
    let rules_path = app.config.get().manager.global_rules_file.clone();
    app.note_base_rules_internal_write(
        r#"
[hostdo]
default_policy = "deny"
"#
        .to_string(),
    );
    std::fs::write(
        &rules_path,
        r#"
[hostdo]
default_policy = "auto"
"#,
    )
    .expect("write base rules");

    std::thread::sleep(Duration::from_millis(800));
    app.tick_base_rules_file_watch();
    assert!(app.base_rules_changed.is_none());
}
