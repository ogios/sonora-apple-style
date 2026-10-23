//! NetEase Cloud Music data access over the endpoints its web player and desktop clients use,
//! with the account's `MUSIC_U` cookie. The transport, the song, playlist, album, artist,
//! like and playlist-edit endpoints and the stream fallback follow Mineral's NetEase channel
//! (MIT, © 2025 wanger, <https://github.com/10knamesmore/Mineral>). The follow, save,
//! similar-song and desktop search endpoints are the ones the NeteaseCloudMusicApi project
//! documents.

use std::collections::HashMap;
use std::fmt;
use std::io::Read as _;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::netease::auth::Credentials;
use crate::netease::{crypto, wire};
use crate::{
    Album, AlbumDetail, Artist, ArtistProfile, Encoding, MediaKind, MusicApi, Offer, Page, Pages,
    Playlist, PlaylistDetail, Quality, SavedArtist, Throttled, Track, UserProfile, distinct_covers,
};

const BASE: &str = "https://music.163.com";
/// What the web player's calls present themselves as.
const AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 Edg/124.0.0.0";
/// What the old Linux client presents itself as, for the calls forwarded through it.
const LINUX_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36";
/// The cookies the desktop client always sends beside the session.
const CLIENT_COOKIES: &str = "os=pc; appver=2.7.1.198277; __remember_me=true";

/// The answer codes NetEase uses for a session that is no longer signed in, and for a caller
/// it wants to slow down.
const SIGNED_OUT: i64 = 301;
const RATE_LIMITED: [i64; 2] = [405, 512];

/// How long a connection may take to open, how long one API call may take in all, and how long
/// a stream may go without sending anything.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const CALL_TIMEOUT: Duration = Duration::from_secs(20);
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// How many songs one `song/detail` call is asked for.
const DETAIL_BATCH: usize = 500;
/// How many tracks the first `playlist/detail` call carries in full. The rest come by id.
const PLAYLIST_HEAD: usize = 1000;
/// How many entries one page of the saved albums or artists asks for.
const LIBRARY_PAGE: usize = 100;
/// The most pages a library listing reads, a stop for an answer that never says it is done.
const LIBRARY_PAGES: usize = 100;
/// How many releases an artist page asks for.
const DISCOGRAPHY: usize = 500;
/// How many results a search asks for.
const SEARCH_LIMIT: usize = 50;
/// How many similar songs a station asks for.
const RADIO_LIMIT: usize = 50;
/// The most artists one `artist_images` call looks up.
const PORTRAIT_LIMIT: usize = 12;
/// How far ahead the paged favorites run of the page reading them.
const FAN: usize = 2;

/// The levels NetEase serves, best first. Hi-res and lossless are FLAC, the rest MP3; what an
/// account gets depends on its plan, and NetEase steps down by itself when the plan or the
/// song has nothing at the level asked for.
pub(crate) const QUALITIES: &[Offer] = &[
    Offer {
        quality: Quality::HiRes,
        encoding: Encoding::Flac {
            depth: 24,
            rate: 192_000,
        },
        up_to: true,
    },
    Offer {
        quality: Quality::Lossless,
        encoding: Encoding::Flac {
            depth: 16,
            rate: 44_100,
        },
        up_to: false,
    },
    Offer {
        quality: Quality::High,
        encoding: Encoding::Mp3 { kbps: 320 },
        up_to: false,
    },
    Offer {
        quality: Quality::Normal,
        encoding: Encoding::Mp3 { kbps: 128 },
        up_to: false,
    },
];

/// How a call is encrypted, which also decides where it is sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scheme {
    Weapi,
    Linuxapi,
}

/// NetEase turning a call down with its own answer code. Code 301 means the session is gone.
#[derive(Debug)]
pub struct Refused {
    pub code: i64,
    pub message: Option<String>,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.message {
            Some(message) => write!(
                f,
                "netease refused the call with code {}: {message}",
                self.code
            ),
            None => write!(f, "netease refused the call with code {}", self.code),
        }
    }
}

impl std::error::Error for Refused {}

/// A stream ready to fetch, and what NetEase said about it.
pub struct Resolved {
    pub url: String,
    pub encoding: Option<Encoding>,
    /// The bit rate NetEase actually serves, which tells two copies of one song apart.
    pub bitrate: u32,
}

#[derive(Clone)]
pub struct NetEaseClient {
    inner: Arc<Inner>,
}

struct Inner {
    http: reqwest::Client,
    /// The whole `Cookie` header every call carries.
    cookie: String,
    /// The `__csrf` cookie, sent again inside every encrypted body as `csrf_token`.
    csrf: String,
    profile: UserProfile,
    /// Artist portraits already looked up, so search results do not ask again.
    portraits: Mutex<HashMap<String, String>>,
}

impl NetEaseClient {
    /// Signs in with stored or pasted credentials, which reading the account checks. A session
    /// NetEase no longer knows answers with no profile, and the sign-in fails.
    pub(crate) async fn connect(credentials: &Credentials) -> Result<Self> {
        let unopened = Self::unopened(credentials, empty_profile())?;
        let account: wire::Account = unopened
            .call("/api/nuser/account/get", Map::new(), Scheme::Weapi)
            .await
            .context("cannot read the netease account")?;
        let profile = account
            .profile
            .context("netease does not know this session; sign in to music.163.com again")?
            .profile();
        Self::unopened(credentials, profile)
    }

    /// A client for `credentials` that has not checked them yet, answering `profile` as the
    /// account.
    pub(crate) fn unopened(credentials: &Credentials, profile: UserProfile) -> Result<Self> {
        // No overall timeout here: the same client streams songs minutes long. API calls set
        // their own, and a stalled stream ends on the read timeout.
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .build()
            .context("cannot build the netease http client")?;
        let mut cookie = format!("{CLIENT_COOKIES}; MUSIC_U={}", credentials.music_u);
        if let Some(csrf) = &credentials.csrf {
            cookie.push_str(&format!("; __csrf={csrf}"));
        }
        Ok(Self {
            inner: Arc::new(Inner {
                http,
                cookie,
                csrf: credentials.csrf.clone().unwrap_or_default(),
                profile,
                portraits: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// The signed-in account, as NetEase answered it at sign-in.
    pub(crate) fn account(&self) -> &UserProfile {
        &self.inner.profile
    }

    fn user_id(&self) -> &str {
        &self.inner.profile.id
    }

    /// One call, read into `T`. `path` is the endpoint as NetEase's own `/api/…` names it.
    async fn call<T: DeserializeOwned>(
        &self,
        path: &str,
        params: Map<String, Value>,
        scheme: Scheme,
    ) -> Result<T> {
        let answer = self.raw(path, params, scheme).await?;
        serde_json::from_value(answer)
            .with_context(|| format!("cannot read the netease answer to {path}"))
    }

    /// One call as JSON, refused unless NetEase answers code 200. A rate limit comes back with
    /// `Throttled` in its chain and a lost session with `Refused` code 301.
    async fn raw(
        &self,
        path: &str,
        mut params: Map<String, Value>,
        scheme: Scheme,
    ) -> Result<Value> {
        let (url, agent, body) = match scheme {
            Scheme::Weapi => {
                params.insert("csrf_token".into(), json!(self.inner.csrf));
                let tail = path.strip_prefix("/api").unwrap_or(path);
                (
                    format!("{BASE}/weapi{tail}"),
                    AGENT,
                    crypto::weapi(&Value::Object(params).to_string()),
                )
            }
            Scheme::Linuxapi => {
                let forwarded = json!({
                    "method": "POST",
                    "url": format!("{BASE}{path}"),
                    "params": params,
                });
                (
                    format!("{BASE}/api/linux/forward"),
                    LINUX_AGENT,
                    crypto::linuxapi(&forwarded.to_string()),
                )
            }
        };
        let response = self
            .inner
            .http
            .post(&url)
            .header("Cookie", &self.inner.cookie)
            .header("Referer", BASE)
            .header("User-Agent", agent)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .timeout(CALL_TIMEOUT)
            .body(body)
            .send()
            .await
            .with_context(|| format!("cannot reach netease for {path}"))?;
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("cannot read the netease answer to {path}"))?;
        let answer =
            decode(&bytes).with_context(|| format!("netease answered {path} with no json"))?;
        let code = answer.get("code").and_then(Value::as_i64).unwrap_or(200);
        if code == 200 {
            return Ok(answer);
        }
        let message = answer
            .get("message")
            .or_else(|| answer.get("msg"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let refused = anyhow::Error::new(Refused { code, message });
        match RATE_LIMITED.contains(&code) {
            true => Err(refused.context(Throttled)),
            false => Err(refused.context(format!("netease refused {path}"))),
        }
    }

    /// The songs with these ids, in the order asked, each with its privilege. An id NetEase no
    /// longer knows is left out.
    pub(crate) async fn songs(&self, ids: &[u64]) -> Result<Vec<wire::Song>> {
        let mut found: HashMap<u64, wire::Song> = HashMap::new();
        for batch in ids.chunks(DETAIL_BATCH) {
            let wanted: Vec<Value> = batch.iter().map(|id| json!({ "id": id })).collect();
            let answer: SongsAnswer = self
                .call(
                    "/api/v3/song/detail",
                    params([("c", json!(Value::Array(wanted).to_string()))]),
                    Scheme::Weapi,
                )
                .await
                .context("cannot load netease songs")?;
            let mut songs = answer.songs;
            wire::merge(&mut songs, answer.privileges);
            found.extend(songs.into_iter().map(|song| (song.id, song)));
        }
        Ok(ids.iter().filter_map(|id| found.remove(id)).collect())
    }

    /// The ids of the songs the account liked, newest first.
    async fn liked_ids(&self) -> Result<Vec<u64>> {
        let answer: LikedAnswer = self
            .call(
                "/api/song/like/get",
                params([("uid", json!(self.user_id()))]),
                Scheme::Weapi,
            )
            .await
            .context("cannot load the liked songs")?;
        Ok(answer.ids)
    }

    /// A playlist's details with up to `head` of its tracks in full and the order of all of
    /// them, privileges merged in.
    async fn playlist_head(&self, playlist_id: &str, head: usize) -> Result<wire::PlaylistInfo> {
        let answer: PlaylistAnswer = self
            .call(
                "/api/v6/playlist/detail",
                params([
                    ("id", json!(playlist_id)),
                    ("offset", json!("0")),
                    ("total", json!("true")),
                    ("limit", json!(head.to_string())),
                    ("n", json!(head.to_string())),
                ]),
                Scheme::Linuxapi,
            )
            .await
            .with_context(|| format!("cannot load the netease playlist {playlist_id}"))?;
        let mut playlist = answer.playlist;
        wire::merge(&mut playlist.tracks, answer.privileges);
        Ok(playlist)
    }

    /// Pages through a library listing that answers `data` and `hasMore`.
    async fn library<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>> {
        let mut items = Vec::new();
        for page in 0..LIBRARY_PAGES {
            let answer: LibraryAnswer<T> = self
                .call(
                    path,
                    params([
                        ("limit", json!(LIBRARY_PAGE)),
                        ("offset", json!(page * LIBRARY_PAGE)),
                        ("total", json!(true)),
                    ]),
                    Scheme::Weapi,
                )
                .await?;
            let count = answer.data.len();
            items.extend(answer.data);
            if !answer.has_more || count == 0 {
                break;
            }
        }
        Ok(items)
    }

    /// One search of `kind`: 1 songs, 10 albums, 100 artists, 1000 playlists.
    async fn search_kind(&self, query: &str, kind: u32) -> Result<SearchResult> {
        let answer: SearchAnswer = self
            .call(
                "/api/cloudsearch/pc",
                params([
                    ("s", json!(query)),
                    ("type", json!(kind.to_string())),
                    ("offset", json!("0")),
                    ("limit", json!(SEARCH_LIMIT.to_string())),
                ]),
                // The web player's own search endpoint refuses a session it does not trust,
                // while the desktop one answers anyone, in the same shape.
                Scheme::Linuxapi,
            )
            .await
            .context("cannot search netease")?;
        Ok(answer.result.unwrap_or_default())
    }

    async fn artist_detail(&self, artist_id: &str) -> Result<wire::ArtistDetail> {
        self.call(
            &format!("/api/v1/artist/{}", digits(artist_id)?),
            Map::new(),
            Scheme::Weapi,
        )
        .await
        .with_context(|| format!("cannot load the netease artist {artist_id}"))
    }

    async fn artist_albums(&self, artist_id: &str) -> Result<Vec<Album>> {
        let answer: ArtistAlbumsAnswer = self
            .call(
                &format!("/api/artist/albums/{}", digits(artist_id)?),
                params([
                    ("limit", json!(DISCOGRAPHY.to_string())),
                    ("offset", json!("0")),
                    ("total", json!("true")),
                ]),
                Scheme::Weapi,
            )
            .await
            .with_context(|| format!("cannot load the albums of netease artist {artist_id}"))?;
        Ok(answer
            .hot_albums
            .iter()
            .map(wire::AlbumInfo::album)
            .collect())
    }

    /// A write that answers nothing worth reading.
    async fn write(&self, path: &str, body: Map<String, Value>, what: &str) -> Result<()> {
        self.raw(path, body, Scheme::Weapi)
            .await
            .with_context(|| format!("cannot {what}"))?;
        Ok(())
    }

    async fn manipulate(&self, op: &str, playlist_id: &str, track_ids: &[String]) -> Result<()> {
        let ids = Value::from(track_ids.to_vec()).to_string();
        self.write(
            "/api/playlist/manipulate/tracks",
            params([
                ("op", json!(op)),
                ("pid", json!(playlist_id)),
                ("trackIds", json!(ids)),
                ("imme", json!("true")),
            ]),
            "change the netease playlist",
        )
        .await
    }

    /// The stream of a song at `quality`, or the best below it this account may hear. The v1
    /// endpoint answers first; when it fails, or hands out only a preview without saying the
    /// song is refused outright, the legacy endpoint is asked too. A song with no whole stream
    /// for this account is an error, which plays as unavailable.
    pub(crate) async fn resolve(&self, track_id: &str, quality: Quality) -> Result<Resolved> {
        let id = digits(track_id)?;
        let quality = quality.within(QUALITIES);
        let level = match quality {
            Quality::HiRes => "hires",
            Quality::Lossless => "lossless",
            Quality::High => "exhigh",
            Quality::Normal | Quality::Low => "standard",
        };
        let v1 = self
            .call::<UrlAnswer>(
                "/api/song/enhance/player/url/v1",
                params([
                    ("ids", json!(format!("[{id}]"))),
                    ("level", json!(level)),
                    ("encodeType", json!("flac")),
                ]),
                Scheme::Weapi,
            )
            .await;
        match v1 {
            Ok(answer) => {
                if let Some(resolved) = whole(&answer.data) {
                    return Ok(resolved);
                }
                if !answer.data.is_empty()
                    && answer
                        .data
                        .iter()
                        .all(|url| url.code.is_some_and(|code| code != 200))
                {
                    bail!("netease has no stream of {track_id} for this account");
                }
            }
            Err(error) => log::debug!("netease: the v1 stream of {track_id} failed: {error:#}"),
        }
        let bitrate = match quality {
            Quality::HiRes => "1999000",
            Quality::Lossless => "999000",
            Quality::High => "320000",
            Quality::Normal | Quality::Low => "128000",
        };
        let legacy: UrlAnswer = self
            .call(
                "/api/song/enhance/player/url",
                params([("ids", json!(format!("[{id}]"))), ("br", json!(bitrate))]),
                Scheme::Linuxapi,
            )
            .await
            .with_context(|| format!("cannot find a stream of {track_id}"))?;
        whole(&legacy.data)
            .with_context(|| format!("netease has no whole stream of {track_id} for this account"))
    }

    /// Opens a resolved stream.
    pub(crate) async fn open(&self, resolved: &Resolved) -> Result<reqwest::Response> {
        self.inner
            .http
            .get(&resolved.url)
            .header("User-Agent", AGENT)
            .header("Referer", BASE)
            .send()
            .await
            .context("cannot stream the netease track")?
            .error_for_status()
            .context("the netease cdn refused the stream")
    }

    /// Whether a failure means the account has to sign in again.
    pub(crate) fn signed_out(error: &anyhow::Error) -> bool {
        error
            .downcast_ref::<Refused>()
            .is_some_and(|refused| refused.code == SIGNED_OUT)
    }
}

#[async_trait]
impl MusicApi for NetEaseClient {
    fn share_url(&self, kind: MediaKind, id: &str) -> Option<String> {
        let kind = match kind {
            MediaKind::Track => "song",
            MediaKind::Album => "album",
            MediaKind::Artist => "artist",
            MediaKind::Playlist => "playlist",
        };
        let id = digits(id).ok()?;
        Some(format!("{BASE}/#/{kind}?id={id}"))
    }

    async fn profile(&self) -> Result<UserProfile> {
        Ok(self.inner.profile.clone())
    }

    async fn artist(&self, artist_id: &str) -> Result<Artist> {
        let (detail, albums) =
            tokio::join!(self.artist_detail(artist_id), self.artist_albums(artist_id));
        let detail = detail?;
        let albums = albums
            .inspect_err(|error| log::warn!("netease: no discography for {artist_id}: {error:#}"))
            .unwrap_or_default();
        Ok(detail.artist(albums))
    }

    async fn artist_profile(&self, artist_id: &str) -> Result<ArtistProfile> {
        let detail = self.artist_detail(artist_id).await?;
        let artist = detail.artist(Vec::new());
        Ok(ArtistProfile {
            name: artist.name,
            cover_large: artist.cover_large,
            biography: artist.biography,
        })
    }

    /// Answers from the portrait cache first and looks a few of the rest up at once. An artist
    /// NetEase will not answer for is left out.
    async fn artist_images(&self, ids: Vec<String>) -> Result<HashMap<String, String>> {
        let mut images = HashMap::new();
        let mut missing = Vec::new();
        {
            let known = lock(&self.inner.portraits);
            for id in ids {
                match known.get(&id) {
                    Some(cover) => {
                        images.insert(id, cover.clone());
                    }
                    None if missing.len() < PORTRAIT_LIMIT => missing.push(id),
                    None => {}
                }
            }
        }
        let looked = futures::future::join_all(missing.into_iter().map(|id| async move {
            let detail = self.artist_detail(&id).await.ok()?;
            Some((id, detail.artist.picture(300)?))
        }))
        .await;
        let mut known = lock(&self.inner.portraits);
        for (id, cover) in looked.into_iter().flatten() {
            known.insert(id.clone(), cover.clone());
            images.insert(id, cover);
        }
        Ok(images)
    }

    async fn saved_tracks(&self) -> Result<Vec<Track>> {
        let ids = self.liked_ids().await?;
        Ok(self
            .songs(&ids)
            .await?
            .iter()
            .map(|song| song.track(None))
            .collect())
    }

    /// The liked songs a detail batch at a time, so the first rows show while the rest load.
    async fn saved_tracks_paged(&self) -> Result<Pages<Track>> {
        let ids = self.liked_ids().await?;
        let (sink, pages) = tokio::sync::mpsc::channel(FAN);
        let client = self.clone();
        tokio::spawn(async move {
            let total = Some(ids.len());
            if ids.is_empty() {
                sink.send(Ok(Page {
                    total,
                    items: Vec::new(),
                }))
                .await
                .ok();
                return;
            }
            for batch in ids.chunks(DETAIL_BATCH) {
                let page = client.songs(batch).await.map(|songs| Page {
                    total,
                    items: songs.iter().map(|song| song.track(None)).collect(),
                });
                let failed = page.is_err();
                if sink.send(page).await.is_err() || failed {
                    return;
                }
            }
        });
        Ok(pages)
    }

    async fn set_track_saved(&self, track_id: &str, saved: bool) -> Result<()> {
        self.write(
            "/api/radio/like",
            params([
                ("trackId", json!(digits(track_id)?)),
                ("like", json!(saved.to_string())),
                ("alg", json!("itembased")),
                ("time", json!("3")),
            ]),
            "change the like on the netease song",
        )
        .await
    }

    async fn track(&self, track_id: &str) -> Result<Track> {
        let id: u64 = digits(track_id)?
            .parse()
            .with_context(|| format!("{track_id} is not a netease song id"))?;
        let songs = self.songs(&[id]).await?;
        songs
            .first()
            .map(|song| song.track(None))
            .with_context(|| format!("netease does not know the song {track_id}"))
    }

    async fn track_playcount(&self, _track_id: &str) -> Result<Option<u64>> {
        Ok(None)
    }

    async fn playlists(&self) -> Result<Vec<Playlist>> {
        let answer: PlaylistsAnswer = self
            .call(
                "/api/user/playlist",
                params([
                    ("uid", json!(self.user_id())),
                    ("limit", json!("1000")),
                    ("offset", json!("0")),
                    ("includeVideo", json!(true)),
                ]),
                Scheme::Weapi,
            )
            .await
            .context("cannot load the netease playlists")?;
        Ok(answer
            .playlist
            .iter()
            .map(|playlist| playlist.playlist(self.user_id()))
            .collect())
    }

    async fn create_playlist(&self, name: &str) -> Result<String> {
        let answer = self
            .raw(
                "/api/playlist/create",
                params([
                    ("name", json!(name)),
                    ("privacy", json!("0")),
                    ("type", json!("NORMAL")),
                ]),
                Scheme::Weapi,
            )
            .await
            .context("cannot create the netease playlist")?;
        answer
            .get("id")
            .or_else(|| {
                answer
                    .get("playlist")
                    .and_then(|playlist| playlist.get("id"))
            })
            .and_then(Value::as_u64)
            .map(|id| id.to_string())
            .context("the created playlist came back with no id")
    }

    async fn rename_playlist(&self, playlist_id: &str, name: &str) -> Result<()> {
        self.write(
            "/api/playlist/update/name",
            params([("id", json!(playlist_id)), ("name", json!(name))]),
            "rename the netease playlist",
        )
        .await
    }

    async fn delete_playlist(&self, playlist_id: &str) -> Result<()> {
        self.write(
            "/api/playlist/remove",
            params([("ids", json!(format!("[{}]", digits(playlist_id)?)))]),
            "delete the netease playlist",
        )
        .await
    }

    async fn remove_playlist_from_library(&self, playlist_id: &str) -> Result<()> {
        self.write(
            "/api/playlist/unsubscribe",
            params([("id", json!(playlist_id))]),
            "unfollow the netease playlist",
        )
        .await
    }

    async fn add_playlist_to_library(&self, playlist_id: &str) -> Result<()> {
        self.write(
            "/api/playlist/subscribe",
            params([("id", json!(playlist_id))]),
            "follow the netease playlist",
        )
        .await
    }

    async fn set_playlist_public(&self, _playlist_id: &str, _public: bool) -> Result<()> {
        bail!("netease cannot change a playlist's visibility from here")
    }

    async fn add_track_to_playlist(&self, playlist_id: &str, track_id: &str) -> Result<()> {
        self.manipulate("add", playlist_id, &[track_id.to_owned()])
            .await
    }

    /// NetEase keeps a song in a playlist only once, so a song named twice goes in once.
    async fn add_tracks_to_playlist(&self, playlist_id: &str, track_ids: &[String]) -> Result<()> {
        if track_ids.is_empty() {
            return Ok(());
        }
        self.manipulate("add", playlist_id, track_ids).await
    }

    async fn remove_track_from_playlist(&self, playlist_id: &str, track_id: &str) -> Result<()> {
        self.manipulate("del", playlist_id, &[track_id.to_owned()])
            .await
    }

    async fn saved_albums(&self) -> Result<Vec<Album>> {
        let albums: Vec<wire::AlbumInfo> = self
            .library("/api/album/sublist")
            .await
            .context("cannot load the saved netease albums")?;
        Ok(albums.iter().map(wire::AlbumInfo::album).collect())
    }

    async fn set_album_saved(&self, album_id: &str, saved: bool) -> Result<()> {
        let path = match saved {
            true => "/api/album/sub",
            false => "/api/album/unsub",
        };
        self.write(
            path,
            params([("id", json!(album_id))]),
            "save the netease album",
        )
        .await
    }

    async fn saved_artists(&self) -> Result<Vec<SavedArtist>> {
        let artists: Vec<wire::Named> = self
            .library("/api/artist/sublist")
            .await
            .context("cannot load the followed netease artists")?;
        Ok(artists.iter().map(wire::Named::saved).collect())
    }

    async fn set_artist_saved(&self, artist_id: &str, saved: bool) -> Result<()> {
        let path = match saved {
            true => "/api/artist/sub",
            false => "/api/artist/unsub",
        };
        let id = digits(artist_id)?;
        self.write(
            path,
            params([
                ("artistId", json!(id)),
                ("artistIds", json!(format!("[{id}]"))),
            ]),
            "follow the netease artist",
        )
        .await
    }

    async fn album(&self, album_id: &str) -> Result<AlbumDetail> {
        let answer: AlbumAnswer = self
            .call(
                &format!("/api/v1/album/{}", digits(album_id)?),
                Map::new(),
                Scheme::Weapi,
            )
            .await
            .with_context(|| format!("cannot load the netease album {album_id}"))?;
        let mut songs = answer.songs;
        wire::merge(&mut songs, answer.privileges);
        let mut album = answer.album.album();
        if album.track_count == 0 {
            album.track_count = songs.len() as u32;
        }
        Ok(AlbumDetail {
            album,
            tracks: songs.iter().map(|song| song.track(None)).collect(),
        })
    }

    async fn album_tracks(&self, album_id: &str) -> Result<Vec<Track>> {
        Ok(self.album(album_id).await?.tracks)
    }

    /// The whole playlist: the first stretch comes with the details, and the rest by id in
    /// detail batches.
    async fn playlist(&self, playlist_id: &str) -> Result<PlaylistDetail> {
        let head = self.playlist_head(playlist_id, PLAYLIST_HEAD).await?;
        let added: HashMap<u64, i64> = head
            .track_ids
            .iter()
            .filter_map(|entry| Some((entry.id, entry.at? / 1000)))
            .collect();
        let mut songs: HashMap<u64, wire::Song> = head
            .tracks
            .iter()
            .map(|song| (song.id, song.clone()))
            .collect();
        let missing: Vec<u64> = head
            .track_ids
            .iter()
            .map(|entry| entry.id)
            .filter(|id| !songs.contains_key(id))
            .collect();
        if !missing.is_empty() {
            songs.extend(
                self.songs(&missing)
                    .await?
                    .into_iter()
                    .map(|song| (song.id, song)),
            );
        }
        let tracks: Vec<Track> = match head.track_ids.is_empty() {
            true => head.tracks.iter().map(|song| song.track(None)).collect(),
            false => head
                .track_ids
                .iter()
                .filter_map(|entry| songs.get(&entry.id))
                .map(|song| song.track(added.get(&song.id).copied()))
                .collect(),
        };
        let mut playlist = head.playlist(self.user_id());
        if playlist.track_count == 0 {
            playlist.track_count = tracks.len() as u32;
        }
        if playlist.cover.is_none() {
            playlist.cover = tracks.iter().find_map(|track| track.cover.clone());
        }
        Ok(PlaylistDetail {
            playlist,
            tracks,
            continuation: None,
        })
    }

    async fn playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Track>> {
        Ok(self.playlist(playlist_id).await?.tracks)
    }

    async fn playlist_covers(&self, playlist_id: &str, wanted: usize) -> Result<Vec<String>> {
        let head = self
            .playlist_head(playlist_id, (wanted * 8).clamp(8, 100))
            .await?;
        let tracks: Vec<Track> = head.tracks.iter().map(|song| song.track(None)).collect();
        Ok(distinct_covers(&tracks, wanted))
    }

    /// The songs NetEase calls similar, in one stretch. NetEase answers that list only to
    /// some sessions, so an empty one falls back to the popular songs of the seed's artist.
    async fn track_radio(
        &self,
        track_id: &str,
        _from: Option<&str>,
    ) -> Result<(Vec<Track>, Option<String>)> {
        let similar = self
            .call::<SongsAnswer>(
                "/api/v1/discovery/simiSong",
                params([
                    ("songid", json!(digits(track_id)?)),
                    ("limit", json!(RADIO_LIMIT)),
                    ("offset", json!(0)),
                ]),
                Scheme::Weapi,
            )
            .await
            .inspect_err(|error| log::warn!("netease: no similar songs for {track_id}: {error:#}"))
            .map(|answer| answer.songs)
            .unwrap_or_default();
        if !similar.is_empty() {
            return Ok((similar.iter().map(|song| song.track(None)).collect(), None));
        }
        let Some(artist_id) = self
            .track(track_id)
            .await
            .ok()
            .and_then(|seed| seed.artist_refs.into_iter().find_map(|artist| artist.id))
        else {
            return Ok((Vec::new(), None));
        };
        let popular = self
            .artist_detail(&artist_id)
            .await
            .inspect_err(|error| log::warn!("netease: no songs to follow {track_id}: {error:#}"))
            .map(|detail| detail.hot_songs)
            .unwrap_or_default();
        Ok((
            popular
                .iter()
                .filter(|song| song.id.to_string() != track_id)
                .map(|song| song.track(None))
                .collect(),
            None,
        ))
    }

    async fn search(&self, query: &str) -> Result<Vec<Track>> {
        let result = self.search_kind(query, 1).await?;
        Ok(result.songs.iter().map(|song| song.track(None)).collect())
    }

    async fn search_albums(&self, query: &str) -> Result<Vec<Album>> {
        let result = self.search_kind(query, 10).await?;
        Ok(result.albums.iter().map(wire::AlbumInfo::album).collect())
    }

    async fn search_playlists(&self, query: &str) -> Result<Vec<Playlist>> {
        let result = self.search_kind(query, 1000).await?;
        Ok(result
            .playlists
            .iter()
            .map(|playlist| playlist.playlist(self.user_id()))
            .collect())
    }
}

#[derive(serde::Deserialize)]
struct SongsAnswer {
    #[serde(default)]
    songs: Vec<wire::Song>,
    #[serde(default)]
    privileges: Vec<wire::Privilege>,
}

#[derive(serde::Deserialize)]
struct LikedAnswer {
    #[serde(default)]
    ids: Vec<u64>,
}

#[derive(serde::Deserialize)]
struct PlaylistAnswer {
    playlist: wire::PlaylistInfo,
    #[serde(default)]
    privileges: Vec<wire::Privilege>,
}

#[derive(serde::Deserialize)]
struct PlaylistsAnswer {
    #[serde(default)]
    playlist: Vec<wire::PlaylistInfo>,
}

#[derive(serde::Deserialize)]
struct AlbumAnswer {
    album: wire::AlbumInfo,
    #[serde(default)]
    songs: Vec<wire::Song>,
    #[serde(default)]
    privileges: Vec<wire::Privilege>,
}

#[derive(serde::Deserialize)]
struct ArtistAlbumsAnswer {
    #[serde(default, rename = "hotAlbums")]
    hot_albums: Vec<wire::AlbumInfo>,
}

#[derive(serde::Deserialize)]
struct LibraryAnswer<T> {
    #[serde(default = "Vec::new")]
    data: Vec<T>,
    #[serde(default, rename = "hasMore")]
    has_more: bool,
}

#[derive(serde::Deserialize)]
struct SearchAnswer {
    #[serde(default)]
    result: Option<SearchResult>,
}

#[derive(Default, serde::Deserialize)]
struct SearchResult {
    #[serde(default)]
    songs: Vec<wire::Song>,
    #[serde(default)]
    albums: Vec<wire::AlbumInfo>,
    #[serde(default)]
    playlists: Vec<wire::PlaylistInfo>,
}

#[derive(serde::Deserialize)]
struct UrlAnswer {
    #[serde(default)]
    data: Vec<wire::SongUrl>,
}

/// The account a client answers before it has read the real one.
fn empty_profile() -> UserProfile {
    UserProfile {
        id: String::new(),
        display_name: String::new(),
        avatar: None,
    }
}

/// A parameter map from pairs.
fn params<const N: usize>(pairs: [(&str, Value); N]) -> Map<String, Value> {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

/// An id NetEase takes in a path or a JSON list, which is digits alone.
fn digits(id: &str) -> Result<&str> {
    let id = id.trim();
    if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("{id} is not a netease id");
    }
    Ok(id)
}

/// The first whole stream of an answer.
fn whole(urls: &[wire::SongUrl]) -> Option<Resolved> {
    urls.iter().find_map(|url| {
        Some(Resolved {
            url: url.whole()?.to_owned(),
            encoding: url.encoding(),
            bitrate: url.br,
        })
    })
}

/// An answer body as JSON. Some endpoints send it zlib-compressed without saying so.
fn decode(bytes: &[u8]) -> Result<Value> {
    if let Ok(answer) = serde_json::from_slice(bytes) {
        return Ok(answer);
    }
    let mut inflated = Vec::new();
    flate2::read::ZlibDecoder::new(bytes)
        .read_to_end(&mut inflated)
        .context("the answer is neither json nor zlib")?;
    serde_json::from_slice(&inflated).context("the inflated answer is not json")
}

/// Locks a mutex whose guarded value is still sound after a panic elsewhere.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_must_be_digits() {
        assert_eq!(digits(" 186016 ").ok(), Some("186016"));
        assert!(digits("").is_err());
        assert!(digits("1,2").is_err());
        assert!(digits("../x").is_err());
    }

    #[test]
    fn a_lost_session_is_told_apart() {
        let lost = anyhow::Error::new(Refused {
            code: SIGNED_OUT,
            message: None,
        })
        .context("netease refused /api/user/playlist");
        assert!(NetEaseClient::signed_out(&lost));
        let busy = anyhow::Error::new(Refused {
            code: 512,
            message: None,
        })
        .context(Throttled);
        assert!(!NetEaseClient::signed_out(&busy));
        assert!(
            busy.context("cannot load")
                .downcast_ref::<Throttled>()
                .is_some()
        );
    }

    #[test]
    fn a_compressed_answer_still_reads() {
        use std::io::Write as _;
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(br#"{"code":200}"#).ok();
        let packed = encoder.finish().unwrap_or_default();
        assert_eq!(decode(&packed).ok(), Some(json!({ "code": 200 })));
        assert_eq!(
            decode(br#"{"code":301}"#).ok(),
            Some(json!({ "code": 301 }))
        );
    }
}
