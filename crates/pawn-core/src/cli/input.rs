// Copyright 2026 davenonymous

//! The files a run works on: the paths of the command line, or standard
//! input under a name.

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::discover::{DiscoverError, DiscoverOptions, discover_files};
use crate::source::{LoadError, SourceFile};

/// The name standard input has without `--stdin-filename`.
pub const DEFAULT_STDIN_NAME: &str = "stdin.sp";

/// Why the inputs of a run cannot be collected.
#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("`-` reads standard input and cannot be combined with other paths")]
    StdinWithOtherPaths,
    #[error("--stdin-filename needs `-` as the path")]
    StdinFilenameWithoutStdin,
    #[error("cannot read standard input: {0}")]
    Stdin(std::io::Error),
    #[error(transparent)]
    Discover(#[from] DiscoverError),
}

/// Where the inputs of a run come from, as the command line says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputSource {
    /// Standard input, worked on as the file `name`.
    Stdin { name: PathBuf },
    /// Files, and directories searched for SourcePawn files.
    Paths(Vec<PathBuf>),
}

impl InputSource {
    /// Reads the paths of the command line: `-` alone is standard input,
    /// named by `stdin_filename` (by default [`DEFAULT_STDIN_NAME`]); no
    /// paths at all is the current directory.
    pub fn from_arguments(paths: &[PathBuf], stdin_filename: Option<&Path>) -> Result<InputSource, InputError> {
        let reads_stdin = paths.iter().any(|path| path.as_os_str() == "-");
        if reads_stdin && paths.len() > 1 {
            return Err(InputError::StdinWithOtherPaths);
        }
        if reads_stdin {
            let name = stdin_filename.map_or_else(|| PathBuf::from(DEFAULT_STDIN_NAME), Path::to_path_buf);
            return Ok(InputSource::Stdin { name });
        }
        if stdin_filename.is_some() {
            return Err(InputError::StdinFilenameWithoutStdin);
        }
        let paths = if paths.is_empty() { vec![PathBuf::from(".")] } else { paths.to_vec() };
        Ok(InputSource::Paths(paths))
    }

    /// The inputs: standard input read from `stdin`, or the files the paths
    /// name and those found below its directories.
    pub fn collect(self, options: DiscoverOptions, mut stdin: impl Read) -> Result<Vec<Input>, InputError> {
        match self {
            InputSource::Stdin { name } => {
                let mut content = Vec::new();
                stdin.read_to_end(&mut content).map_err(InputError::Stdin)?;
                Ok(vec![Input { path: name, content: Some(content) }])
            }
            InputSource::Paths(paths) => {
                let files = discover_files(&paths, options)?;
                Ok(files.into_iter().map(|path| Input { path, content: None }).collect())
            }
        }
    }
}

/// One file a run works on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    /// The path the file is read from, or the name standard input is
    /// worked on under.
    pub path: PathBuf,
    /// The content when it does not come from the path (standard input).
    pub content: Option<Vec<u8>>,
}

impl Input {
    /// The decoded file: its content, or what its path holds.
    pub fn load(&self) -> Result<SourceFile, LoadError> {
        match &self.content {
            Some(bytes) => SourceFile::from_bytes(&self.path, bytes.clone()),
            None => SourceFile::read(&self.path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn standard_input_stands_alone() {
        let error = InputSource::from_arguments(&paths(&["-", "a.sp"]), None).unwrap_err();
        assert!(matches!(error, InputError::StdinWithOtherPaths), "{error}");
        let error = InputSource::from_arguments(&paths(&["a.sp"]), Some(Path::new("b.sp"))).unwrap_err();
        assert!(matches!(error, InputError::StdinFilenameWithoutStdin), "{error}");
    }

    #[test]
    fn standard_input_is_read_under_its_name() {
        let default = InputSource::from_arguments(&paths(&["-"]), None).unwrap();
        assert_eq!(default, InputSource::Stdin { name: PathBuf::from("stdin.sp") });
        let named = InputSource::from_arguments(&paths(&["-"]), Some(Path::new("dir/x.inc"))).unwrap();
        let options = DiscoverOptions { respect_ignore_files: true };
        let inputs = named.collect(options, &b"int x;\n"[..]).unwrap();
        assert_eq!(inputs, vec![Input { path: PathBuf::from("dir/x.inc"), content: Some(b"int x;\n".to_vec()) }]);
        let source = inputs[0].load().unwrap();
        assert_eq!((source.name(), source.text()), (Path::new("dir/x.inc"), "int x;\n"));
    }

    #[test]
    fn no_paths_is_the_current_directory() {
        assert_eq!(InputSource::from_arguments(&[], None).unwrap(), InputSource::Paths(paths(&["."])));
    }

    #[test]
    fn paths_are_discovered_and_loaded_from_disk() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.sp"), "int a;\n").unwrap();
        let source = InputSource::from_arguments(&[root.path().to_path_buf()], None).unwrap();
        let options = DiscoverOptions { respect_ignore_files: true };
        let inputs = source.collect(options, std::io::empty()).unwrap();
        assert_eq!(inputs, vec![Input { path: root.path().join("a.sp"), content: None }]);
        assert_eq!(inputs[0].load().unwrap().text(), "int a;\n");
    }
}
