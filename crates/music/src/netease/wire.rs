//! NetEase's answers and their conversion to the crate's models. The shapes follow Mineral's
//! NetEase channel (MIT, © 2025 wanger). NetEase answers null for many fields a client would
//! take for granted, an artist list of `[null]` included, so every list and name is read
//! leniently rather than failing the whole batch.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Deserializer};

use crate::{
    Album, Artist, ArtistRef, Encoding, Playlist, ReleaseType, SavedArtist, Track, UserProfile,
};

/// The bit of a song's `mark` that flags explicit lyrics.
const EXPLICIT: u64 = 1 << 20;
/// NetEase stamps release dates at midnight in China, eight hours ahead of UTC.
const CHINA: i64 = 8 * 3_600;
/// The `privacy` of a playlist only its owner can see.
const PRIVATE: u64 = 10;
/// The artwork sizes asked of NetEase's image server, a thumbnail and a hero.
const SMALL: u32 = 300;
const LARGE: u32 = 1000;
/// The file name of the placeholder NetEase serves for an artist without a picture.
const PLACEHOLDER: &str = "5639395138885805.jpg";

/// One artist as songs, albums and searches name them.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct Named {
    #[serde(default)]
    pub id: u64,
    #[serde(default, deserialize_with = "text")]
    pub name: String,
    #[serde(default, rename = "picUrl")]
    pub pic_url: Option<String>,
    #[serde(default, rename = "img1v1Url")]
    pub square_url: Option<String>,
}

/// The album a song sits on.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct AlbumRef {
    #[serde(default)]
    pub id: u64,
    #[serde(default, deserialize_with = "text")]
    pub name: String,
    #[serde(default, rename = "picUrl")]
    pub pic_url: Option<String>,
}

/// What the account may do with a song. `st` below zero means the song is gone from the
/// catalogue, and `pl` is the best bit rate the account may play, zero when it may play none,
/// which is how a song behind the VIP plan shows to an account without it.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub(crate) struct Privilege {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub st: i64,
    #[serde(default)]
    pub pl: Option<u64>,
}

/// One song, in the `ar`/`al`/`dt` shape of the detail, playlist and cloud search endpoints or
/// the `artists`/`album`/`duration` shape of the older ones.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct Song {
    pub id: u64,
    #[serde(default, deserialize_with = "text")]
    pub name: String,
    #[serde(default, alias = "artists", deserialize_with = "list")]
    pub ar: Vec<Named>,
    #[serde(default, alias = "album", deserialize_with = "nullable")]
    pub al: AlbumRef,
    #[serde(default, alias = "duration")]
    pub dt: u64,
    #[serde(default)]
    pub no: u32,
    #[serde(default, deserialize_with = "disc")]
    pub cd: u32,
    #[serde(default)]
    pub mark: u64,
    #[serde(default)]
    pub pop: f64,
    #[serde(default)]
    pub privilege: Option<Privilege>,
}

/// An album as its detail, a search or an artist's discography lists it.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct AlbumInfo {
    pub id: u64,
    #[serde(default, deserialize_with = "text")]
    pub name: String,
    #[serde(default, deserialize_with = "nullable")]
    pub artist: Named,
    #[serde(default, deserialize_with = "list")]
    pub artists: Vec<Named>,
    #[serde(default, rename = "picUrl")]
    pub pic_url: Option<String>,
    #[serde(default, rename = "publishTime")]
    pub publish_time: i64,
    #[serde(default)]
    pub size: u32,
    #[serde(default, deserialize_with = "text")]
    pub company: String,
    #[serde(default, rename = "type", deserialize_with = "text")]
    pub kind: String,
    #[serde(default, rename = "subTime")]
    pub sub_time: Option<i64>,
}

/// The owner of a playlist.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct Creator {
    #[serde(default, rename = "userId")]
    pub user_id: u64,
    #[serde(default, deserialize_with = "text")]
    pub nickname: String,
}

/// One entry of a playlist's full track order, with when it was added in milliseconds.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub(crate) struct Entry {
    pub id: u64,
    #[serde(default)]
    pub at: Option<i64>,
}

/// A playlist as the account's list, a search or its own detail gives it. Only the detail
/// carries `trackIds`, the whole order, and only its first stretch of `tracks`.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct PlaylistInfo {
    pub id: u64,
    #[serde(default, deserialize_with = "text")]
    pub name: String,
    #[serde(default, rename = "coverImgUrl")]
    pub cover: Option<String>,
    #[serde(default, rename = "trackCount")]
    pub track_count: u32,
    #[serde(default, deserialize_with = "nullable")]
    pub creator: Creator,
    #[serde(default, rename = "userId")]
    pub user_id: u64,
    #[serde(default)]
    pub privacy: u64,
    #[serde(default, rename = "updateTime")]
    pub update_time: Option<i64>,
    #[serde(default, rename = "trackIds", deserialize_with = "list")]
    pub track_ids: Vec<Entry>,
    #[serde(default, deserialize_with = "list")]
    pub tracks: Vec<Song>,
}

/// One stream `song/enhance/player/url` hands out. A `code` other than 200 means there is no
/// stream for this account, and a `freeTrialInfo` means the url is only a preview cut.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct SongUrl {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub br: u32,
    #[serde(default, rename = "type")]
    pub format: Option<String>,
    #[serde(default)]
    pub code: Option<i64>,
    #[serde(default, rename = "freeTrialInfo")]
    pub free_trial: Option<serde_json::Value>,
}

/// The signed-in account from `nuser/account/get`. An anonymous or expired session answers with
/// no profile.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct Account {
    #[serde(default)]
    pub profile: Option<Profile>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct Profile {
    #[serde(rename = "userId")]
    pub user_id: u64,
    #[serde(default, deserialize_with = "text")]
    pub nickname: String,
    #[serde(default, rename = "avatarUrl")]
    pub avatar_url: Option<String>,
}

/// The parts of an artist page `v1/artist/{id}` answers with.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct ArtistDetail {
    #[serde(default)]
    pub artist: ArtistInfo,
    #[serde(default, rename = "hotSongs", deserialize_with = "list")]
    pub hot_songs: Vec<Song>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct ArtistInfo {
    #[serde(default, deserialize_with = "text")]
    pub name: String,
    #[serde(default, rename = "picUrl")]
    pub pic_url: Option<String>,
    #[serde(default, rename = "img1v1Url")]
    pub square_url: Option<String>,
    #[serde(default, rename = "briefDesc", deserialize_with = "text")]
    pub brief: String,
}

impl Song {
    /// Whether this account can hear the whole song. A song without a privilege block is taken
    /// as playable, and the stream request has the last word.
    pub fn playable(&self) -> bool {
        self.privilege
            .is_none_or(|privilege| privilege.st >= 0 && privilege.pl != Some(0))
    }

    /// The song as a track, with `added_at` in seconds when a playlist says when it went in.
    pub fn track(&self, added_at: Option<i64>) -> Track {
        let (artists, artist_refs) = credits(&self.ar);
        Track {
            id: Some(self.id.to_string()),
            name: self.name.clone(),
            playable: self.playable(),
            artists,
            artist_refs,
            album: self.al.name.clone(),
            album_id: (self.al.id != 0).then(|| self.al.id.to_string()),
            cover: image(self.al.pic_url.as_deref(), SMALL),
            duration: Duration::from_millis(self.dt),
            added_at,
            added_by: None,
            playcount: None,
            popularity: self.pop.clamp(0.0, 100.0) as u32,
            explicit: self.mark & EXPLICIT != 0,
            track_number: self.no,
            disc_number: self.cd.max(1),
            tags: Vec::new(),
            languages: Vec::new(),
            credits: Vec::new(),
        }
    }
}

impl AlbumInfo {
    pub fn album(&self) -> Album {
        let named: Vec<Named> = match self.artists.is_empty() {
            true => [self.artist.clone()]
                .into_iter()
                .filter(|artist| !artist.name.is_empty())
                .collect(),
            false => self.artists.clone(),
        };
        let (artists, artist_refs) = credits(&named);
        let (year, release_date) = released(self.publish_time);
        Album {
            id: self.id.to_string(),
            name: self.name.clone(),
            artists,
            artist_refs,
            cover: image(self.pic_url.as_deref(), SMALL),
            cover_large: image(self.pic_url.as_deref(), LARGE),
            release_type: release_type(&self.kind),
            year,
            track_count: self.size,
            release_date,
            label: self.company.clone(),
            copyrights: Vec::new(),
            added_at: self.sub_time.map(|at| at / 1000),
        }
    }
}

impl PlaylistInfo {
    /// The playlist as the account `user_id` sees it, which decides whether it counts as owned.
    pub fn playlist(&self, user_id: &str) -> Playlist {
        let owner_id = match self.creator.user_id {
            0 => self.user_id,
            id => id,
        };
        let owner_id = match owner_id {
            0 => String::new(),
            id => id.to_string(),
        };
        Playlist {
            id: self.id.to_string(),
            name: self.name.clone(),
            owner: self.creator.nickname.clone(),
            owned: !owner_id.is_empty() && owner_id == user_id,
            owner_id,
            collaborative: false,
            blend: false,
            public: self.privacy != PRIVATE,
            cover: image(self.cover.as_deref(), SMALL),
            track_count: self.track_count,
            modified_at: self.update_time.map(|at| at / 1000),
        }
    }
}

impl ArtistInfo {
    pub fn picture(&self, size: u32) -> Option<String> {
        image(self.pic_url.as_deref(), size).or_else(|| portrait(self.square_url.as_deref(), size))
    }
}

impl Named {
    /// The artist as a library entry, with when it was followed when the list says.
    pub fn saved(&self) -> SavedArtist {
        SavedArtist {
            id: self.id.to_string(),
            name: self.name.clone(),
            cover: image(self.pic_url.as_deref(), SMALL)
                .or_else(|| portrait(self.square_url.as_deref(), SMALL)),
            added_at: None,
        }
    }
}

impl ArtistDetail {
    /// The artist page, with `albums` as the discography to show under it.
    pub fn artist(&self, albums: Vec<Album>) -> Artist {
        Artist {
            name: self.artist.name.clone(),
            cover_large: self.artist.picture(LARGE),
            biography: (!self.artist.brief.trim().is_empty())
                .then(|| self.artist.brief.trim().to_owned()),
            monthly_listeners: None,
            top_tracks: self.hot_songs.iter().map(|song| song.track(None)).collect(),
            albums,
        }
    }
}

impl SongUrl {
    /// The stream url when the account may hear the whole song through it.
    pub fn whole(&self) -> Option<&str> {
        if self.code.is_some_and(|code| code != 200) || self.free_trial.is_some() {
            return None;
        }
        self.url.as_deref().filter(|url| !url.is_empty())
    }

    /// What the stream is encoded as, from its format and bit rate.
    pub fn encoding(&self) -> Option<Encoding> {
        let kbps = self.br / 1000;
        match self.format.as_deref()?.to_ascii_lowercase().as_str() {
            // Above CD's 1411 kbps the depth and rate are not said, so none are guessed.
            "flac" if kbps > 1_500 => None,
            "flac" => Some(Encoding::Flac {
                depth: 16,
                rate: 44_100,
            }),
            "mp3" => Some(Encoding::Mp3 { kbps }),
            "aac" | "m4a" => Some(Encoding::Aac { kbps }),
            _ => Some(Encoding::Other),
        }
    }
}

impl Profile {
    pub fn profile(&self) -> UserProfile {
        UserProfile {
            id: self.user_id.to_string(),
            display_name: match self.nickname.is_empty() {
                true => "NetEase Cloud Music".to_owned(),
                false => self.nickname.clone(),
            },
            avatar: image(self.avatar_url.as_deref(), SMALL),
        }
    }
}

/// Puts each song's privilege beside it when the answer lists them apart, matched by id. A
/// song that already carries its own keeps it.
pub(crate) fn merge(songs: &mut [Song], privileges: Vec<Privilege>) {
    let mut by_id: HashMap<u64, Privilege> = privileges
        .into_iter()
        .map(|privilege| (privilege.id, privilege))
        .collect();
    for song in songs {
        if song.privilege.is_none() {
            song.privilege = by_id.remove(&song.id);
        }
    }
}

/// A NetEase artwork url at `size` pixels square, over https. NetEase's image server scales
/// on request through the `param` query.
pub(crate) fn image(url: Option<&str>, size: u32) -> Option<String> {
    let url = url?.trim();
    if url.is_empty() {
        return None;
    }
    let url = match url.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None => url.to_owned(),
    };
    let bare = url.split('?').next().unwrap_or(&url);
    Some(format!("{bare}?param={size}y{size}"))
}

/// An artist's square picture, except the grey placeholder NetEase hands out for an artist
/// with none, which is better left to the app's own.
fn portrait(url: Option<&str>, size: u32) -> Option<String> {
    let url = url.filter(|url| !url.contains(PLACEHOLDER))?;
    image(Some(url), size)
}

/// The artists' names joined for display, and each one as a link where NetEase gave an id.
fn credits(named: &[Named]) -> (String, Vec<ArtistRef>) {
    let named: Vec<&Named> = named
        .iter()
        .filter(|artist| !artist.name.is_empty())
        .collect();
    let names = named
        .iter()
        .map(|artist| artist.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let refs = named
        .iter()
        .map(|artist| ArtistRef {
            name: artist.name.clone(),
            id: (artist.id != 0).then(|| artist.id.to_string()),
        })
        .collect();
    (names, refs)
}

/// The year and `YYYY-MM-DD` date of a release from NetEase's millisecond stamp, read on
/// China's calendar. Zero and an empty date when there is no stamp.
fn released(publish_ms: i64) -> (i32, String) {
    if publish_ms <= 0 {
        return (0, String::new());
    }
    let Ok(at) = time::OffsetDateTime::from_unix_timestamp(publish_ms / 1000 + CHINA) else {
        return (0, String::new());
    };
    let date = at.date();
    (
        date.year(),
        format!(
            "{:04}-{:02}-{:02}",
            date.year(),
            u8::from(date.month()),
            date.day()
        ),
    )
}

/// What kind of release an album's `type` names. NetEase writes it in English or Chinese.
fn release_type(kind: &str) -> ReleaseType {
    match kind.trim() {
        "Single" | "单曲" => ReleaseType::Single,
        "EP" | "EP/Single" => ReleaseType::Ep,
        "合集" | "精选集" | "Compilation" => ReleaseType::Compilation,
        _ => ReleaseType::Album,
    }
}

/// A string that may be null, read as empty.
fn text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

/// A value that may be null, read as its default.
fn nullable<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// A list that may be null or hold nulls, read without them.
fn list<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let items = Option::<Vec<Option<T>>>::deserialize(deserializer)?.unwrap_or_default();
    Ok(items.into_iter().flatten().collect())
}

/// A disc number, which NetEase writes as a string such as `"01"` or `"1/2"`, or as a number.
fn disc<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::Number(number)) => number.as_u64().unwrap_or(0) as u32,
        Some(serde_json::Value::String(text)) => text
            .split('/')
            .next()
            .and_then(|first| first.trim().parse().ok())
            .unwrap_or(0),
        _ => 0,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_dead_song_with_nulls_still_reads() {
        let songs: Vec<Song> = serde_json::from_value(json!([{
            "id": 1,
            "name": "张洲",
            "ar": [null],
            "al": { "id": 0, "name": null, "picUrl": "http://p4.music.126.net/x.jpg" },
            "dt": 0
        }]))
        .expect("nulls are tolerated");
        let track = songs[0].track(None);
        assert_eq!(track.artists, "");
        assert!(track.artist_refs.is_empty());
        assert_eq!(track.album, "");
        assert_eq!(track.album_id, None);
        assert_eq!(
            track.cover.as_deref(),
            Some("https://p4.music.126.net/x.jpg?param=300y300")
        );
    }

    #[test]
    fn both_song_shapes_read_alike() {
        let new: Song = serde_json::from_value(json!({
            "id": 7, "name": "詩超絆", "ar": [{ "id": 1, "name": "MyGO!!!!!" }],
            "al": { "id": 3, "name": "迷跡波" }, "dt": 233_000, "no": 4, "cd": "01",
            "mark": 1_048_576
        }))
        .expect("the detail shape reads");
        let old: Song = serde_json::from_value(json!({
            "id": 7, "name": "詩超絆", "artists": [{ "id": 1, "name": "MyGO!!!!!" }],
            "album": { "id": 3, "name": "迷跡波" }, "duration": 233_000, "no": 4
        }))
        .expect("the search shape reads");
        let (new, old) = (new.track(None), old.track(None));
        assert_eq!(new.artists, old.artists);
        assert_eq!(new.album_id.as_deref(), Some("3"));
        assert_eq!(new.duration, Duration::from_secs(233));
        assert_eq!(new.disc_number, 1);
        assert!(new.explicit);
        assert!(!old.explicit);
    }

    #[test]
    fn privileges_decide_what_plays() {
        let mut songs: Vec<Song> = serde_json::from_value(json!([
            { "id": 1, "name": "gone", "al": { "id": 10 } },
            { "id": 2, "name": "vip", "al": { "id": 11 } },
            { "id": 3, "name": "free", "al": { "id": 12 } },
            { "id": 4, "name": "unknown", "al": { "id": 13 } }
        ]))
        .expect("songs read");
        let privileges: Vec<Privilege> = serde_json::from_value(json!([
            { "id": 1, "st": -200, "pl": 0 },
            { "id": 2, "st": 0, "pl": 0 },
            { "id": 3, "st": 0, "pl": 128_000 }
        ]))
        .expect("privileges read");
        merge(&mut songs, privileges);
        let playable: Vec<bool> = songs.iter().map(Song::playable).collect();
        assert_eq!(playable, vec![false, false, true, true]);
    }

    #[test]
    fn a_preview_or_refused_stream_is_not_whole() {
        let urls: Vec<SongUrl> = serde_json::from_value(json!([
            { "id": 1, "url": null, "code": 404 },
            { "id": 2, "url": "https://m701.music.126.net/a.mp3", "code": 200,
              "freeTrialInfo": { "start": 0, "end": 30 } },
            { "id": 3, "url": "https://m701.music.126.net/b.flac", "code": 200,
              "br": 950_000, "type": "flac", "freeTrialInfo": null }
        ]))
        .expect("urls read");
        assert_eq!(urls[0].whole(), None);
        assert_eq!(urls[1].whole(), None);
        assert_eq!(urls[2].whole(), Some("https://m701.music.126.net/b.flac"));
        assert_eq!(
            urls[2].encoding(),
            Some(Encoding::Flac {
                depth: 16,
                rate: 44_100
            })
        );
    }

    #[test]
    fn an_album_reads_its_release() {
        let album: AlbumInfo = serde_json::from_value(json!({
            "id": 9, "name": "乐与怒", "picUrl": "https://p1.music.126.net/a.jpg",
            "publishTime": 736_185_600_000_i64, "size": 10, "company": "滚石",
            "type": "EP/Single", "artist": { "id": 11127, "name": "Beyond" }, "artists": null
        }))
        .expect("the album reads");
        let album = album.album();
        assert_eq!(album.artists, "Beyond");
        assert_eq!(album.year, 1993);
        assert_eq!(album.release_date, "1993-05-01");
        assert_eq!(album.release_type, ReleaseType::Ep);
        assert_eq!(album.label, "滚石");
        assert_eq!(
            album.cover_large.as_deref(),
            Some("https://p1.music.126.net/a.jpg?param=1000y1000")
        );
    }

    #[test]
    fn a_playlist_knows_its_owner() {
        let playlist: PlaylistInfo = serde_json::from_value(json!({
            "id": 5, "name": "我喜欢的音乐", "trackCount": 557, "privacy": 10,
            "creator": { "userId": 42, "nickname": "me" }, "tracks": null, "trackIds": null
        }))
        .expect("the playlist reads");
        let mine = playlist.playlist("42");
        assert!(mine.owned);
        assert!(!mine.public);
        assert_eq!(mine.track_count, 557);
        assert!(!playlist.playlist("7").owned);
        assert!(playlist.tracks.is_empty());
    }
}
