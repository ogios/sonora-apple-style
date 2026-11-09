use std::sync::Arc;

use crate::runtime::App;
use music::{LyricsProvider, MusicProvider};

use crate::Io;

/// Installs the service's runtime, settings and shared state with all bundled providers.
/// Call once before creating clients or restoring provider sessions.
pub fn start(io: Io, cx: &mut App) {
    let database = storage::Database::standard();
    let providers: Vec<Arc<dyn MusicProvider>> = vec![
        Arc::new(music::spotify::SpotifyProvider::from_env()),
        Arc::new(music::apple::AppleProvider::new()),
        Arc::new(music::youtube::YouTubeProvider::new()),
        Arc::new(music::subsonic::SubsonicProvider::new()),
        Arc::new(music::deezer::DeezerProvider::new()),
        Arc::new(music::tidal::TidalProvider::new()),
        Arc::new(music::netease::NetEaseProvider::new()),
    ];
    let local_provider: Arc<dyn MusicProvider> = Arc::new(music::local::LocalProvider::new(
        dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("sonora"),
        database.clone(),
        storage::Cache::standard(),
    ));
    let lyrics: Vec<Arc<dyn LyricsProvider>> = vec![
        Arc::new(music::local::LocalLyrics),
        Arc::new(music::subsonic::SubsonicLyrics::new()),
        Arc::new(music::spotify::SpotifyLyrics::from_env()),
        Arc::new(music::youtube::YouTubeLyrics::new()),
        Arc::new(music::binimum::Binimum::new()),
        Arc::new(music::musixmatch::Musixmatch::new()),
        Arc::new(music::lrclib::LrcLib::new()),
        Arc::new(music::kugou::Kugou::new()),
        Arc::new(music::netease::NetEase::new()),
    ];
    crate::init(cx, io, database, providers, local_provider, lyrics);
}
