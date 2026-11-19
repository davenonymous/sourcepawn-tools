// Copyright 2026 davenonymous

//! The configuration file that applies to each file of a run: the nearest
//! one above it, one named on the command line, or none at all. Each file
//! is read once per run.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::discover::ConfigLocator;

/// Why a configuration file cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{path}: cannot read: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{path}: {message}")]
    Invalid { path: PathBuf, message: String },
}

/// A configuration a tool reads from a file.
pub trait ConfigFile: Default {
    /// The names a configuration file may have, in the order they are
    /// looked for in a directory.
    const FILE_NAMES: &'static [&'static str];

    /// Reads the configuration file at `path`.
    fn read(path: &Path) -> Result<Self, ConfigError>;
}

/// A configuration and the file it came from; no file for the defaults.
#[derive(Debug)]
pub struct Configured<C> {
    pub path: Option<PathBuf>,
    pub config: Arc<C>,
}

impl<C> Clone for Configured<C> {
    fn clone(&self) -> Configured<C> {
        Configured { path: self.path.clone(), config: self.config.clone() }
    }
}

impl<C: Default> Configured<C> {
    /// The defaults, from no file.
    pub fn defaults() -> Configured<C> {
        Configured { path: None, config: Arc::new(C::default()) }
    }
}

/// Which configuration applies to the files of a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigChoice {
    /// The nearest configuration file above each file, or the defaults.
    Nearest,
    /// This file, for every file.
    File(PathBuf),
    /// The defaults, for every file.
    Defaults,
}

impl ConfigChoice {
    /// The choice of the command line options `--config FILE` and
    /// `--no-config`; `--no-config` wins.
    pub fn from_options(config: Option<&Path>, no_config: bool) -> ConfigChoice {
        match config {
            _ if no_config => ConfigChoice::Defaults,
            Some(path) => ConfigChoice::File(path.to_path_buf()),
            None => ConfigChoice::Nearest,
        }
    }
}

/// The configurations of a run, each file read once.
pub struct ConfigCache<C> {
    lookup: Lookup<C>,
}

enum Lookup<C> {
    Fixed(Configured<C>),
    Nearest { locator: ConfigLocator, read: HashMap<PathBuf, Arc<C>> },
}

impl<C: ConfigFile> ConfigCache<C> {
    /// The cache of [`ConfigChoice::Nearest`], which reads nothing up
    /// front.
    pub fn nearest() -> ConfigCache<C> {
        ConfigCache { lookup: Lookup::Nearest { locator: ConfigLocator::new(C::FILE_NAMES), read: HashMap::new() } }
    }

    /// The cache for `choice`; a named file is read at once, so a broken one
    /// stops the run before any work.
    pub fn new(choice: ConfigChoice) -> Result<ConfigCache<C>, ConfigError> {
        let lookup = match choice {
            ConfigChoice::Nearest => return Ok(ConfigCache::nearest()),
            ConfigChoice::File(path) => {
                let config = Arc::new(C::read(&path)?);
                Lookup::Fixed(Configured { path: Some(path), config })
            }
            ConfigChoice::Defaults => Lookup::Fixed(Configured::defaults()),
        };
        Ok(ConfigCache { lookup })
    }

    /// The configuration of `file`: with [`ConfigChoice::Nearest`], from
    /// the first of the file names in the file's directory or the closest
    /// directory above it. The path is made absolute and normalized as it
    /// is spelled (`.` dropped, `..` removing the component before it);
    /// symbolic links are not resolved.
    pub fn for_file(&mut self, file: &Path) -> Result<Configured<C>, ConfigError> {
        match &mut self.lookup {
            Lookup::Fixed(configured) => Ok(configured.clone()),
            Lookup::Nearest { locator, read } => {
                let found = locator.locate_for_file(file);
                configured_from(found, read)
            }
        }
    }

    /// The configuration of the files directly in `directory`, normalized
    /// like the path of [`ConfigCache::for_file`].
    pub fn for_directory(&mut self, directory: &Path) -> Result<Configured<C>, ConfigError> {
        match &mut self.lookup {
            Lookup::Fixed(configured) => Ok(configured.clone()),
            Lookup::Nearest { locator, read } => {
                let found = locator.locate_from(directory);
                configured_from(found, read)
            }
        }
    }
}

/// The configuration in the file `found`, read unless `read` holds it
/// already; the defaults without a file.
fn configured_from<C: ConfigFile>(
    found: Option<PathBuf>,
    read: &mut HashMap<PathBuf, Arc<C>>,
) -> Result<Configured<C>, ConfigError> {
    let Some(path) = found else { return Ok(Configured::defaults()) };
    if let Some(config) = read.get(&path) {
        return Ok(Configured { path: Some(path), config: config.clone() });
    }
    let config = Arc::new(C::read(&path)?);
    read.insert(path.clone(), config.clone());
    Ok(Configured { path: Some(path), config })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        /// How often [`Probe::read`] ran on this test's thread.
        static READS: Cell<usize> = const { Cell::new(0) };
    }

    /// A configuration that is the text of its file.
    #[derive(Debug, Default, PartialEq, Eq)]
    struct Probe(String);

    impl ConfigFile for Probe {
        const FILE_NAMES: &'static [&'static str] = &[".probe.toml"];

        fn read(path: &Path) -> Result<Probe, ConfigError> {
            READS.set(READS.get() + 1);
            std::fs::read_to_string(path)
                .map(Probe)
                .map_err(|source| ConfigError::Io { path: path.to_path_buf(), source })
        }
    }

    fn tree() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("a/b")).unwrap();
        std::fs::write(root.path().join(".probe.toml"), "root").unwrap();
        std::fs::write(root.path().join("a/.probe.toml"), "a").unwrap();
        std::fs::write(root.path().join("other.toml"), "other").unwrap();
        root
    }

    #[test]
    fn no_config_wins_over_a_named_file() {
        let file = Path::new("x.toml");
        assert_eq!(ConfigChoice::from_options(Some(file), true), ConfigChoice::Defaults);
        assert_eq!(ConfigChoice::from_options(Some(file), false), ConfigChoice::File(file.to_path_buf()));
        assert_eq!(ConfigChoice::from_options(None, false), ConfigChoice::Nearest);
        assert_eq!(ConfigChoice::from_options(None, true), ConfigChoice::Defaults);
    }

    #[test]
    fn a_named_file_or_the_defaults_apply_everywhere() {
        let root = tree();
        let mut named = ConfigCache::<Probe>::new(ConfigChoice::File(root.path().join("other.toml"))).unwrap();
        let configured = named.for_file(&root.path().join("a/b/x.sp")).unwrap();
        assert_eq!(
            (configured.path, &*configured.config),
            (Some(root.path().join("other.toml")), &Probe("other".into()))
        );
        let mut defaults = ConfigCache::<Probe>::new(ConfigChoice::Defaults).unwrap();
        let configured = defaults.for_file(&root.path().join("a/b/x.sp")).unwrap();
        assert_eq!((configured.path, &*configured.config), (None, &Probe::default()));
        let missing = ConfigCache::<Probe>::new(ConfigChoice::File(root.path().join("missing.toml")));
        assert!(matches!(missing, Err(ConfigError::Io { .. })));
    }

    #[test]
    fn the_nearest_file_is_read_once() {
        let root = tree();
        let mut cache = ConfigCache::<Probe>::new(ConfigChoice::Nearest).unwrap();
        let first = cache.for_file(&root.path().join("a/b/x.sp")).unwrap();
        let second = cache.for_file(&root.path().join("a/y.sp")).unwrap();
        assert_eq!(READS.get(), 1);
        assert!(Arc::ptr_eq(&first.config, &second.config));
        assert_eq!(first.path, Some(root.path().join("a/.probe.toml")));
        assert_eq!(cache.for_file(&root.path().join("z.sp")).unwrap().config.0, "root");
    }

    #[test]
    fn a_directory_is_configured_like_its_files() {
        let root = tree();
        let mut cache = ConfigCache::<Probe>::new(ConfigChoice::Nearest).unwrap();
        assert_eq!(cache.for_directory(&root.path().join("a/b")).unwrap().config.0, "a");
        assert_eq!(cache.for_directory(&root.path().join("a")).unwrap().config.0, "a");
        assert_eq!(cache.for_directory(root.path()).unwrap().config.0, "root");
    }
}
