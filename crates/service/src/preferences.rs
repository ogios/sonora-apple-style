//! Serialized preference values persisted by the server without applying presentation behavior.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A serialized choice carried by settings and IPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ThemeKind {
    System,
    Dark,
    Light,
}

impl ThemeKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
    pub fn from_id(id: &str) -> Self {
        match id {
            "system" => Self::System,
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }
}

/// A serialized choice carried by settings and IPC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Pace {
    Slow,
    #[default]
    Base,
    Quick,
}

impl Pace {
    pub fn id(self) -> &'static str {
        match self {
            Self::Slow => "slow",
            Self::Base => "base",
            Self::Quick => "quick",
        }
    }
    pub fn from_id(id: &str) -> Self {
        match id {
            "slow" => Self::Slow,
            "quick" => Self::Quick,
            _ => Self::Base,
        }
    }
}

/// A serialized choice carried by settings and IPC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Stillness {
    #[default]
    System,
    Always,
    Never,
}

impl Stillness {
    pub fn id(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Always => "always",
            Self::Never => "never",
        }
    }
    pub fn from_id(id: &str) -> Self {
        match id {
            "always" => Self::Always,
            "never" => Self::Never,
            _ => Self::System,
        }
    }
}

/// A serialized choice carried by settings and IPC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Saver {
    #[default]
    Off,
    Light,
    Medium,
    Strong,
}

impl Saver {
    pub fn id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Light => "light",
            Self::Medium => "medium",
            Self::Strong => "strong",
        }
    }
    pub fn from_id(id: &str) -> Self {
        match id {
            "light" => Self::Light,
            "medium" => Self::Medium,
            "strong" => Self::Strong,
            _ => Self::Off,
        }
    }
}

/// A serialized choice carried by settings and IPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Rounding {
    Square,
    Subtle,
    Rounded,
    Round,
}

impl Rounding {
    pub fn id(self) -> &'static str {
        match self {
            Self::Square => "square",
            Self::Subtle => "subtle",
            Self::Rounded => "rounded",
            Self::Round => "round",
        }
    }
    pub fn from_id(id: &str) -> Self {
        match id {
            "square" => Self::Square,
            "rounded" => Self::Rounded,
            "round" => Self::Round,
            _ => Self::Subtle,
        }
    }
}

/// A serialized choice carried by settings and IPC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    List,
    #[serde(rename = "cards", alias = "grid")]
    Grid,
}

impl Mode {}

/// A serialized choice carried by settings and IPC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VisualizerStyle {
    None,
    Bars,
    Wave,
    #[default]
    Both,
}

impl VisualizerStyle {
    pub fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bars => "bars",
            Self::Wave => "wave",
            Self::Both => "both",
        }
    }
    pub fn from_id(id: &str) -> Self {
        match id {
            "none" => Self::None,
            "bars" => Self::Bars,
            "wave" => Self::Wave,
            _ => Self::default(),
        }
    }
    pub fn shown(self) -> bool {
        self != Self::None
    }
}

/// A serialized choice carried by settings and IPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PinKind {
    Album,
    Artist,
    Playlist,
    Song,
}

impl PinKind {}

/// A serialized choice carried by settings and IPC.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
    Ascending,
    Descending,
}

/// Structured preference data stored without creating UI elements.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub order: Vec<String>,
    pub hidden: Vec<String>,
    pub widths: HashMap<String, f32>,
}

/// Structured preference data stored without creating UI elements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sorting {
    pub column: String,
    pub order: Sort,
}

/// Structured preference data stored without creating UI elements.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pin {
    pub kind: PinKind,
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
}

impl Pin {
    pub fn new(kind: PinKind, id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
            title: title.into(),
            cover: None,
        }
    }
    pub fn cover(mut self, cover: Option<String>) -> Self {
        self.cover = cover;
        self
    }
    pub fn same(&self, other: &Self) -> bool {
        self.kind == other.kind && self.id == other.id
    }
}

/// Opaque theme values; only the GUI interprets color and layout tokens.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ThemeOverrides {
    #[serde(flatten)]
    values: serde_json::Map<String, serde_json::Value>,
}

pub const MIN_FONT: f32 = 10.;
pub const MAX_FONT: f32 = 24.;
pub const MAX_TRANSPARENCY: f32 = 1.;
pub const BACKDROP_TRANSPARENCY: f32 = 0.15;
pub const MIN_LYRICS_SCALE: f32 = 0.6;
pub const MAX_LYRICS_SCALE: f32 = 2.;
