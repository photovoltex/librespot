use librespot_core::SpotifyId;
use librespot_metadata::audio::AudioItem;

#[derive(Debug, Clone)]
pub enum PlayerEvent<'ev> {
    // Play request id changed
    PlayRequestIdChanged {
        play_request_id: u64,
    },
    // Fired when the player is stopped (e.g. by issuing a "stop" command to the player).
    Stopped {
        play_request_id: u64,
        track_id: SpotifyId,
    },
    // The player is delayed by loading a track.
    Loading {
        play_request_id: u64,
        track_id: SpotifyId,
        position_ms: u32,
    },
    // The player is preloading a track.
    Preloading {
        track_id: &'ev SpotifyId,
    },
    // The player is playing a track.
    // This event is issued at the start of playback of whenever the position must be communicated
    // because it is out of sync. This includes:
    // start of a track
    // un-pausing
    // after a seek
    // after a buffer-underrun
    Playing {
        play_request_id: u64,
        track_id: &'ev SpotifyId,
        position_ms: u32,
    },
    // The player entered a paused state.
    Paused {
        play_request_id: u64,
        track_id: &'ev SpotifyId,
        position_ms: u32,
    },
    // The player thinks it's a good idea to issue a preload command for the next track now.
    // This event is intended for use within spirc.
    TimeToPreloadNextTrack {
        play_request_id: u64,
        track_id: &'ev SpotifyId,
    },
    // The player reached the end of a track.
    // This event is intended for use within spirc. Spirc will respond by issuing another command.
    EndOfTrack {
        play_request_id: u64,
        track_id: &'ev SpotifyId,
    },
    // The player was unable to load the requested track.
    Unavailable {
        play_request_id: u64,
        track_id: &'ev SpotifyId,
    },
    // The mixer volume was set to a new level.
    VolumeChanged {
        volume: u16,
    },
    PositionCorrection {
        play_request_id: u64,
        track_id: &'ev SpotifyId,
        position_ms: u32,
    },
    /// Requires `PlayerConfig::position_update_interval` to be set to [Some].
    /// Once set this event will be sent periodically while playing the track to inform about the
    /// current playback position
    PositionChanged {
        play_request_id: u64,
        track_id: &'ev SpotifyId,
        position_ms: u32,
    },
    Seeked {
        play_request_id: u64,
        track_id: &'ev SpotifyId,
        position_ms: u32,
    },
    TrackChanged {
        audio_item: &'ev AudioItem,
    },
    SessionConnected {
        connection_id: String,
        user_name: String,
    },
    SessionDisconnected {
        connection_id: String,
        user_name: String,
    },
    SessionClientChanged {
        client_id: String,
        client_name: String,
        client_brand_name: String,
        client_model_name: String,
    },
    ShuffleChanged {
        shuffle: bool,
    },
    RepeatChanged {
        context: bool,
        track: bool,
    },
    AutoPlayChanged {
        auto_play: bool,
    },
    FilterExplicitContentChanged {
        filter: bool,
    },
}

impl PlayerEvent<'_> {
    pub fn get_play_request_id(&self) -> Option<u64> {
        use PlayerEvent::*;
        match self {
            Loading {
                play_request_id, ..
            }
            | Unavailable {
                play_request_id, ..
            }
            | Playing {
                play_request_id, ..
            }
            | TimeToPreloadNextTrack {
                play_request_id, ..
            }
            | EndOfTrack {
                play_request_id, ..
            }
            | Paused {
                play_request_id, ..
            }
            | Stopped {
                play_request_id, ..
            }
            | PositionCorrection {
                play_request_id, ..
            }
            | Seeked {
                play_request_id, ..
            } => Some(*play_request_id),
            _ => None,
        }
    }
}

pub trait PlayerEventListener {
    /// An option to invalidate the listener.
    ///
    /// The check is done before and after invoking [PlayerEventListener::on_event]. So that
    /// [PlayerEventListener::on_event] is always called in a valid state and dropped as soon
    /// as it becomes invalid.
    fn is_valid(&self) -> bool;

    /// Handle the event that is given by the player
    ///
    /// The listener is invoked inside the player thread, so any blocking action
    /// will impact the player performance.
    ///
    /// This could be improved later on by changing how the packets are handled, but currently is what it is.
    fn on_event(&mut self, event: &PlayerEvent);
}
