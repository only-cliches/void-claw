use anyhow::{Context, Result};
use std::env;
use std::path::Path;

pub(crate) fn read_container_id(cidfile: &Path, docker_name: &str) -> Result<String> {
    for _ in 0..400 {
        if let Ok(contents) = std::fs::read_to_string(cidfile) {
            let id = contents.trim().to_string();
            if !id.is_empty() {
                return Ok(id);
            }
        }
        if let Some(id) = inspect_container_id(docker_name)? {
            return Ok(id);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    anyhow::bail!(
        "failed to read docker container id from {} or inspect container {}",
        cidfile.display(),
        docker_name
    );
}

fn inspect_container_id(docker_name: &str) -> Result<Option<String>> {
    let output = std::process::Command::new("docker")
        .args(["inspect", "--format", "{{.Id}}", docker_name])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .context("running docker inspect")?;

    if !output.status.success() {
        return Ok(None);
    }

    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if id.is_empty() {
        Ok(None)
    } else {
        Ok(Some(id))
    }
}

pub fn inspect_container_exit(docker_name: &str) -> Result<Option<(Option<i32>, String)>> {
    let output = std::process::Command::new("docker")
        .args([
            "inspect",
            "--format",
            "{{.State.ExitCode}}|{{.State.Error}}",
            docker_name,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .context("running docker inspect")?;

    if !output.status.success() {
        return Ok(None);
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let mut parts = raw.trim().splitn(2, '|');
    let exit_code = parts.next().and_then(|s| s.trim().parse::<i32>().ok());
    let error = parts.next().unwrap_or("").trim().to_string();
    Ok(Some((exit_code, error)))
}

pub(crate) fn compose_no_proxy(bypass_proxy: &[String]) -> String {
    let mut entries = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "host.docker.internal".to_string(),
    ];
    for host in bypass_proxy {
        let host = host.trim();
        if host.is_empty() {
            continue;
        }
        if !entries.iter().any(|e| e == host) {
            entries.push(host.to_string());
        }
    }
    entries.join(",")
}

pub(crate) fn detect_default_colors() -> ((u8, u8, u8), (u8, u8, u8)) {
    parse_colorfgbg(env::var("COLORFGBG").ok().as_deref())
}

fn parse_colorfgbg(colorfgbg: Option<&str>) -> ((u8, u8, u8), (u8, u8, u8)) {
    let fallback = (ansi_index_to_rgb(15), ansi_index_to_rgb(0));
    let Some(val) = colorfgbg else {
        return fallback;
    };
    let parts: Vec<u8> = val
        .split(';')
        .filter_map(|s| s.trim().parse::<u8>().ok())
        .collect();
    if parts.len() < 2 {
        return fallback;
    }
    let fg_idx = parts[parts.len().saturating_sub(2)];
    let bg_idx = parts[parts.len().saturating_sub(1)];
    if fg_idx == bg_idx {
        return fallback;
    }
    let fg = ansi_index_to_rgb(fg_idx);
    let bg = ansi_index_to_rgb(bg_idx);
    if fg == bg {
        return fallback;
    }
    (fg, bg)
}

fn ansi_index_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0 => (0x00, 0x00, 0x00),
        1 => (0xcd, 0x00, 0x00),
        2 => (0x00, 0xcd, 0x00),
        3 => (0xcd, 0xcd, 0x00),
        4 => (0x00, 0x00, 0xee),
        5 => (0xcd, 0x00, 0xcd),
        6 => (0x00, 0xcd, 0xcd),
        7 => (0xe5, 0xe5, 0xe5),
        8 => (0x7f, 0x7f, 0x7f),
        9 => (0xff, 0x00, 0x00),
        10 => (0x00, 0xff, 0x00),
        11 => (0xff, 0xff, 0x00),
        12 => (0x5c, 0x5c, 0xff),
        13 => (0xff, 0x00, 0xff),
        14 => (0x00, 0xff, 0xff),
        _ => (0xff, 0xff, 0xff),
    }
}

pub(crate) fn xterm_256_index_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0..=15 => ansi_index_to_rgb(idx),
        16..=231 => {
            let i = idx - 16;
            let r = i / 36;
            let g = (i / 6) % 6;
            let b = i % 6;
            (xterm_cube(r), xterm_cube(g), xterm_cube(b))
        }
        232..=255 => {
            let shade = 8 + (idx - 232) * 10;
            (shade, shade, shade)
        }
    }
}

fn xterm_cube(v: u8) -> u8 {
    match v {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}

pub(crate) fn blend_toward_bg(fg: (u8, u8, u8), bg: (u8, u8, u8), fg_weight: f32) -> (u8, u8, u8) {
    let fg_weight = fg_weight.clamp(0.0, 1.0);
    let bg_weight = 1.0 - fg_weight;
    let blend = |f: u8, b: u8| -> u8 {
        ((f as f32) * fg_weight + (b as f32) * bg_weight)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    (blend(fg.0, bg.0), blend(fg.1, bg.1), blend(fg.2, bg.2))
}

pub(crate) fn luma_u8((r, g, b): (u8, u8, u8)) -> u8 {
    let y = 0.2126 * (r as f32) + 0.7152 * (g as f32) + 0.0722 * (b as f32);
    y.round().clamp(0.0, 255.0) as u8
}
