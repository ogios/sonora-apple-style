use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use music::{Album, Playlist, SavedArtist, Shape, Track};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::library::LibraryPart;

/// Which of a shelf's two lists a snapshot holds. `Listed` is what the page shows; `Starred` is
/// the favorites a catalog shelf keeps beside it, which the hearts and the favorites filter
/// read. A `Saved` shelf writes only `Listed`, since there the two are one list.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Listed,
    Starred,
}

impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Listed => "",
            Self::Starred => "starred-",
        }
    }
}

/// One list of one part as its last load left it. `total` is how many rows there were, which is
/// what the page header shows until this run's own rows arrive.
#[derive(Deserialize)]
struct Snapshot<T> {
    shape: Shape,
    total: usize,
    rows: Vec<T>,
}

/// The same thing on the way out, borrowing the rows it is about to write.
#[derive(Serialize)]
struct Keeping<'a, T> {
    shape: Shape,
    total: usize,
    rows: &'a [T],
}

/// What a shelf looked like at the end of its last successful load. `parts` names the parts a
/// snapshot was found for, which are the ones the next load replaces rather than appends to.
pub(crate) struct Remembered {
    pub shape: Shape,
    pub totals: HashMap<LibraryPart, usize>,
    pub parts: Vec<LibraryPart>,
    pub tracks: Vec<Track>,
    pub playlists: Vec<Playlist>,
    pub albums: Vec<Album>,
    pub artists: Vec<SavedArtist>,
    pub starred_tracks: Vec<Track>,
    pub starred_albums: Vec<Album>,
    pub starred_artists: Vec<SavedArtist>,
}

/// The library snapshots, filed under the provider's slug and the part: `apple/songs`,
/// `apple/starred-albums`, `local/artists`. A provider owns one shelf, so its slug is the whole
/// prefix and dropping it forgets that provider. Every method blocks on sqlite and json, so a
/// caller runs it off the main thread.
#[derive(Clone)]
pub(crate) struct Snapshots {
    cache: storage::Cache,
}

impl Snapshots {
    pub fn new(cache: storage::Cache) -> Self {
        Self { cache }
    }

    /// Reads back everything kept for a provider. `None` means nothing ever was.
    pub fn restore(&self, provider: &str) -> Option<Remembered> {
        let listed = Kind::Listed;
        let tracks: Option<Snapshot<Track>> = self.read(provider, listed, LibraryPart::Tracks);
        let playlists: Option<Snapshot<Playlist>> =
            self.read(provider, listed, LibraryPart::Playlists);
        let albums: Option<Snapshot<Album>> = self.read(provider, listed, LibraryPart::Albums);
        let artists: Option<Snapshot<SavedArtist>> =
            self.read(provider, listed, LibraryPart::Artists);

        let shape = [
            tracks.as_ref().map(|kept| kept.shape),
            playlists.as_ref().map(|kept| kept.shape),
            albums.as_ref().map(|kept| kept.shape),
            artists.as_ref().map(|kept| kept.shape),
        ]
        .into_iter()
        .flatten()
        .next()?;

        let starred = Kind::Starred;
        let mut remembered = Remembered {
            shape,
            totals: HashMap::new(),
            parts: Vec::new(),
            tracks: Vec::new(),
            playlists: Vec::new(),
            albums: Vec::new(),
            artists: Vec::new(),
            starred_tracks: self.rows(provider, starred, LibraryPart::Tracks),
            starred_albums: self.rows(provider, starred, LibraryPart::Albums),
            starred_artists: self.rows(provider, starred, LibraryPart::Artists),
        };
        if let Some(kept) = tracks {
            remembered.note(LibraryPart::Tracks, kept.total);
            remembered.tracks = kept.rows;
        }
        if let Some(kept) = playlists {
            remembered.note(LibraryPart::Playlists, kept.total);
            remembered.playlists = kept.rows;
        }
        if let Some(kept) = albums {
            remembered.note(LibraryPart::Albums, kept.total);
            remembered.albums = kept.rows;
        }
        if let Some(kept) = artists {
            remembered.note(LibraryPart::Artists, kept.total);
            remembered.artists = kept.rows;
        }
        Some(remembered)
    }

    /// Remembers a list whole. A track costs about 600 bytes here, so a library of tens of
    /// thousands is a cache file of tens of megabytes, written off the main thread.
    pub fn keep<T: Serialize>(
        &self,
        provider: &str,
        kind: Kind,
        part: LibraryPart,
        shape: Shape,
        rows: &[T],
    ) {
        let kept = Keeping {
            shape,
            total: rows.len(),
            rows,
        };
        let recorded = serde_json::to_string(&kept)
            .map_err(anyhow::Error::from)
            .and_then(|value| {
                self.cache
                    .write(&key(provider, kind, part), &value, stamp())
            });
        if let Err(error) = recorded {
            log::warn!(
                "library: cannot record the {} snapshot: {error:#}",
                part.key()
            );
        }
    }

    /// Drops everything kept for a provider, which is what signing out of it means.
    pub fn forget(&self, provider: &str) {
        if let Err(error) = self.cache.forget(&format!("{provider}/")) {
            log::warn!("library: cannot drop the {provider} snapshots: {error:#}");
        }
    }

    /// The rows of a list, empty when none were kept.
    fn rows<T: DeserializeOwned>(&self, provider: &str, kind: Kind, part: LibraryPart) -> Vec<T> {
        self.read(provider, kind, part)
            .map(|kept| kept.rows)
            .unwrap_or_default()
    }

    /// A list as it was kept. A payload that no longer parses, because the models moved on,
    /// counts as absent and the next load overwrites it.
    fn read<T: DeserializeOwned>(
        &self,
        provider: &str,
        kind: Kind,
        part: LibraryPart,
    ) -> Option<Snapshot<T>> {
        self.cache
            .read(&key(provider, kind, part), |value| {
                serde_json::from_str::<Snapshot<T>>(value)
            })
            .inspect_err(|error| log::warn!("library: cannot read a snapshot: {error:#}"))
            .ok()??
            .inspect_err(|error| {
                log::debug!(
                    "library: the {} snapshot no longer parses: {error:#}",
                    part.key()
                );
            })
            .ok()
    }
}

impl Remembered {
    fn note(&mut self, part: LibraryPart, total: usize) {
        self.totals.insert(part, total);
        self.parts.push(part);
    }
}

fn key(provider: &str, kind: Kind, part: LibraryPart) -> String {
    format!("{provider}/{}{}", kind.prefix(), part.key())
}

fn stamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
