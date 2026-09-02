use std::path::{Path, PathBuf};

/// Resolves `path` relative to the process's current working directory,
/// and rejects it if it escapes that directory (e.g. via `..` or an absolute path elsewhere).
pub fn resolve_within_root(path: &str) -> Result<PathBuf, String> {
    let root = std::env::current_dir().map_err(|e| format!("failed to get cwd: {}", e))?;
    let root_canonical = root
        .canonicalize()
        .map_err(|e| format!("failed to resolve root: {}", e))?;

    let candidate = Path::new(path);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };

    // canonicalize resolves symlinks and `..` — but the target must already exist for this to work.
    // for paths that don't exist yet (e.g. a new file to write), canonicalize the parent instead.
    let canonical = match joined.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            let parent = joined
                .parent()
                .ok_or_else(|| "invalid path".to_string())?;
            let parent_canonical = parent
                .canonicalize()
                .map_err(|e| format!("invalid path: {}", e))?;
            let file_name = joined
                .file_name()
                .ok_or_else(|| "invalid path".to_string())?;
            parent_canonical.join(file_name)
        }
    };

    if !canonical.starts_with(&root_canonical) {
        return Err(format!(
            "access denied: '{}' is outside the project directory",
            path
        ));
    }

    Ok(canonical)
}
