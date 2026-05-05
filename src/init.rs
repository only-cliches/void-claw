use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tracing::instrument;

const SAMPLE_CONFIG: &str = include_str!("../harness-hat.example.toml");
const DOCKER_DIR_PLACEHOLDER: &str = "__HARNESS_HAT_DOCKER_DIR__";
const BASE_DOCKERFILE_TEMPLATE: &str = include_str!("../docker/harness-hat-base.dockerfile");
const DEFAULT_DOCKERFILE_TEMPLATE: &str = include_str!("../docker/default.dockerfile");
const GITHUB_DOCKER_BASE_URL: &str =
    "https://raw.githubusercontent.com/only-cliches/harness-hat/refs/heads/main/docker";
const BUILTIN_DOCKERFILES: &[&str] = &["harness-hat-base.dockerfile"];

const HOSTDO_SCRIPT: &str = include_str!("../docker/scripts/hostdo.py");
const KILLME_SCRIPT: &str = include_str!("../docker/scripts/killme.py");

#[instrument(skip(output))]
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
        .join(".config/harness-hat");
    let docker_dir = resolve_init_docker_dir(&cwd, &home_config_root);
    fs::create_dir_all(&docker_dir)?;
    ensure_base_dockerfile(&docker_dir)?;
    ensure_default_dockerfile(&docker_dir)?;
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

#[instrument(skip(docker_dir))]
pub fn ensure_docker_assets(docker_dir: &Path) -> Result<()> {
    ensure_base_dockerfile(docker_dir)?;
    ensure_default_dockerfile(docker_dir)?;
    ensure_helper_scripts(docker_dir)?;

    let missing_dockerfiles = missing_builtin_dockerfiles(docker_dir);
    let missing_helper_scripts = missing_helper_scripts(docker_dir);

    if missing_dockerfiles.is_empty() && missing_helper_scripts.is_empty() {
        return Ok(());
    }

    println!(
        "harness-hat: the docker assets in {} are incomplete",
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
    ensure_base_dockerfile(docker_dir)?;
    ensure_default_dockerfile(docker_dir)?;
    ensure_helper_scripts(docker_dir)?;
    download_missing_dockerfiles(docker_dir, &missing_dockerfiles)?;
    Ok(())
}

#[instrument(skip(docker_dir))]
pub fn ensure_default_dockerfile(docker_dir: &Path) -> Result<()> {
    let path = docker_dir.join("default.dockerfile");
    if path.exists() {
        return Ok(());
    }
    write_text_file(&path, DEFAULT_DOCKERFILE_TEMPLATE)
}

#[instrument(skip(docker_dir))]
pub fn ensure_base_dockerfile(docker_dir: &Path) -> Result<()> {
    let path = docker_dir.join("harness-hat-base.dockerfile");
    if path.exists() {
        return Ok(());
    }
    write_text_file(&path, BASE_DOCKERFILE_TEMPLATE)
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

#[instrument(skip(docker_dir))]
pub fn ensure_helper_scripts(docker_dir: &Path) -> Result<()> {
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
        builtin_dockerfile_paths, ensure_base_dockerfile, ensure_default_dockerfile,
        ensure_docker_assets, resolve_init_docker_dir, write_sample_config,
    };
    use crate::config::Config;

    #[test]
    fn sample_config_writes_parseable_docker_dir() {
        let root = std::env::temp_dir().join(format!("harness-hat-init-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let output = root.join("harness-hat.toml");
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
            std::env::temp_dir().join(format!("harness-hat-init-local-{}", uuid::Uuid::new_v4()));
        let cwd = root.join("cwd");
        let home = root.join("home/.config/harness-hat");
        std::fs::create_dir_all(cwd.join("docker")).expect("create local docker dir");
        let selected = resolve_init_docker_dir(&cwd, &home);
        assert_eq!(selected, cwd.join("docker"));
    }

    #[test]
    fn resolve_init_docker_dir_falls_back_to_home_config_root() {
        let root =
            std::env::temp_dir().join(format!("harness-hat-init-home-{}", uuid::Uuid::new_v4()));
        let cwd = root.join("cwd");
        let home = root.join("home/.config/harness-hat");
        std::fs::create_dir_all(&cwd).expect("create cwd");
        let selected = resolve_init_docker_dir(&cwd, &home);
        assert_eq!(selected, home.join("docker"));
    }

    #[test]
    fn builtin_dockerfile_paths_include_expected_templates() {
        let paths = builtin_dockerfile_paths();
        assert!(paths.contains(&"harness-hat-base.dockerfile"));
    }

    #[test]
    fn ensure_docker_assets_is_a_noop_when_complete() {
        let root =
            std::env::temp_dir().join(format!("harness-hat-docker-{}", uuid::Uuid::new_v4()));
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

    #[test]
    fn ensure_docker_assets_refreshes_helper_scripts() {
        let root = std::env::temp_dir().join(format!(
            "harness-hat-docker-refresh-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(root.join("scripts")).expect("create scripts dir");
        for path in builtin_dockerfile_paths() {
            let file = root.join(path);
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent).expect("create dockerfile dir");
            }
            std::fs::write(&file, "FROM scratch").expect("write template");
        }
        std::fs::write(root.join("scripts/hostdo.py"), "old hostdo").expect("write old hostdo");
        std::fs::write(root.join("scripts/killme.py"), "old killme").expect("write old killme");

        ensure_docker_assets(&root).expect("ensure assets");

        let hostdo = std::fs::read_to_string(root.join("scripts/hostdo.py")).expect("read hostdo");
        let killme = std::fs::read_to_string(root.join("scripts/killme.py")).expect("read killme");
        assert!(hostdo.contains("X-Hostdo-Protocol"));
        assert!(killme.contains("killme"));
        assert_ne!(killme, "old killme");
    }

    #[test]
    fn ensure_default_dockerfile_creates_template_when_missing() {
        let root = std::env::temp_dir().join(format!(
            "harness-hat-default-dockerfile-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("create root");
        let path = root.join("default.dockerfile");
        assert!(!path.exists());

        ensure_default_dockerfile(&root).expect("write default dockerfile");
        assert!(path.exists());

        let content = std::fs::read_to_string(path).expect("read default dockerfile");
        assert!(content.contains("harness-hat default image"));
    }

    #[test]
    fn ensure_base_dockerfile_creates_template_when_missing() {
        let root = std::env::temp_dir().join(format!(
            "harness-hat-base-dockerfile-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("create root");
        let path = root.join("harness-hat-base.dockerfile");
        assert!(!path.exists());

        ensure_base_dockerfile(&root).expect("write base dockerfile");
        assert!(path.exists());

        let content = std::fs::read_to_string(path).expect("read base dockerfile");
        assert!(content.contains("harness-hat base"));
    }
}
