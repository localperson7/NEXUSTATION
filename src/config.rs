use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::highscores::HighScoreStore;
use crate::theme::ThemeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Easy, Self::Normal, Self::Hard];

    pub fn name(self) -> &'static str {
        match self {
            Self::Easy => "Chill",
            Self::Normal => "Arcade",
            Self::Hard => "Turbo",
        }
    }

    pub fn badge(self) -> &'static str {
        match self {
            Self::Easy => "EASY // CHILL",
            Self::Normal => "NORMAL // ARCADE",
            Self::Hard => "HARD // TURBO",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Easy => Self::Normal,
            Self::Normal => Self::Hard,
            Self::Hard => Self::Easy,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Easy => Self::Hard,
            Self::Normal => Self::Easy,
            Self::Hard => Self::Normal,
        }
    }

    pub fn score_multiplier(self) -> u32 {
        match self {
            Self::Easy => 1,
            Self::Normal => 2,
            Self::Hard => 3,
        }
    }
}

/// Cross-platform local user data directory layout for NEXUSTATION.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Discovers the primary NEXUSTATION user data directory.
    ///
    /// Precedence:
    /// 1. `NEXUSTATION_DATA_DIR` (if set and non-empty)
    /// 2. Platform-standard user data dir:
    ///    - Linux/Unix: `$XDG_DATA_HOME/nexustation` or `$HOME/.local/share/nexustation`
    ///    - macOS: `$HOME/Library/Application Support/NEXUSTATION`
    ///    - Windows: `%LOCALAPPDATA%\NEXUSTATION` or `%APPDATA%\NEXUSTATION`
    /// 3. Fallback: `./.nexustation-data`
    pub fn discover() -> Self {
        if let Some(override_dir) = env::var_os("NEXUSTATION_DATA_DIR")
            && !override_dir.is_empty()
        {
            return Self::with_root(override_dir);
        }

        Self::platform_dir("nexustation", "NEXUSTATION", ".nexustation-data")
    }

    /// Discovers the legacy CrabStation v0.1 user data directory for non-destructive migration.
    pub fn discover_legacy() -> Option<PathBuf> {
        if let Some(override_dir) = env::var_os("CRABSTATION_DATA_DIR")
            && !override_dir.is_empty()
        {
            return Some(PathBuf::from(override_dir));
        }

        Some(Self::platform_dir("crabstation", "CrabStation", ".crabstation-data").root)
    }

    fn platform_dir(unix_slug: &str, desktop_name: &str, fallback: &str) -> Self {
        #[cfg(windows)]
        {
            let _ = unix_slug;
            if let Some(appdata) = env::var_os("LOCALAPPDATA").or_else(|| env::var_os("APPDATA"))
                && !appdata.is_empty()
            {
                return Self::with_root(PathBuf::from(appdata).join(desktop_name));
            }
        }

        #[cfg(target_os = "macos")]
        {
            let _ = unix_slug;
            if let Some(home) = env::var_os("HOME")
                && !home.is_empty()
            {
                return Self::with_root(
                    PathBuf::from(home)
                        .join("Library")
                        .join("Application Support")
                        .join(desktop_name),
                );
            }
        }

        #[cfg(not(any(windows, target_os = "macos")))]
        {
            let _ = desktop_name;
            if let Some(xdg) = env::var_os("XDG_DATA_HOME")
                && !xdg.is_empty()
            {
                return Self::with_root(PathBuf::from(xdg).join(unix_slug));
            }
            if let Some(home) = env::var_os("HOME")
                && !home.is_empty()
            {
                return Self::with_root(
                    PathBuf::from(home)
                        .join(".local")
                        .join("share")
                        .join(unix_slug),
                );
            }
        }

        Self::with_root(PathBuf::from(fallback))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn settings_file(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn highscores_file(&self) -> PathBuf {
        self.root.join("highscores.json")
    }

    pub fn ensure_dir(&self) -> io::Result<()> {
        fs::create_dir_all(&self.root)
    }
}

/// Atomically writes `bytes` to `target` via a sibling temporary file, preserving permissions.
pub fn atomic_write_bytes(
    target: &Path,
    bytes: &[u8],
    explicit_perms: Option<fs::Permissions>,
) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    let preserved_perms =
        explicit_perms.or_else(|| fs::metadata(target).ok().map(|meta| meta.permissions()));

    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("data.json");
    let tmp_path = target.with_file_name(format!(".{file_name}.tmp.{}", std::process::id()));

    if let Err(err) = fs::write(&tmp_path, bytes) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }

    if let Some(perms) = preserved_perms {
        let _ = fs::set_permissions(&tmp_path, perms);
    }

    if let Err(err) = fs::rename(&tmp_path, target) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }

    Ok(())
}

/// Summary of non-destructive legacy data migration from CrabStation to NEXUSTATION.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub migrated_settings: bool,
    pub migrated_highscores: bool,
    pub skipped_existing: usize,
    pub skipped_corrupt: usize,
}

impl MigrationReport {
    pub fn any_migrated(&self) -> bool {
        self.migrated_settings || self.migrated_highscores
    }
}

/// Safely migrates settings and high scores from a legacy CrabStation directory into `target`.
///
/// Guarantees:
/// - Never deletes or modifies files in `legacy_dir`.
/// - Never overwrites files that already exist in `target`.
/// - Validates JSON before writing so corrupt legacy files are skipped gracefully.
/// - Preserves source file permissions and uses atomic temp-file + rename writes.
pub fn migrate_legacy_data(legacy_dir: &Path, target: &AppPaths) -> MigrationReport {
    let mut report = MigrationReport::default();

    if legacy_dir == target.root() || !legacy_dir.is_dir() {
        return report;
    }

    // 1. Migrate settings.json if absent in target and valid in legacy_dir
    let legacy_settings = legacy_dir.join("settings.json");
    let target_settings = target.settings_file();
    if target_settings.exists() {
        report.skipped_existing += 1;
    } else if legacy_settings.is_file() {
        match fs::read_to_string(&legacy_settings) {
            Ok(raw) if serde_json::from_str::<Settings>(&raw).is_ok() => {
                let perms = fs::metadata(&legacy_settings).ok().map(|m| m.permissions());
                if atomic_write_bytes(&target_settings, raw.as_bytes(), perms).is_ok() {
                    report.migrated_settings = true;
                }
            }
            _ => {
                report.skipped_corrupt += 1;
            }
        }
    }

    // 2. Migrate highscores.json if absent in target and valid in legacy_dir
    let legacy_scores = legacy_dir.join("highscores.json");
    let target_scores = target.highscores_file();
    if target_scores.exists() {
        report.skipped_existing += 1;
    } else if legacy_scores.is_file() {
        match fs::read_to_string(&legacy_scores) {
            Ok(raw) if serde_json::from_str::<HighScoreStore>(&raw).is_ok() => {
                let perms = fs::metadata(&legacy_scores).ok().map(|m| m.permissions());
                if atomic_write_bytes(&target_scores, raw.as_bytes(), perms).is_ok() {
                    report.migrated_highscores = true;
                }
            }
            _ => {
                report.skipped_corrupt += 1;
            }
        }
    }

    report
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeId,
    pub difficulty: Difficulty,
    pub scanlines: bool,
    pub boot_animation: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeId::NeonArcade,
            difficulty: Difficulty::Normal,
            scanlines: true,
            boot_animation: true,
        }
    }
}

impl Settings {
    pub fn load_from(paths: &AppPaths) -> Self {
        let path = paths.settings_file();
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save_to(&self, paths: &AppPaths) -> io::Result<()> {
        paths.ensure_dir()?;
        let serialized = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        atomic_write_bytes(&paths.settings_file(), serialized.as_bytes(), None)
    }
}
