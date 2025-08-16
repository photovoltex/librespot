use crate::player::{Decoder, NormalisationData, PlayerLoadedTrackData, TrackLoader};
use librespot_audio::StreamLoaderController;
use librespot_core::SpotifyId;
use librespot_metadata::audio::AudioItem;
use std::fmt;
use std::time::{Duration, Instant};

pub(super) enum PlayerState {
    Stopped,
    Loading(LoadingState),
    Paused(PausedState),
    Playing(PlayingState),
    EndOfTrack(EndOfTrackState),
}

pub(super) struct LoadingState {
    pub track_id: SpotifyId,
    pub play_request_id: u64,
    pub start_playback: bool,
    pub loader: TrackLoader,
}

pub(super) struct PausedState {
    pub track_id: SpotifyId,
    pub play_request_id: u64,
    pub decoder: Decoder,
    pub audio_item: AudioItem,
    pub normalisation_data: NormalisationData,
    pub normalisation_factor: f64,
    pub stream_loader_controller: StreamLoaderController,
    pub bytes_per_second: usize,
    pub duration_ms: u32,
    pub stream_position_ms: u32,
    pub suggested_to_preload_next_track: bool,
    pub is_explicit: bool,
}

pub(super) struct PlayingState {
    pub track_id: SpotifyId,
    pub play_request_id: u64,
    pub decoder: Decoder,
    pub normalisation_data: NormalisationData,
    pub audio_item: AudioItem,
    pub normalisation_factor: f64,
    pub stream_loader_controller: StreamLoaderController,
    pub bytes_per_second: usize,
    pub duration_ms: u32,
    pub stream_position_ms: u32,
    pub reported_nominal_start_time: Option<Instant>,
    pub suggested_to_preload_next_track: bool,
    pub is_explicit: bool,
}

pub(super) struct EndOfTrackState {
    pub track_id: SpotifyId,
    pub play_request_id: u64,
    pub loaded_track: PlayerLoadedTrackData,
}

impl fmt::Debug for PlayerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PlayerState::*;
        match self {
            Stopped => f.debug_struct("Stopped").finish(),
            Loading(LoadingState {
                track_id,
                play_request_id,
                ..
            }) => f
                .debug_struct("Loading")
                .field("track_id", &track_id)
                .field("play_request_id", &play_request_id)
                .finish(),
            Paused(PausedState {
                track_id,
                play_request_id,
                ..
            }) => f
                .debug_struct("Paused")
                .field("track_id", &track_id)
                .field("play_request_id", &play_request_id)
                .finish(),
            Playing(PlayingState {
                track_id,
                play_request_id,
                ..
            }) => f
                .debug_struct("Playing")
                .field("track_id", &track_id)
                .field("play_request_id", &play_request_id)
                .finish(),
            EndOfTrack(EndOfTrackState {
                track_id,
                play_request_id,
                ..
            }) => f
                .debug_struct("EndOfTrack")
                .field("track_id", &track_id)
                .field("play_request_id", &play_request_id)
                .finish(),
        }
    }
}

impl From<PlayingState> for EndOfTrackState {
    fn from(value: PlayingState) -> Self {
        let PlayingState {
            track_id,
            play_request_id,
            decoder,
            duration_ms,
            bytes_per_second,
            normalisation_data,
            stream_loader_controller,
            stream_position_ms,
            is_explicit,
            audio_item,
            ..
        } = value;

        EndOfTrackState {
            track_id,
            play_request_id,
            loaded_track: PlayerLoadedTrackData {
                decoder,
                normalisation_data,
                stream_loader_controller,
                audio_item,
                bytes_per_second,
                duration_ms,
                stream_position_ms,
                is_explicit,
            },
        }
    }
}

impl From<PausedState> for PlayingState {
    fn from(value: PausedState) -> Self {
        let PausedState {
            track_id,
            play_request_id,
            decoder,
            audio_item,
            normalisation_data,
            normalisation_factor,
            stream_loader_controller,
            duration_ms,
            bytes_per_second,
            stream_position_ms,
            suggested_to_preload_next_track,
            is_explicit,
        } = value;

        PlayingState {
            track_id,
            play_request_id,
            decoder,
            audio_item,
            normalisation_data,
            normalisation_factor,
            stream_loader_controller,
            duration_ms,
            bytes_per_second,
            stream_position_ms,
            reported_nominal_start_time: Instant::now()
                .checked_sub(Duration::from_millis(stream_position_ms as u64)),
            suggested_to_preload_next_track,
            is_explicit,
        }
    }
}

impl From<PlayingState> for PausedState {
    fn from(value: PlayingState) -> Self {
        let PlayingState {
            track_id,
            play_request_id,
            decoder,
            audio_item,
            normalisation_data,
            normalisation_factor,
            stream_loader_controller,
            duration_ms,
            bytes_per_second,
            stream_position_ms,
            suggested_to_preload_next_track,
            is_explicit,
            ..
        } = value;

        PausedState {
            track_id,
            play_request_id,
            decoder,
            audio_item,
            normalisation_data,
            normalisation_factor,
            stream_loader_controller,
            duration_ms,
            bytes_per_second,
            stream_position_ms,
            suggested_to_preload_next_track,
            is_explicit,
        }
    }
}

impl PlayerState {
    pub fn loader(&mut self) -> Option<&mut TrackLoader> {
        match self {
            PlayerState::Loading(loading) => Some(&mut loading.loader),
            _ => None,
        }
    }
}
