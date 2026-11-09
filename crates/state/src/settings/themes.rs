//! Theme files
//!
//! A theme file names its colours over the dark palette. The app bundles some from
//! `assets/themes`, the user adds their own to the themes folder, and both are read by the same
//! parser. A folder is a family and each file in it a variant, while a loose file is a family of
//! one. A user file with the id of a bundled one takes its place.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use serde::Deserialize;
use ui::{Theme, ThemeKind, ThemeOverrides};

include!(concat!(env!("OUT_DIR"), "/themes.rs"));

const FORMAT_VERSION: u32 = 1;
/// The bundled themes in the order the Appearance page lists them. A bundled file missing here
/// sorts after these by id.
const ORDER: [&str; 15] = [
    "catppuccin/latte",
    "catppuccin/frappe",
    "catppuccin/macchiato",
    "catppuccin/mocha",
    "rose-pine/main",
    "rose-pine/moon",
    "rose-pine/dawn",
    "gruvbox/dark",
    "gruvbox/dark-hard",
    "gruvbox/light",
    "gruvbox/light-hard",
    "solarized/dark",
    "solarized/light",
    "nord",
    "dracula",
];

/// The bundled themes, parsed once and sorted by `ORDER`.
static BUNDLED_THEMES: LazyLock<Vec<ThemeVariant>> = LazyLock::new(|| {
    let mut themes = BUNDLED
        .iter()
        .filter_map(|(id, bytes)| {
            let family = id.split_once('/').map_or(*id, |(family, _)| family);
            parse(bytes, id, family)
                .map(|theme| ThemeVariant {
                    id: (*id).to_owned(),
                    bundled: true,
                    ..theme
                })
                .inspect_err(|error| {
                    log::error!("settings: cannot load bundled theme {id}: {error:#}")
                })
                .ok()
        })
        .collect::<Vec<_>>();
    themes.sort_by_key(|theme| {
        let rank = ORDER.iter().position(|id| *id == theme.id);
        (rank.unwrap_or(ORDER.len()), theme.id.clone())
    });
    themes
});

/// One theme file. Its id is the path under the themes folder without the extension, such as
/// `catppuccin/mocha` or `noctalia`, and that id is what the settings store.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeVariant {
    pub id: String,
    /// The id of the family it belongs to, the folder it sits in or its own id when loose.
    pub family: String,
    /// The name inside its family, such as "Mocha". A family of one uses the whole title.
    pub name: String,
    /// The name the file gives, such as "Catppuccin Mocha".
    pub title: String,
    pub author: String,
    pub overrides: ThemeOverrides,
    /// Whether the app ships this file rather than the user's themes folder holding it.
    pub bundled: bool,
}

impl ThemeVariant {
    /// The colours this variant paints with, before rounding, font size, opacity or a cover tint
    /// are applied. Enough to draw a miniature of it.
    pub fn palette(&self) -> Theme {
        Theme::for_kind(ThemeKind::Dark).with_overrides(&self.overrides)
    }

    /// Whether the variant has a light background, which is how a family picks the variant that
    /// matches the system appearance.
    pub fn is_light(&self) -> bool {
        self.palette().background.l >= 0.5
    }
}

/// A theme as the Appearance page offers it: a name over one or more variants. Bundled families
/// come first in a fixed order, then the user's own sorted by name.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeFamily {
    pub id: String,
    /// The words every variant title starts with, or the folder name when they share none.
    pub name: String,
    /// Never empty.
    pub variants: Vec<ThemeVariant>,
    /// Whether the app ships the family. The user may still have replaced or added variants.
    pub bundled: bool,
}

impl ThemeFamily {
    pub fn variant(&self, id: &str) -> Option<&ThemeVariant> {
        self.variants.iter().find(|variant| variant.id == id)
    }
}

/// A scan of the user's themes folder and whether transient I/O failures merit another scan.
pub(super) struct Loaded {
    pub themes: Vec<ThemeVariant>,
    pub retry: bool,
}

#[derive(Deserialize)]
struct ThemeFile {
    name: String,
    author: String,
    version: u32,
    theme: ThemeOverrides,
}

/// Groups the bundled themes and the user's into families, with a user file replacing the
/// bundled one of the same id.
pub(super) fn catalog(user: &[ThemeVariant]) -> Vec<ThemeFamily> {
    let mut user = user.to_vec();
    user.sort_by(|left, right| {
        left.title
            .to_lowercase()
            .cmp(&right.title.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut families: Vec<ThemeFamily> = Vec::new();
    for theme in BUNDLED_THEMES.iter().cloned().chain(user) {
        let Some(family) = families.iter_mut().find(|family| family.id == theme.family) else {
            families.push(ThemeFamily {
                id: theme.family.clone(),
                name: String::new(),
                bundled: theme.bundled,
                variants: vec![theme],
            });
            continue;
        };
        match family
            .variants
            .iter_mut()
            .find(|variant| variant.id == theme.id)
        {
            Some(variant) => *variant = theme,
            None => family.variants.push(theme),
        }
    }

    families.iter_mut().for_each(name);
    let user = families.iter().position(|family| !family.bundled);
    if let Some(start) = user {
        families[start..].sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
    }
    families
}

/// Loads every valid theme in `directory`: loose JSON files and JSON files one folder down. A
/// file that fails to read or parse keeps its `previous` value, so a half-written save does not
/// make a theme flicker away.
pub(super) fn load(directory: &Path, previous: &[ThemeVariant]) -> Loaded {
    let kept = |retry| Loaded {
        themes: previous.to_vec(),
        retry,
    };
    match fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.file_type().is_dir() => {}
        Ok(_) => {
            log::warn!(
                "settings: themes path is not a directory: {}",
                directory.display()
            );
            return kept(false);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Loaded {
                themes: Vec::new(),
                retry: false,
            };
        }
        Err(error) => {
            log::warn!("settings: cannot inspect {}: {error}", directory.display());
            return kept(true);
        }
    }
    let Ok((files, folders)) = entries(directory) else {
        return kept(true);
    };

    let mut found = Vec::new();
    for path in files {
        let Some(id) = utf8_stem(&path) else {
            continue;
        };
        if reserved(id) {
            log::warn!(
                "settings: theme {} uses a reserved identifier",
                path.display()
            );
            continue;
        }
        found.push((path.clone(), id.to_owned(), id.to_owned()));
    }

    let mut themes = Vec::new();
    let mut retry = false;
    for folder in folders {
        let Some(family) = folder.file_name().and_then(|name| name.to_str()) else {
            log::warn!("settings: theme folder is not UTF-8: {}", folder.display());
            continue;
        };
        if reserved(family) {
            log::warn!(
                "settings: theme folder {} uses a reserved identifier",
                folder.display()
            );
            continue;
        }
        let Ok((files, _)) = entries(&folder) else {
            retry = true;
            themes.extend(
                previous
                    .iter()
                    .filter(|theme| theme.family == family)
                    .cloned(),
            );
            continue;
        };
        for path in files {
            if let Some(stem) = utf8_stem(&path) {
                found.push((path.clone(), format!("{family}/{stem}"), family.to_owned()));
            }
        }
    }

    for (path, id, family) in found {
        let earlier = || previous.iter().find(|theme| theme.id == id).cloned();
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                log::warn!("settings: cannot read theme {}: {error}", path.display());
                retry = true;
                themes.extend(earlier());
                continue;
            }
        };
        match parse(&bytes, &path.display().to_string(), &family) {
            Ok(theme) => themes.push(ThemeVariant { id, ..theme }),
            Err(error) => {
                log::warn!("settings: cannot load theme {}: {error:#}", path.display());
                themes.extend(earlier());
            }
        }
    }
    Loaded { themes, retry }
}

/// Writes the backup of a removed built-in theme into `directory` when `id` names one and no
/// theme file of that id is there yet, so whoever wore it keeps it. Returns whether it wrote.
pub(super) fn adopt_legacy(id: &str, directory: &Path) -> bool {
    let Some((_, bytes)) = LEGACY.iter().find(|(name, _)| *name == id) else {
        return false;
    };
    let path = directory.join(format!("{id}.json"));
    if fs::symlink_metadata(&path).is_ok() {
        return false;
    }
    if let Err(error) = fs::create_dir_all(directory).and_then(|()| fs::write(&path, bytes)) {
        log::warn!("settings: cannot write {}: {error}", path.display());
        return false;
    }
    log::info!("settings: moved legacy theme {id} into the themes folder");
    true
}

/// Parses one theme file. The id it returns is `family`, which a caller inside a folder
/// replaces with the full one. `origin` names the file in errors.
fn parse(bytes: &[u8], origin: &str, family: &str) -> Result<ThemeVariant> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let value: serde_json::Value =
        serde_json::from_slice(bytes).with_context(|| format!("cannot parse {origin}"))?;
    let object = value
        .as_object()
        .context("theme file must contain an object")?;
    for field in object.keys() {
        anyhow::ensure!(
            matches!(field.as_str(), "name" | "author" | "version" | "theme"),
            "unknown theme file field `{field}`"
        );
    }
    let theme = object
        .get("theme")
        .and_then(serde_json::Value::as_object)
        .context("theme must contain an object")?;
    for field in theme.keys() {
        anyhow::ensure!(
            ThemeOverrides::is_color(field),
            "`{field}` is not a theme color"
        );
    }
    let file: ThemeFile =
        serde_json::from_value(value).with_context(|| format!("cannot decode {origin}"))?;
    anyhow::ensure!(
        file.version == FORMAT_VERSION,
        "unsupported theme format version {}",
        file.version
    );
    let title = file.name.trim();
    anyhow::ensure!(!title.is_empty(), "theme name is empty");
    let author = file.author.trim();
    anyhow::ensure!(!author.is_empty(), "theme author is empty");
    if let Some(field) = file.theme.invalid_color() {
        anyhow::bail!("theme.{field} is not a valid color");
    }
    Ok(ThemeVariant {
        id: family.to_owned(),
        family: family.to_owned(),
        name: title.to_owned(),
        title: title.to_owned(),
        author: author.to_owned(),
        overrides: file.theme,
        bundled: false,
    })
}

/// The JSON files and the folders directly inside `directory`, each sorted. Symlinks are
/// skipped. Fails on the first entry that cannot be read, after logging it.
fn entries(directory: &Path) -> Result<(Vec<PathBuf>, Vec<PathBuf>), ()> {
    let failed = |error: std::io::Error| {
        log::warn!("settings: cannot read {}: {error}", directory.display());
    };
    let mut files = Vec::new();
    let mut folders = Vec::new();
    for entry in fs::read_dir(directory).map_err(failed)? {
        let entry = entry.map_err(failed)?;
        let path = entry.path();
        let kind = entry.file_type().map_err(failed)?;
        if kind.is_dir() {
            folders.push(path);
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "json")
        {
            files.push(path);
        }
    }
    files.sort();
    folders.sort();
    Ok((files, folders))
}

fn utf8_stem(path: &Path) -> Option<&str> {
    let stem = path.file_stem().and_then(|stem| stem.to_str());
    if stem.is_none() {
        log::warn!("settings: theme filename is not UTF-8: {}", path.display());
    }
    stem
}

/// Whether `id` is taken by a built-in theme, which a theme file cannot replace.
fn reserved(id: &str) -> bool {
    ThemeKind::ALL.into_iter().any(|kind| kind.id() == id)
}

/// Names a family after the words all its variant titles share and each variant after the rest
/// of its title. A family of one, or one whose titles share nothing, keeps the whole titles.
fn name(family: &mut ThemeFamily) {
    let titles = family
        .variants
        .iter()
        .map(|variant| {
            variant
                .title
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let shared = match titles.as_slice() {
        [] | [_] => 0,
        [first, rest @ ..] => (0..first.len())
            .take_while(|&index| {
                rest.iter()
                    .all(|title| title.get(index) == first.get(index))
            })
            .count(),
    };

    family.name = match (shared, titles.as_slice()) {
        (_, [only]) => only.join(" "),
        (0, _) => family.id.clone(),
        (shared, [first, ..]) => first[..shared].join(" "),
        (_, []) => family.id.clone(),
    };
    for (variant, words) in family.variants.iter_mut().zip(&titles) {
        variant.name = match words.get(shared..).filter(|rest| !rest.is_empty()) {
            Some(rest) if titles.len() > 1 => rest.join(" "),
            _ => variant.title.clone(),
        };
    }
}
