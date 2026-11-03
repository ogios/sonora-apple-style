use super::Arguments;
use crate::{Addition, Collection, DeviceRule, PinSort, RuleDevice, Shelf, Sonora, Sound};
use anyhow::{Context as _, Result, bail};
use gpui::App;
use music::connect::Command;
use music::equalizer::{Curve, Preset, Tone};
use music::scrobble::Secret;
use music::{Album, AlbumDetail, Playlist, PlaylistDetail, SavedArtist, Track};
use protocol::Call;
use serde_json::Value;
use std::path::PathBuf;
use ui::Pin;

/// Routes typed commands to the authoritative singleton entities.
pub(super) fn dispatch(call: &Call, cx: &mut App) -> Result<Value> {
    let sonora = Sonora::global(cx);
    match call.object.as_str() {
        "session" if call.method == "share_url" => {
            let mut args = Arguments::new(call, 2)?;
            let kind: music::MediaKind = args.next()?;
            let id: String = args.next()?;
            let session = sonora.session.read(cx);
            let provider = match music::is_local_id(&id) {
                true => session.local_client(),
                false => session.client(),
            };
            Ok(serde_json::to_value(
                provider.and_then(|provider| provider.share_url(kind, &id)),
            )?)
        }
        "session" => {
            let entity = sonora.session.clone();
            commands!(call, entity, cx, {
                forget(slug: String => &slug),
                switch(slug: String => &slug),
                restore(),
                sign_in(slug: String => &slug, method: music::SignIn => method),
                sign_in_with_cookies(slug: String => &slug),
                cancel_sign_in(),
                submit_input(text: String => text),
                sign_out(),
                go_offline(),
                restore_if_offline(),
                reconnect_if_stale(),
                add_local_folder(path: PathBuf => path),
                add_local_folders(paths: Vec<PathBuf> => paths),
                remove_local_folder(path: PathBuf => &path),
                rescan_local(thorough: bool => thorough),
            })
        }
        "library" => {
            let entity = sonora.library.clone();
            commands!(call, entity, cx, {
                add_local_folder(path: PathBuf => path),
                add_local_folders(paths: Vec<PathBuf> => paths),
                remove_local_folder(path: PathBuf => path),
                rescan_local(thorough: bool => thorough),
                delete_track_files(ids: Vec<String> => ids),
                hide_local_tracks(ids: Vec<String> => &ids),
                toggle(track: Track => track),
                save_tracks(tracks: Vec<Track> => tracks, saved: bool => saved),
                set_track_in_library(track: Track => track, present: bool => present),
                set_album_in_library(album: Album => album, present: bool => present),
                toggle_album(album: Album => album),
                heart_album(album: Album => album),
                toggle_artist(artist: SavedArtist => artist),
                heart_artist(artist: SavedArtist => artist),
                create_playlist(name: String => name, addition: Addition => addition, shelf: Shelf => shelf),
                rename_playlist(id: String => id, name: String => name),
                set_playlist_public(id: String => id, public: bool => public),
                add_to_playlist(playlist_id: String => playlist_id, track_id: String => track_id),
                add_tracks_to_playlist(playlist_id: String => playlist_id, addition: Addition => addition),
                remove_from_playlist(playlist_id: String => playlist_id, track_id: String => track_id),
                remove_tracks_from_playlist(playlist_id: String => playlist_id, rows: Vec<String> => rows),
                delete_playlist(id: String => id),
                add_playlist_to_library(playlist: Playlist => playlist),
                remove_playlist_from_library(id: String => id),
                remove_playlists_from_library(ids: Vec<String> => ids),
                read_playlists(shelf: Shelf => shelf),
                liked_elsewhere(id: String => &id, track: Option<Track> => track),
                refresh(shelf: Shelf => shelf),
                reload(shelf: Shelf => shelf),
            })
        }
        "outputs" => {
            let entity = sonora.outputs.clone();
            commands!(call, entity, cx, {
                set_rate(rate: u32 => rate),
                set_buffer(frames: u32 => frames),
                choose(id: Option<String> => id),
                refresh(),
            })
        }
        "history" => {
            let entity = sonora.history.clone();
            commands!(call, entity, cx, {
                remove(track: Track => &track),
                clear(),
                reload(),
                refresh(),
            })
        }
        "network" => {
            let entity = sonora.network.clone();
            commands!(call, entity, cx, {
                set_offline(offline: bool => offline),
            })
        }
        "drm" => {
            let entity = sonora.drm.clone();
            commands!(call, entity, cx, {
                look(),
                dismiss(),
                download(),
                accept(),
                uninstall(),
            })
        }
        "usage" => {
            let entity = sonora.usage.clone();
            commands!(call, entity, cx, {
                consent(consented: bool => consented),
                report(),
            })
        }
        "downloads" => {
            let entity = sonora.downloads.clone();
            commands!(call, entity, cx, {
                download_tracks(tracks: Vec<Track> => tracks),
                download_album(detail: AlbumDetail => detail),
                download_playlist(detail: PlaylistDetail => detail),
                download_album_by_id(album: Album => &album),
                download_playlist_by_id(playlist: Playlist => &playlist),
                remove_tracks(ids: Vec<String> => &ids),
                remove_set(kind: Collection => kind, id: String => &id),
                remove_all(),
            })
        }
        "pins" => {
            let entity = sonora.pins.clone();
            commands!(call, entity, cx, {
                choose(sort: PinSort => sort),
                toggle(pin: Pin => pin),
                place(pin: Pin => pin, gap: Option<usize> => gap),
                unpin(pin: Pin => pin),
            })
        }
        "devices" => {
            let entity = sonora.devices.clone();
            commands!(call, entity, cx, {
                peek(link: String => &link),
                accept(),
                dismiss(),
                kick(member: String => &member),
                start_jam(),
                show_code(shown: bool => shown),
                show_join(shown: bool => shown),
                leave(),
                transfer_here(),
                transfer(device: String => &device),
                request_control(device: String => &device, command: Command => command),
            })
        }
        "scan" => {
            let entity = sonora.scan.clone();
            commands!(call, entity, cx, {
                viewing_settings(viewing: bool => viewing),
            })
        }
        "equalizer" => {
            let entity = sonora.equalizer.clone();
            commands!(call, entity, cx, {
                set_tone(tone: Tone => tone),
                glide_tone(tone: Tone => tone),
                set_enabled(on: bool => on),
                set_curve(curve: Curve => curve),
                undo(),
                redo(),
                glide_curve(curve: Curve => curve),
                lower_preamp(),
                set_rule(rule: DeviceRule => rule),
                remove_rule(device: RuleDevice => &device),
                play_sound(sound: Sound => &sound),
                select_profile(index: usize => index),
                play_profile(index: usize => index),
                play_preset(preset: Preset => preset),
                add_profile(name: String => &name, curve: Curve => curve),
                save_as(name: String => &name),
                remove_profile(index: usize => index, name: String => &name),
            })
        }
        "scrobbling" => {
            let entity = sonora.scrobbling.clone();
            commands!(call, entity, cx, {
                set_enabled(service: String => &service, enabled: bool => enabled),
                connect(service: String => &service, secret: Secret => secret),
                disconnect(service: String => &service),
                cancel(),
            })
        }
        "links" => {
            if call.method != "open" {
                bail!("unknown link command");
            }
            let text: String = Arguments::new(call, 1)?.next()?;
            let entity = sonora.links.clone();
            let opened = entity.update(cx, |links, cx| links.open(&text, cx));
            if !opened {
                crate::Toasts::show(crate::Outcome::Failed, "toast-link-unreadable", cx);
            }
            Ok(Value::Bool(opened))
        }
        _ => bail!("the service does not expose this object"),
    }
}
