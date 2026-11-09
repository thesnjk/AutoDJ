//! Rekordbox DJ_PLAYLISTS XML ingest (offline).
//!
//! Parses the official playlist-share schema subset. User-exported XML only —
//! do not commit commercial collection dumps.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;

use crate::camelot::CamelotKey;

/// Parsed Rekordbox playlist with resolved local paths and prep metadata.
#[derive(Debug, Clone)]
pub struct RekordboxPlaylist {
    /// Playlist NODE name.
    pub name: String,
    /// Tracks in playlist order.
    pub tracks: Vec<RekordboxTrack>,
}

/// One track from a Rekordbox playlist export.
#[derive(Debug, Clone)]
pub struct RekordboxTrack {
    /// Local filesystem path (from `Location`, resolved).
    pub path: PathBuf,
    /// BPM from `AverageBpm` / first `TEMPO/@Bpm`.
    pub bpm: Option<f32>,
    /// Camelot key string after flexible Tonality mapping.
    pub key: Option<String>,
    /// Beat-grid offset from first `TEMPO/@Inizio` (seconds, `[0, beat)`).
    pub beat_offset_secs: Option<f64>,
    /// Fade-In mark start → intro end hint.
    pub intro_end: Option<f64>,
    /// Fade-Out mark start → outro start hint.
    pub outro_start: Option<f64>,
}

#[derive(Debug, Default, Clone)]
struct RawTrack {
    track_id: String,
    location: String,
    average_bpm: Option<f32>,
    tonality: Option<String>,
    tempo_bpm: Option<f32>,
    tempo_inizio: Option<f64>,
    intro_end: Option<f64>,
    outro_start: Option<f64>,
}

#[derive(Debug, Clone)]
struct NamedPlaylist {
    name: String,
    keys: Vec<String>,
}

/// Parse a Rekordbox `DJ_PLAYLISTS` XML file.
///
/// When `playlist_name` is `Some`, selects the first leaf playlist NODE whose
/// name matches (case-insensitive). Otherwise uses the first leaf playlist.
pub fn parse_rekordbox_xml(
    path: &Path,
    playlist_name: Option<&str>,
) -> Result<RekordboxPlaylist> {
    let xml_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    parse_rekordbox_bytes(&bytes, &xml_dir, playlist_name)
}

/// Parse Rekordbox XML bytes with paths resolved relative to `xml_dir`.
pub fn parse_rekordbox_bytes(
    bytes: &[u8],
    xml_dir: &Path,
    playlist_name: Option<&str>,
) -> Result<RekordboxPlaylist> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut collection: HashMap<String, RawTrack> = HashMap::new();
    let mut playlists: Vec<NamedPlaylist> = Vec::new();

    let mut in_collection = false;
    let mut in_playlists = false;
    let mut current_track: Option<RawTrack> = None;
    let mut node_stack: Vec<(String, u8)> = Vec::new(); // (name, type)
    let mut current_pl_keys: Option<Vec<String>> = None;
    let mut current_pl_name: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag.as_str() {
                    "COLLECTION" => in_collection = true,
                    "PLAYLISTS" => in_playlists = true,
                    "TRACK" if in_collection && current_track.is_none() => {
                        let mut raw = RawTrack::default();
                        for a in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            let val = a
                                .unescape_value()
                                .map(|v| v.into_owned())
                                .unwrap_or_default();
                            match key.as_str() {
                                "TrackID" => raw.track_id = val,
                                "Location" => raw.location = val,
                                "AverageBpm" => raw.average_bpm = val.parse().ok(),
                                "Tonality" if !val.trim().is_empty() => {
                                    raw.tonality = Some(val);
                                }
                                _ => {}
                            }
                        }
                        current_track = Some(raw);
                    }
                    "TRACK" if in_playlists && current_pl_keys.is_some() => {
                        // Empty TRACK elements usually use Empty event; handle Start too.
                        for a in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            if key == "Key" {
                                let val = a
                                    .unescape_value()
                                    .map(|v| v.into_owned())
                                    .unwrap_or_default();
                                if let Some(keys) = current_pl_keys.as_mut() {
                                    keys.push(val);
                                }
                            }
                        }
                    }
                    "TEMPO" if current_track.is_some() => {
                        let mut bpm = None;
                        let mut inizio = None;
                        for a in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            let val = a
                                .unescape_value()
                                .map(|v| v.into_owned())
                                .unwrap_or_default();
                            match key.as_str() {
                                "Bpm" => bpm = val.parse().ok(),
                                "Inizio" => inizio = val.parse().ok(),
                                _ => {}
                            }
                        }
                        if let Some(t) = current_track.as_mut() {
                            if t.tempo_bpm.is_none() {
                                t.tempo_bpm = bpm;
                                t.tempo_inizio = inizio;
                            }
                        }
                    }
                    "POSITION_MARK" if current_track.is_some() => {
                        let mut mark_type: Option<i32> = None;
                        let mut start: Option<f64> = None;
                        for a in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            let val = a
                                .unescape_value()
                                .map(|v| v.into_owned())
                                .unwrap_or_default();
                            match key.as_str() {
                                "Type" => mark_type = val.parse().ok(),
                                "Start" => start = val.parse().ok(),
                                _ => {}
                            }
                        }
                        if let (Some(ty), Some(st), Some(t)) =
                            (mark_type, start, current_track.as_mut())
                        {
                            match ty {
                                1 if t.intro_end.is_none() => t.intro_end = Some(st),
                                2 if t.outro_start.is_none() => t.outro_start = Some(st),
                                _ => {}
                            }
                        }
                    }
                    "NODE" if in_playlists => {
                        let mut name = String::new();
                        let mut ty = 0u8;
                        for a in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            let val = a
                                .unescape_value()
                                .map(|v| v.into_owned())
                                .unwrap_or_default();
                            match key.as_str() {
                                "Name" => name = val,
                                "Type" => ty = val.parse().unwrap_or(0),
                                _ => {}
                            }
                        }
                        node_stack.push((name.clone(), ty));
                        if ty == 1 {
                            current_pl_name = Some(name);
                            current_pl_keys = Some(Vec::new());
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag.as_str() {
                    "TRACK" if in_collection => {
                        let mut raw = RawTrack::default();
                        for a in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            let val = a
                                .unescape_value()
                                .map(|v| v.into_owned())
                                .unwrap_or_default();
                            match key.as_str() {
                                "TrackID" => raw.track_id = val,
                                "Location" => raw.location = val,
                                "AverageBpm" => raw.average_bpm = val.parse().ok(),
                                "Tonality" if !val.trim().is_empty() => {
                                    raw.tonality = Some(val);
                                }
                                _ => {}
                            }
                        }
                        if !raw.track_id.is_empty() {
                            collection.insert(raw.track_id.clone(), raw);
                        }
                    }
                    "TRACK" if in_playlists && current_pl_keys.is_some() => {
                        for a in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            if key == "Key" {
                                let val = a
                                    .unescape_value()
                                    .map(|v| v.into_owned())
                                    .unwrap_or_default();
                                if let Some(keys) = current_pl_keys.as_mut() {
                                    keys.push(val);
                                }
                            }
                        }
                    }
                    "TEMPO" if current_track.is_some() => {
                        let mut bpm = None;
                        let mut inizio = None;
                        for a in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            let val = a
                                .unescape_value()
                                .map(|v| v.into_owned())
                                .unwrap_or_default();
                            match key.as_str() {
                                "Bpm" => bpm = val.parse().ok(),
                                "Inizio" => inizio = val.parse().ok(),
                                _ => {}
                            }
                        }
                        if let Some(t) = current_track.as_mut() {
                            if t.tempo_bpm.is_none() {
                                t.tempo_bpm = bpm;
                                t.tempo_inizio = inizio;
                            }
                        }
                    }
                    "POSITION_MARK" if current_track.is_some() => {
                        let mut mark_type: Option<i32> = None;
                        let mut start: Option<f64> = None;
                        for a in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            let val = a
                                .unescape_value()
                                .map(|v| v.into_owned())
                                .unwrap_or_default();
                            match key.as_str() {
                                "Type" => mark_type = val.parse().ok(),
                                "Start" => start = val.parse().ok(),
                                _ => {}
                            }
                        }
                        if let (Some(ty), Some(st), Some(t)) =
                            (mark_type, start, current_track.as_mut())
                        {
                            match ty {
                                1 if t.intro_end.is_none() => t.intro_end = Some(st),
                                2 if t.outro_start.is_none() => t.outro_start = Some(st),
                                _ => {}
                            }
                        }
                    }
                    "NODE" if in_playlists => {
                        let mut name = String::new();
                        let mut ty = 0u8;
                        for a in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                            let val = a
                                .unescape_value()
                                .map(|v| v.into_owned())
                                .unwrap_or_default();
                            match key.as_str() {
                                "Name" => name = val,
                                "Type" => ty = val.parse().unwrap_or(0),
                                _ => {}
                            }
                        }
                        if ty == 1 {
                            playlists.push(NamedPlaylist {
                                name,
                                keys: Vec::new(),
                            });
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag.as_str() {
                    "COLLECTION" => in_collection = false,
                    "PLAYLISTS" => in_playlists = false,
                    "TRACK" if current_track.is_some() => {
                        if let Some(raw) = current_track.take() {
                            if !raw.track_id.is_empty() {
                                collection.insert(raw.track_id.clone(), raw);
                            }
                        }
                    }
                    "NODE" if in_playlists => {
                        if let Some((_n, ty)) = node_stack.pop() {
                            if ty == 1 {
                                if let (Some(name), Some(keys)) =
                                    (current_pl_name.take(), current_pl_keys.take())
                                {
                                    playlists.push(NamedPlaylist { name, keys });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => bail!("XML parse error at {}: {e}", reader.buffer_position()),
            _ => {}
        }
        buf.clear();
    }

    if playlists.is_empty() {
        bail!("no playlist NODE (Type=1) found in Rekordbox XML");
    }

    let chosen = if let Some(want) = playlist_name {
        playlists
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(want))
            .with_context(|| format!("playlist named '{want}' not found"))?
    } else {
        &playlists[0]
    };

    let mut tracks = Vec::with_capacity(chosen.keys.len());
    for key in &chosen.keys {
        let raw = collection
            .get(key)
            .with_context(|| format!("playlist Key={key} missing from COLLECTION"))?;
        let path = resolve_location(&raw.location, xml_dir)
            .with_context(|| format!("resolve Location for TrackID={}", raw.track_id))?;
        let bpm = raw.tempo_bpm.or(raw.average_bpm).filter(|b| *b > 0.0);
        let key_str = raw
            .tonality
            .as_deref()
            .and_then(CamelotKey::parse_flexible)
            .map(|k| k.as_str());
        let beat_offset_secs = match (raw.tempo_inizio, bpm) {
            (Some(inizio), Some(b)) => {
                let beat = 60.0 / b.max(1.0) as f64;
                Some(inizio.rem_euclid(beat))
            }
            (Some(inizio), None) => Some(inizio.max(0.0)),
            _ => None,
        };
        tracks.push(RekordboxTrack {
            path,
            bpm,
            key: key_str,
            beat_offset_secs,
            intro_end: raw.intro_end,
            outro_start: raw.outro_start,
        });
    }

    Ok(RekordboxPlaylist {
        name: chosen.name.clone(),
        tracks,
    })
}

/// Resolve a Rekordbox `Location` URI or relative path to a local [`PathBuf`].
pub fn resolve_location(location: &str, xml_dir: &Path) -> Result<PathBuf> {
    let loc = location.trim();
    if loc.is_empty() {
        bail!("empty Location");
    }

    // Relative / absolute filesystem path (synthetic fixtures).
    if !loc.contains("://") {
        let p = PathBuf::from(loc);
        if p.is_absolute() {
            return Ok(p);
        }
        return Ok(xml_dir.join(p));
    }

    let without_scheme = loc
        .strip_prefix("file://localhost")
        .or_else(|| loc.strip_prefix("file://"))
        .unwrap_or(loc);

    let decoded = percent_decode(without_scheme);
    // Windows-style file:///C:/... leaves "/C:/..." — keep as-is on Unix for fixtures.
    Ok(PathBuf::from(decoded))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = hex_nibble(bytes[i + 1]);
            let l = hex_nibble(bytes[i + 2]);
            if let (Some(h), Some(l)) = (h, l) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_xml(a: &Path, b: &Path, c: &Path) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<DJ_PLAYLISTS Version="1.0.0">
  <PRODUCT Name="rekordbox" Version="6.0.0" Company="Pioneer DJ"/>
  <COLLECTION Entries="3">
    <TRACK TrackID="1" Name="loop_a" AverageBpm="128.00" Tonality="Am"
           Location="{a}">
      <TEMPO Inizio="0.125" Bpm="128.00" Metro="4/4" Battito="1"/>
      <POSITION_MARK Name="" Type="1" Start="0.5" Num="-1"/>
      <POSITION_MARK Name="" Type="2" Start="6.0" Num="-1"/>
    </TRACK>
    <TRACK TrackID="2" Name="loop_b" AverageBpm="128.00" Tonality="8A"
           Location="{b}">
      <TEMPO Inizio="0.0" Bpm="128.00" Metro="4/4" Battito="1"/>
    </TRACK>
    <TRACK TrackID="3" Name="loop_c" AverageBpm="140.00" Tonality="F#m"
           Location="{c}">
      <TEMPO Inizio="0.05" Bpm="140.00" Metro="4/4" Battito="1"/>
    </TRACK>
  </COLLECTION>
  <PLAYLISTS>
    <NODE Type="0" Name="ROOT" Count="1">
      <NODE Name="Gold Set" Type="1" KeyType="0" Entries="3">
        <TRACK Key="1"/>
        <TRACK Key="2"/>
        <TRACK Key="3"/>
      </NODE>
    </NODE>
  </PLAYLISTS>
</DJ_PLAYLISTS>
"#,
            a = a.display(),
            b = b.display(),
            c = c.display()
        )
    }

    #[test]
    fn resolve_file_uri_and_percent() {
        let p = resolve_location(
            "file://localhost/Users/admin/Music/My%20Track.wav",
            Path::new("/tmp"),
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("/Users/admin/Music/My Track.wav"));
    }

    #[test]
    fn resolve_relative_vs_xml_dir() {
        let p = resolve_location("../gold/loop_a_128.wav", Path::new("/tmp/rb")).unwrap();
        assert_eq!(p, PathBuf::from("/tmp/rb/../gold/loop_a_128.wav"));
    }

    #[test]
    fn parses_playlist_metadata() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/gold");
        let a = dir.join("loop_a_128.wav");
        let b = dir.join("loop_b_128.wav");
        let c = dir.join("loop_c_140.wav");
        let xml = sample_xml(&a, &b, &c);
        let pl = parse_rekordbox_bytes(xml.as_bytes(), Path::new("."), None).unwrap();
        assert_eq!(pl.name, "Gold Set");
        assert_eq!(pl.tracks.len(), 3);
        assert_eq!(pl.tracks[0].path, a);
        assert!((pl.tracks[0].bpm.unwrap() - 128.0).abs() < 0.01);
        assert_eq!(pl.tracks[0].key.as_deref(), Some("8A")); // Am → 8A
        assert!((pl.tracks[0].beat_offset_secs.unwrap() - 0.125).abs() < 1e-6);
        assert!((pl.tracks[0].intro_end.unwrap() - 0.5).abs() < 1e-6);
        assert!((pl.tracks[0].outro_start.unwrap() - 6.0).abs() < 1e-6);
        assert_eq!(pl.tracks[1].key.as_deref(), Some("8A"));
        assert_eq!(pl.tracks[2].path, c);
        assert!((pl.tracks[2].bpm.unwrap() - 140.0).abs() < 0.01);
    }

    #[test]
    fn committed_gold_playlist_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../testdata/rekordbox/gold_playlist.xml");
        let pl = parse_rekordbox_xml(&path, Some("Gold Set")).unwrap();
        assert_eq!(pl.tracks.len(), 3);
        assert!(pl.tracks[0].path.ends_with("loop_a_128.wav"));
        assert_eq!(pl.tracks[0].key.as_deref(), Some("8A"));
        assert!((pl.tracks[2].bpm.unwrap() - 140.0).abs() < 0.01);
    }

    #[test]
    fn playlist_name_selects_node() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<DJ_PLAYLISTS Version="1.0.0">
  <COLLECTION Entries="1">
    <TRACK TrackID="1" Name="x" AverageBpm="120" Tonality="8A" Location="/tmp/x.wav"/>
  </COLLECTION>
  <PLAYLISTS>
    <NODE Type="0" Name="ROOT" Count="2">
      <NODE Name="First" Type="1" KeyType="0" Entries="0"/>
      <NODE Name="Second" Type="1" KeyType="0" Entries="1">
        <TRACK Key="1"/>
      </NODE>
    </NODE>
  </PLAYLISTS>
</DJ_PLAYLISTS>"#;
        let pl = parse_rekordbox_bytes(xml.as_bytes(), Path::new("."), Some("Second")).unwrap();
        assert_eq!(pl.name, "Second");
        assert_eq!(pl.tracks.len(), 1);
    }
}
