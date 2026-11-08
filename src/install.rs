use std::fs;
use std::path::{Path, PathBuf};

const SKILL: &str = include_str!("../skills/sumi/SKILL.md");
const LICENSE: &str = include_str!("../LICENSE");
const USAGE: &str = "usage: sumi install-skill --path PATH | --target codex";

pub fn run(args: &[&str]) -> Result<PathBuf, String> {
    let path = match args {
        ["--path", path] if !path.is_empty() && !path.starts_with('-') => PathBuf::from(path),
        ["--target", target] => target_path(target)?,
        _ => return Err(format!("invalid installation arguments\n{USAGE}")),
    };
    let path = std::env::current_dir()
        .map_err(|e| format!("cannot get the working directory: {e}"))?
        .join(path);
    let executable = std::env::current_exe()
        .map_err(|e| format!("cannot locate the running executable: {e}"))?;
    install(&path, &executable)?;
    Ok(path)
}

fn target_path(target: &str) -> Result<PathBuf, String> {
    match target {
        "codex" => {
            let home = std::env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .ok_or("HOME is not set or is empty; use install-skill --path PATH")?;
            Ok(PathBuf::from(home).join(".agents/skills/sumi"))
        }
        _ => Err(format!(
            "unsupported target: {target}; supported targets: codex"
        )),
    }
}

fn install(path: &Path, executable: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create parent directory {}: {e}", parent.display()))?;
    }
    fs::create_dir(path).map_err(|e| {
        format!(
            "cannot create skill directory {} (it must not exist): {e}",
            path.display()
        )
    })?;

    let result = (|| {
        let binary = path.join(format!("sumi{}", std::env::consts::EXE_SUFFIX));
        fs::copy(executable, &binary)
            .map_err(|e| format!("cannot copy executable to {}: {e}", binary.display()))?;
        let manifest = path.join("SKILL.md");
        fs::write(&manifest, SKILL)
            .map_err(|e| format!("cannot write {}: {e}", manifest.display()))?;
        let license = path.join("LICENSE");
        fs::write(&license, LICENSE)
            .map_err(|e| format!("cannot write {}: {e}", license.display()))?;
        Ok(())
    })();
    if let Err(reason) = result {
        return match fs::remove_dir_all(path) {
            Ok(()) => Err(reason),
            Err(e) => Err(format!("{reason}; cannot clean up {}: {e}", path.display())),
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_copy_removes_the_skill_directory_and_keeps_the_parent() {
        let parent = std::env::temp_dir().join(format!("sumi-copy-failure-{}", std::process::id()));
        fs::create_dir(&parent).unwrap();
        let destination = parent.join("skill");
        let result = install(&destination, &parent.join("missing-executable"));
        assert!(result.unwrap_err().contains("cannot copy executable"));
        assert!(!destination.exists());
        assert!(parent.is_dir());
        fs::remove_dir(parent).unwrap();
    }
}
