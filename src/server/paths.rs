use std::path::{Path, PathBuf};

pub(super) fn safe_path(shared_dir: &Path, url: &str, allow_missing_leaf: bool) -> Option<PathBuf> {
    let relative = url.trim_start_matches('/');

    if relative.is_empty() {
        return Some(shared_dir.to_path_buf());
    }

    let normalized = relative.trim_end_matches('/');

    if normalized.is_empty() {
        return Some(shared_dir.to_path_buf());
    }

    if normalized
        .split('/')
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return None;
    }

    let path = shared_dir.join(normalized);

    if path.exists() {
        return canonicalize_within_root(shared_dir, &path);
    }

    if !allow_missing_leaf {
        return None;
    }

    let parent = path.parent()?;
    canonicalize_within_root(shared_dir, parent)?;
    Some(path)
}

fn canonicalize_within_root(shared_dir: &Path, path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    if canonical.starts_with(shared_dir) {
        Some(canonical)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::safe_path;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("file-transfer-test-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    #[test]
    fn safe_path_allows_existing_child_within_root() {
        let root = make_temp_dir();
        let nested = root.join("docs");
        fs::create_dir_all(&nested).unwrap();

        let resolved = safe_path(&root, "/docs/", false);
        assert_eq!(resolved, Some(nested.canonicalize().unwrap()));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn safe_path_rejects_dot_segments() {
        let root = make_temp_dir();

        assert!(safe_path(&root, "/docs/../secret.txt", false).is_none());
        assert!(safe_path(&root, "/./secret.txt", true).is_none());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn safe_path_allows_new_upload_inside_existing_parent() {
        let root = make_temp_dir();
        let uploads = root.join("uploads");
        fs::create_dir_all(&uploads).unwrap();

        let resolved = safe_path(&root, "/uploads/new.txt", true);
        assert_eq!(resolved, Some(uploads.join("new.txt")));

        fs::remove_dir_all(root).unwrap();
    }
}
