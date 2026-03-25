use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const VIEWER_SCRIPT_FILE_NAME: &str = "live-trace-matrix.js";
const GENERATED_VIEWER_ROOT_PREFIX: &str = "viewer-v";
const EMBEDDED_VIEWER_SCRIPT: &str = include_str!("../assets/live-trace-matrix.js");

pub fn viewer_script_path() -> Result<PathBuf, String> {
    if let Ok(path) = env::var("EBPF_TRACKER_VIEWER_SCRIPT") {
        let viewer_script = PathBuf::from(path);
        if viewer_script.is_file() {
            return Ok(viewer_script);
        }
        return Err(format!(
            "viewer script from EBPF_TRACKER_VIEWER_SCRIPT not found: {}",
            viewer_script.display()
        ));
    }

    let source_asset = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(VIEWER_SCRIPT_FILE_NAME);
    if source_asset.is_file() {
        return Ok(source_asset);
    }

    let mut errors = Vec::new();

    for root in cache_root_candidates() {
        let viewer_root = root.join(format!(
            "{GENERATED_VIEWER_ROOT_PREFIX}{}",
            env!("CARGO_PKG_VERSION")
        ));
        let result = (|| -> Result<PathBuf, String> {
            let viewer_script = viewer_root.join(VIEWER_SCRIPT_FILE_NAME);
            write_if_changed(&viewer_script, EMBEDDED_VIEWER_SCRIPT)?;
            Ok(viewer_script)
        })();

        match result {
            Ok(viewer_script) => return Ok(viewer_script),
            Err(err) => errors.push(err),
        }
    }

    Err(format!(
        "failed to materialize viewer assets: {}",
        errors.join("; ")
    ))
}

pub fn build_node_command(args: &[String]) -> Result<Command, String> {
    let viewer_script = viewer_script_path()?;
    let mut command = Command::new("node");
    command.arg(viewer_script).args(args);
    Ok(command)
}

fn cache_root_candidates() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(path) = env::var("EBPF_TRACKER_CACHE_DIR") {
        roots.push(PathBuf::from(path));
        return roots;
    }

    if let Ok(path) = env::var("XDG_CACHE_HOME") {
        roots.push(PathBuf::from(path).join("ebpf-tracker"));
    }

    if let Ok(path) = env::var("HOME") {
        roots.push(PathBuf::from(path).join(".cache").join("ebpf-tracker"));
    }

    roots.push(env::temp_dir().join("ebpf-tracker"));
    roots
}

fn write_if_changed(path: &Path, content: &str) -> Result<(), String> {
    if path.exists() {
        let current = fs::read_to_string(path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        if current == content {
            return Ok(());
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }

    fs::write(path, content).map_err(|err| format!("failed to write {}: {err}", path.display()))
}
