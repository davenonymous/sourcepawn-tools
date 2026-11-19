// Copyright 2026 davenonymous

//! Finding the files to check and the configuration file that applies to
//! each of them.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

/// The extensions of SourcePawn files looked for in directories.
pub const SOURCEPAWN_EXTENSIONS: &[&str] = &["sp", "inc"];

/// How directories are walked.
#[derive(Debug, Clone, Copy)]
pub struct DiscoverOptions {
    /// Skip files that `.gitignore`, `.ignore` and git's excludes leave out,
    /// and hidden files and directories.
    pub respect_ignore_files: bool,
}

/// Why the files could not be listed.
#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error("{path}: no such file or directory")]
    NotFound { path: PathBuf },
    #[error("{path}: is neither a file nor a directory")]
    NotFileOrDirectory { path: PathBuf },
    #[error("{path}: cannot list: {message}")]
    Walk { path: PathBuf, message: String },
}

/// The files to work on: every file named in `paths`, whatever its
/// extension, and every `.sp` and `.inc` file below the directories named
/// in `paths`, sorted by path.
///
/// A file reached under several spellings (`a.sp`, `./a.sp`, or relative
/// and absolute) is listed once, under the spelling reached first, the
/// paths taken in command line order. Two spellings are the same file when
/// their paths, made absolute and without `.` components, are equal. `..`
/// components are kept: the component before one may be a symbolic link,
/// so `dir/../a.sp` need not be `a.sp`.
pub fn discover_files(paths: &[PathBuf], options: DiscoverOptions) -> Result<Vec<PathBuf>, DiscoverError> {
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for path in paths {
        let found = if path.is_file() { vec![path.clone()] } else { walk_named_directory(path, options)? };
        for file in found {
            if seen.insert(identity_of(&file)) {
                files.push(file);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// What tells two spellings of a file apart: the path joined to the
/// current directory when relative, without its `.` components. Unlike
/// [`std::path::absolute`] on Windows, this keeps `..` on every platform.
fn identity_of(path: &Path) -> PathBuf {
    let absolute = match std::env::current_dir() {
        Ok(directory) if path.is_relative() => directory.join(path),
        _ => path.to_path_buf(),
    };
    absolute.components().filter(|component| *component != Component::CurDir).collect()
}

/// The SourcePawn files below `path`, which the command line named and
/// which is no file.
fn walk_named_directory(path: &Path, options: DiscoverOptions) -> Result<Vec<PathBuf>, DiscoverError> {
    if path.is_dir() {
        return walk_directory(path, options);
    }
    if path.exists() {
        return Err(DiscoverError::NotFileOrDirectory { path: path.to_path_buf() });
    }
    Err(DiscoverError::NotFound { path: path.to_path_buf() })
}

fn walk_directory(directory: &Path, options: DiscoverOptions) -> Result<Vec<PathBuf>, DiscoverError> {
    let walker = ignore::WalkBuilder::new(directory)
        .standard_filters(options.respect_ignore_files)
        .require_git(false)
        .sort_by_file_name(|a, b| a.cmp(b))
        .build();
    let mut files = Vec::new();
    for entry in walker {
        let entry =
            entry.map_err(|error| DiscoverError::Walk { path: directory.to_path_buf(), message: error.to_string() })?;
        let is_file = entry.file_type().is_some_and(|file_type| file_type.is_file());
        if is_file && has_sourcepawn_extension(entry.path()) {
            files.push(entry.into_path());
        }
    }
    Ok(files)
}

fn has_sourcepawn_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| SOURCEPAWN_EXTENSIONS.iter().any(|known| extension.eq_ignore_ascii_case(known)))
}

/// Finds the nearest configuration file with one of a set of names, from a
/// file's directory upwards, and remembers the answer per directory.
#[derive(Debug)]
pub(crate) struct ConfigLocator {
    file_names: Vec<String>,
    found: HashMap<PathBuf, Option<PathBuf>>,
}

impl ConfigLocator {
    pub(crate) fn new(file_names: &[&str]) -> ConfigLocator {
        ConfigLocator { file_names: file_names.iter().map(|name| name.to_string()).collect(), found: HashMap::new() }
    }

    /// The configuration file for `file`: the first of the names that
    /// exists in the file's directory or the closest directory above it.
    ///
    /// The path is made absolute and normalized lexically, as it is
    /// spelled: `.` is dropped and `..` removes the component before it.
    /// Symbolic links are not resolved, so a file is configured by the
    /// directories its path names, not by those its link target lies in.
    pub(crate) fn locate_for_file(&mut self, file: &Path) -> Option<PathBuf> {
        let path = absolute_lexical(file);
        let directory = path.parent().map(Path::to_path_buf).unwrap_or_default();
        self.locate_above(&directory)
    }

    /// The configuration file for files in `directory`, made absolute and
    /// normalized like the path of [`ConfigLocator::locate_for_file`].
    pub(crate) fn locate_from(&mut self, directory: &Path) -> Option<PathBuf> {
        self.locate_above(&absolute_lexical(directory))
    }

    /// The configuration file in the normalized `directory` or above it.
    fn locate_above(&mut self, directory: &Path) -> Option<PathBuf> {
        if let Some(known) = self.found.get(directory) {
            return known.clone();
        }
        let here = self.file_names.iter().map(|name| directory.join(name)).find(|candidate| candidate.is_file());
        let result = match here {
            Some(found) => Some(found),
            None => directory.parent().and_then(|parent| self.locate_above(parent)),
        };
        self.found.insert(directory.to_path_buf(), result.clone());
        result
    }
}

/// `path` made absolute and normalized lexically, like
/// [`normalize_lexically`].
fn absolute_lexical(path: &Path) -> PathBuf {
    normalize_lexically(&std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()))
}

/// `path` normalized lexically, as it is spelled: without `.` components,
/// each `..` removing the component before it (`..` at the root stays at
/// the root, and a relative path keeps the `..` it starts with). The file
/// system is not asked, so symbolic links are not resolved.
pub fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let can_pop = matches!(normalized.components().next_back(), Some(Component::Normal(_)));
                if can_pop {
                    normalized.pop();
                } else if !normalized.has_root() {
                    normalized.push(component);
                }
            }
            other => normalized.push(other),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directories_yield_sourcepawn_files_and_named_files_are_kept() {
        let root = tempfile::tempdir().unwrap();
        let scripting = root.path().join("scripting");
        std::fs::create_dir_all(scripting.join("include")).unwrap();
        std::fs::write(scripting.join("a.sp"), "").unwrap();
        std::fs::write(scripting.join("include/b.inc"), "").unwrap();
        std::fs::write(scripting.join("notes.txt"), "").unwrap();
        let other = root.path().join("other.txt");
        std::fs::write(&other, "").unwrap();
        let options = DiscoverOptions { respect_ignore_files: true };
        let files = discover_files(&[scripting.clone(), other.clone()], options).unwrap();
        assert_eq!(files, vec![other, scripting.join("a.sp"), scripting.join("include/b.inc")]);
    }

    #[test]
    fn missing_paths_fail() {
        let options = DiscoverOptions { respect_ignore_files: true };
        let error = discover_files(&[PathBuf::from("/does/not/exist.sp")], options).unwrap_err();
        assert!(matches!(error, DiscoverError::NotFound { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn paths_that_are_neither_files_nor_directories_fail() {
        let options = DiscoverOptions { respect_ignore_files: true };
        let error = discover_files(&[PathBuf::from("/dev/null")], options).unwrap_err();
        assert_eq!(error.to_string(), "/dev/null: is neither a file nor a directory");
    }

    #[test]
    fn a_file_reached_under_several_spellings_is_listed_once() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("dir")).unwrap();
        std::fs::write(root.path().join("a.sp"), "").unwrap();
        let spellings = [root.path().join("./a.sp"), root.path().join("a.sp"), root.path().join("dir/../a.sp")];
        let options = DiscoverOptions { respect_ignore_files: true };
        let files = discover_files(&spellings, options).unwrap();
        assert_eq!(files, [root.path().join("./a.sp"), root.path().join("dir/../a.sp")], "`..` is kept");
        let with_directory = discover_files(&[root.path().join("a.sp"), root.path().to_path_buf()], options).unwrap();
        assert_eq!(with_directory, [root.path().join("a.sp")]);
    }

    #[test]
    fn the_nearest_config_wins() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.path().join(".tool.toml"), "").unwrap();
        std::fs::write(root.path().join("a/.tool.toml"), "").unwrap();
        let mut locator = ConfigLocator::new(&[".tool.toml"]);
        assert_eq!(locator.locate_for_file(&nested.join("x.sp")), Some(root.path().join("a/.tool.toml")));
    }

    #[test]
    fn a_path_through_a_sibling_directory_gets_no_config_of_that_directory() {
        let root = tempfile::tempdir().unwrap();
        let sub = root.path().join("proj/sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join(".tool.toml"), "").unwrap();
        std::fs::write(root.path().join("proj/file.sp"), "").unwrap();
        let mut locator = ConfigLocator::new(&[".tool.toml"]);
        assert_eq!(locator.locate_for_file(&sub.join("../file.sp")), None);
        assert_eq!(locator.locate_from(&sub.join("..")), None);
        assert_eq!(locator.locate_from(&sub.join("./.")), Some(sub.join(".tool.toml")));
    }

    #[test]
    fn paths_are_normalized_as_spelled() {
        let root = std::path::absolute("/").unwrap();
        assert_eq!(absolute_lexical(&root.join("a/./b/../c")), root.join("a/c"));
        assert_eq!(absolute_lexical(&root.join("../a")), root.join("a"));
        assert_eq!(normalize_lexically(Path::new("one/../critic/./x")), Path::new("critic/x"));
        assert_eq!(normalize_lexically(Path::new("../a/../../b")), Path::new("../../b"));
    }
}
