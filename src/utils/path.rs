use std::path::{Path, PathBuf};

/// Shorten a path for display, replacing home directory with ~ and truncating long paths
pub fn shorten_path_for_display(path: &str) -> String {
    let path_buf = Path::new(path);

    // Try to replace home directory with ~
    if let Some(user_dirs) = directories::UserDirs::new() {
        let home_dir = user_dirs.home_dir();
        if let Ok(relative_path) = path_buf.strip_prefix(home_dir) {
            let home_path = if relative_path.as_os_str().is_empty() {
                "~".to_string() // Just home directory
            } else {
                format!("~/{}", relative_path.to_string_lossy())
            };
            return shorten_long_path(&home_path);
        }
    }

    shorten_long_path(path)
}

/// Shorten very long paths by truncating the middle
fn shorten_long_path(path: &str) -> String {
    const MAX_LENGTH: usize = 50;

    if path.len() <= MAX_LENGTH {
        return path.to_string();
    }

    // For very long paths, show start...end
    let start_len = MAX_LENGTH / 2 - 2;
    let end_len = MAX_LENGTH / 2 - 1;

    format!("{}...{}", &path[..start_len], &path[path.len() - end_len..])
}

/// Canonicalize a path, resolving symlinks and normalizing
pub fn canonicalize_path(path: &Path) -> anyhow::Result<PathBuf> {
    // Convert to absolute path first if needed
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    // Canonicalize to resolve symlinks and normalize
    let canonical_path = absolute_path.canonicalize().unwrap_or(absolute_path); // Fall back if canonicalize fails

    Ok(canonical_path)
}
