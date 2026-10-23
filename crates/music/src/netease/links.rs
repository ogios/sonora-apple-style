//! NetEase Cloud Music's links: music.163.com pages in the old `/#/song?id=1` shape the website
//! still writes, the `/song?id=1` and `/song/1/` shapes its share sheet hands out, the mobile
//! `y.music.163.com/m/…` pages, and the `163cn.tv` short links.

use crate::link::{self, Linked};

const HOSTS: [&str; 2] = ["music.163.com", "y.music.163.com"];
const SHORT_HOSTS: [&str; 2] = ["163cn.tv", "163cn.link"];

/// What a NetEase link points at.
pub(super) fn read(text: &str) -> Option<Linked> {
    let url = link::address(text)?;
    if !matches!(url.scheme(), "https" | "http") {
        return None;
    }
    let host = link::host(&url)?;
    if SHORT_HOSTS.contains(&host) {
        return Some(Linked::Short);
    }
    if !HOSTS.contains(&host) {
        return None;
    }
    // The website routes inside the fragment, as in /#/song?id=1, so that is read as the
    // address when it holds a path.
    let page = match url.fragment().filter(|fragment| fragment.starts_with('/')) {
        Some(fragment) => link::address(&format!("https://{host}{fragment}"))?,
        None => url,
    };
    let segments = link::segments(&page);
    let at = segments
        .iter()
        .position(|part| matches!(*part, "song" | "album" | "artist" | "playlist" | "toplist"))?;
    let id = link::query(&page, "id")
        .or_else(|| segments.get(at + 1).map(|id| (*id).to_owned()))
        .filter(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))?;
    match segments[at] {
        "song" => Some(Linked::Track(id)),
        "album" => Some(Linked::Album(id)),
        "artist" => Some(Linked::Artist(id)),
        _ => Some(Linked::Playlist(id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_website_shapes() {
        assert_eq!(
            read("https://music.163.com/#/song?id=186016"),
            Some(Linked::Track("186016".into()))
        );
        assert_eq!(
            read("https://music.163.com/song?id=186016&userid=1"),
            Some(Linked::Track("186016".into()))
        );
        assert_eq!(
            read("https://music.163.com/song/186016/?userid=1"),
            Some(Linked::Track("186016".into()))
        );
        assert_eq!(
            read("https://y.music.163.com/m/playlist?id=24381616"),
            Some(Linked::Playlist("24381616".into()))
        );
        assert_eq!(
            read("http://music.163.com/#/album?id=18905"),
            Some(Linked::Album("18905".into()))
        );
        assert_eq!(
            read("https://music.163.com/#/artist?id=6452"),
            Some(Linked::Artist("6452".into()))
        );
        assert_eq!(
            read("https://music.163.com/#/discover/toplist?id=3778678"),
            Some(Linked::Playlist("3778678".into()))
        );
    }

    #[test]
    fn short_links_are_followed_first() {
        assert_eq!(read("https://163cn.tv/abcd123"), Some(Linked::Short));
    }

    #[test]
    fn other_links_are_not_netease() {
        assert_eq!(read("https://music.163.com/#/discover"), None);
        // NetEase user pages are not something Sonora opens.
        assert_eq!(read("https://music.163.com/#/user/home?id=1"), None);
        assert_eq!(read("https://music.163.com/#/song?id=abc"), None);
        assert_eq!(read("https://www.deezer.com/track/1"), None);
        assert_eq!(read("not a link"), None);
    }
}
