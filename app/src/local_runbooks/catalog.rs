//! Local Markdown discovery. Scanning never evaluates or executes document contents.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub fn discover(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut budget = 20_000usize;
    for root in roots {
        if !root.is_absolute() {
            continue;
        }
        if let Ok(root) = root.canonicalize() {
            scan(&root, 0, &mut budget, &mut visited, &mut files);
        }
    }
    files.into_iter().collect()
}

fn scan(
    dir: &Path,
    depth: usize,
    budget: &mut usize,
    visited: &mut BTreeSet<PathBuf>,
    files: &mut BTreeSet<PathBuf>,
) {
    if depth > 16 || *budget == 0 || !visited.insert(dir.to_owned()) {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries = entries
        .take(*budget)
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() && !entry.file_name().to_string_lossy().starts_with('.') {
            scan(&path, depth + 1, budget, visited, files);
        } else if kind.is_file()
            && path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown")
                })
        {
            files.insert(path);
        }
        // Nested symlinks are not traversed, so folders cannot escape the selected roots.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_markdown_only_refreshes_and_deduplicates() {
        let root = std::env::temp_dir().join(format!("warpeer-runbooks-{}", std::process::id()));
        std::fs::create_dir_all(root.join("nested")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        for file in [
            "first.md",
            "nested/second.MARKDOWN",
            "no.yaml",
            ".git/hidden.md",
        ] {
            std::fs::write(root.join(file), "# Example\n```sh\necho hello\n```\n").unwrap();
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&root, root.join("nested/loop")).unwrap();
        let root = root.canonicalize().unwrap();
        let roots = vec![root.clone(), root.join("nested"), root.join("missing")];
        assert_eq!(discover(&roots).len(), 2);
        std::fs::remove_file(root.join("first.md")).unwrap();
        assert_eq!(discover(&roots), vec![root.join("nested/second.MARKDOWN")]);
        assert!(discover(&[PathBuf::from("relative")]).is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
