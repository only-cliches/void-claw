use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const SAMPLE_CONFIG: &str = include_str!("../agent-zero.example.toml");
const DOCKER_DIR_PLACEHOLDER: &str = "__AGENT_ZERO_DOCKER_DIR__";
const GITHUB_DOCKER_BASE_URL: &str =
    "https://raw.githubusercontent.com/only-cliches/agent-zero/refs/heads/main/docker";
const BUILTIN_DOCKERFILES: &[&str] = &[
    "ubuntu-24.04.Dockerfile",
    "claude/ubuntu-24.04.Dockerfile",
    "codex/ubuntu-24.04.Dockerfile",
    "gemini/ubuntu-24.04.Dockerfile",
    "opencode/ubuntu-24.04.Dockerfile",
];

const HOSTDO_SCRIPT: &str = include_str!("../docker/scripts/hostdo.py");
const KILLME_SCRIPT: &str = include_str!("../docker/scripts/killme.py");

pub fn write_sample_config(output: &Path) -> Result<()> {
    if output.exists() {
        bail!(
            "file already exists: {}  (delete it first or choose a different path)",
            output.display()
        );
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let cwd = std::env::current_dir()?;
    let home_config_root = dirs::home_dir()
        .context("could not determine home directory")?
        .join(".config/agent-zero");
    let docker_dir = resolve_init_docker_dir(&cwd, &home_config_root);
    fs::create_dir_all(&docker_dir)?;
    let docker_dir_literal = toml::Value::String(docker_dir.display().to_string()).to_string();
    let sample = SAMPLE_CONFIG.replace(DOCKER_DIR_PLACEHOLDER, &docker_dir_literal);
    std::fs::write(output, sample)?;
    Ok(())
}

fn resolve_init_docker_dir(cwd: &Path, home_config_root: &Path) -> PathBuf {
    let local_docker_dir = cwd.join("docker");
    if local_docker_dir.is_dir() {
        local_docker_dir
    } else {
        home_config_root.join("docker")
    }
}

pub fn ensure_docker_assets(docker_dir: &Path) -> Result<()> {
    let missing_dockerfiles = missing_builtin_dockerfiles(docker_dir);
    let missing_helper_scripts = missing_helper_scripts(docker_dir);

    if missing_dockerfiles.is_empty() && missing_helper_scripts.is_empty() {
        return Ok(());
    }

    println!(
        "agent-zero: the docker assets in {} are incomplete",
        docker_dir.display()
    );
    if !missing_dockerfiles.is_empty() {
        println!("  Missing Dockerfiles:");
        for file in &missing_dockerfiles {
            println!("    - {}", file.display());
        }
        println!("  These can be fetched from GitHub automatically.");
    }
    if !missing_helper_scripts.is_empty() {
        println!("  Missing helper scripts:");
        for file in &missing_helper_scripts {
            println!("    - {}", file.display());
        }
        println!("  These will be written from the installed binary.");
    }

    if !prompt_yes_no("Create the missing docker assets now? [y/N]: ")? {
        return Ok(());
    }

    fs::create_dir_all(docker_dir)?;
    write_helper_scripts(docker_dir)?;
    download_missing_dockerfiles(docker_dir, &missing_dockerfiles)?;
    Ok(())
}

#[cfg(test)]
pub fn builtin_dockerfile_paths() -> &'static [&'static str] {
    BUILTIN_DOCKERFILES
}

fn missing_builtin_dockerfiles(docker_dir: &Path) -> Vec<PathBuf> {
    BUILTIN_DOCKERFILES
        .iter()
        .map(|rel| docker_dir.join(rel))
        .filter(|path| !path.exists())
        .collect()
}

fn missing_helper_scripts(docker_dir: &Path) -> Vec<PathBuf> {
    helper_script_paths(docker_dir)
        .into_iter()
        .filter(|path| !path.exists())
        .collect()
}

fn helper_script_paths(docker_dir: &Path) -> Vec<PathBuf> {
    vec![
        docker_dir.join("scripts/hostdo.py"),
        docker_dir.join("scripts/killme.py"),
    ]
}

fn prompt_yes_no(prompt: &str) -> Result<bool> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn write_helper_scripts(docker_dir: &Path) -> Result<()> {
    let scripts_dir = docker_dir.join("scripts");
    fs::create_dir_all(&scripts_dir)?;
    write_text_file(&scripts_dir.join("hostdo.py"), HOSTDO_SCRIPT)?;
    write_text_file(&scripts_dir.join("killme.py"), KILLME_SCRIPT)?;
    Ok(())
}

fn download_missing_dockerfiles(docker_dir: &Path, missing: &[PathBuf]) -> Result<()> {
    if missing.is_empty() {
        return Ok(());
    }

    let client = Client::builder()
        .build()
        .context("creating HTTP client for docker asset download")?;

    for path in missing {
        let rel = path
            .strip_prefix(docker_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let url = format!("{GITHUB_DOCKER_BASE_URL}/{rel}");
        let response = client
            .get(&url)
            .send()
            .and_then(|resp| resp.error_for_status())
            .with_context(|| format!("downloading {rel} from GitHub"))?;
        let text = response
            .text()
            .with_context(|| format!("reading {rel} from GitHub"))?;
        write_text_file(path, &text)?;
    }

    Ok(())
}

fn write_text_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        builtin_dockerfile_paths, ensure_docker_assets, resolve_init_docker_dir,
        write_sample_config,
    };
    use crate::config::Config;

    #[test]
    fn sample_config_writes_parseable_docker_dir() {
        let root = std::env::temp_dir().join(format!("agent-zero-init-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let output = root.join("agent-zero.toml");
        let cwd = std::env::current_dir().expect("current dir");
        let sample = write_sample_config(&output);
        sample.expect("write sample config");

        let contents = std::fs::read_to_string(&output).expect("read sample config");
        let parsed: Config = toml::from_str(&contents).expect("parse sample config");
        assert_eq!(parsed.docker_dir, cwd.join("docker"));
    }

    #[test]
    fn resolve_init_docker_dir_prefers_local_docker_folder() {
        let root =
            std::env::temp_dir().join(format!("agent-zero-init-local-{}", uuid::Uuid::new_v4()));
        let cwd = root.join("cwd");
        let home = root.join("home/.config/agent-zero");
        std::fs::create_dir_all(cwd.join("docker")).expect("create local docker dir");
        let selected = resolve_init_docker_dir(&cwd, &home);
        assert_eq!(selected, cwd.join("docker"));
    }

    #[test]
    fn resolve_init_docker_dir_falls_back_to_home_config_root() {
        let root =
            std::env::temp_dir().join(format!("agent-zero-init-home-{}", uuid::Uuid::new_v4()));
        let cwd = root.join("cwd");
        let home = root.join("home/.config/agent-zero");
        std::fs::create_dir_all(&cwd).expect("create cwd");
        let selected = resolve_init_docker_dir(&cwd, &home);
        assert_eq!(selected, home.join("docker"));
    }

    #[test]
    fn builtin_dockerfile_paths_include_expected_templates() {
        let paths = builtin_dockerfile_paths();
        assert!(paths.contains(&"ubuntu-24.04.Dockerfile"));
        assert!(paths.contains(&"codex/ubuntu-24.04.Dockerfile"));
        assert!(paths.contains(&"claude/ubuntu-24.04.Dockerfile"));
        assert!(paths.contains(&"gemini/ubuntu-24.04.Dockerfile"));
        assert!(paths.contains(&"opencode/ubuntu-24.04.Dockerfile"));
    }

    #[test]
    fn ensure_docker_assets_is_a_noop_when_complete() {
        let root = std::env::temp_dir().join(format!("agent-zero-docker-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("scripts")).expect("create scripts dir");
        for path in builtin_dockerfile_paths() {
            let file = root.join(path);
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent).expect("create dockerfile dir");
            }
            std::fs::write(&file, "FROM scratch").expect("write template");
        }
        std::fs::write(root.join("scripts/hostdo.py"), "hostdo").expect("write hostdo");
        std::fs::write(root.join("scripts/killme.py"), "killme").expect("write killme");

        ensure_docker_assets(&root).expect("ensure assets");
    }
}
