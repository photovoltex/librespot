use crate::player::player_state::{LoadingState, PausedState, PlayerState, PlayingState};
use crate::player::{PlayerEvent, PlayerInternal, PlayerLoadedTrackData, PlayerPreload};
use std::ops::{Deref, DerefMut};
use std::process::exit;
use std::time::{Duration, Instant};

const PRELOAD_NEXT_TRACK_BEFORE_END_DURATION_MS: u32 = 30000;

pub(super) struct PlayerTask(pub PlayerInternal);

impl Deref for PlayerTask {
    type Target = PlayerInternal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PlayerTask {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PlayerTask {
    pub async fn run(mut self) {
        loop {
            let mut state = self.state.take();
            let mut preload = self.preload.take();

            tokio::select! {
                cmd = self.commands.recv() => {
                    self.state = state;
                    self.preload = preload;

                    match cmd {
                        None => break,
                        Some(cmd) => if let Err(e) = self.handle_command(cmd) {
                            error!("Error handling command: {e}");
                        }
                    }
                }

                loaded = async {
                    state.as_mut().map(PlayerState::loader)??.await.ok()
                }, if matches!(state, Some(PlayerState::Loading { .. })) => {
                    self.handle_loading(state, loaded).await;
                }
                preloaded = async {
                    preload.as_mut().map(PlayerPreload::loader)??.await.ok()
                }, if matches!(preload, Some(PlayerPreload::Loading { .. })) => {
                    self.handle_preload_loading(preload, preloaded);
                }

                _ = async { }, if matches!(state, Some(PlayerState::Playing { .. })) => {
                    self.state = state;
                    self.handle_playing();
                }

                _ = async { }, if Self::suggest_preload(state.as_ref()) => {
                    self.state = state;
                    self.handle_preload_suggestion();
                },
                else => break
            }
        }
    }

    async fn handle_loading(
        &mut self,
        state: Option<PlayerState>,
        load_result: Option<PlayerLoadedTrackData>,
    ) {
        let LoadingState {
            track_id,
            play_request_id,
            start_playback,
            ..
        } = match state {
            Some(PlayerState::Loading(state)) => state,
            _ => {
                error!("Invalid PlayerState to expected");
                exit(1);
            }
        };

        match load_result {
            Some(loaded_track) => {
                self.start_playback(track_id, play_request_id, loaded_track, start_playback);

                if self.state.is_none() {
                    error!("The state wasn't changed by start_playback()");
                    exit(1);
                }
            }
            None => {
                error!("Skipping to next track, unable to load track <{track_id:?}>");
                self.send_event(PlayerEvent::Unavailable {
                    track_id,
                    play_request_id,
                })
            }
        }
    }

    fn handle_preload_loading(
        &mut self,
        preload: Option<PlayerPreload>,
        preload_result: Option<PlayerLoadedTrackData>,
    ) {
        let track_id = match preload {
            Some(PlayerPreload::Loading { track_id, .. }) => track_id,
            _ => {
                error!("Invalid PlayerPreloadState to expected");
                exit(1);
            }
        };

        match preload_result {
            Some(loaded_track) => {
                self.send_event(PlayerEvent::Preloading { track_id });
                self.preload = Some(PlayerPreload::Ready {
                    track_id,
                    loaded_track: Box::new(loaded_track),
                });
            }
            None => {
                debug!("Unable to preload {track_id:?}");
                self.preload = None;

                // Let Spirc know that the track was unavailable.
                if let Some(PlayerState::Playing(PlayingState {
                    play_request_id, ..
                }))
                | Some(PlayerState::Paused(PausedState {
                    play_request_id, ..
                })) = self.state
                {
                    self.send_event(PlayerEvent::Unavailable {
                        track_id,
                        play_request_id,
                    });
                }
            }
        }
    }

    fn handle_playing(&mut self) {
        self.ensure_sink_running();

        let passthrough = self.config.passthrough;
        let mut state = match self.state.take() {
            Some(PlayerState::Playing(state)) => state,
            _ => {
                error!("PlayerInternal poll: Invalid PlayerState");
                exit(1);
            }
        };

        let result = match state.decoder.next_packet() {
            Err(e) => {
                let track_id = state.track_id;
                error!(
                    "Skipping to next track, unable to get next packet for track <{track_id:?}>: {e:?}"
                );
                self.send_event(PlayerEvent::EndOfTrack {
                    play_request_id: state.play_request_id,
                    track_id,
                });
                self.state = Some(PlayerState::Stopped);
                return;
            }
            Ok(result) => result,
        };

        if let Some((ref packet_position, ref packet)) = result {
            let new_stream_position_ms = packet_position.position_ms;

            let expected_position_ms = state.stream_position_ms;
            state.stream_position_ms = new_stream_position_ms;

            if !passthrough {
                if let Err(e) = packet.samples() {
                    let track_id = state.track_id;
                    error!(
                        "Skipping to next track, unable to decode samples for track <{track_id:?}>: {e:?}"
                    );
                    self.send_event(PlayerEvent::EndOfTrack {
                        play_request_id: state.play_request_id,
                        track_id,
                    });
                    self.state = Some(PlayerState::Stopped);
                    return;
                }

                let new_stream_position = Duration::from_millis(new_stream_position_ms as u64);

                let now = Instant::now();

                // Only notify if we're skipped some packets *or* we are behind.
                // If we're ahead it's probably due to a buffer of the backend,
                // and we're actually in time.
                let notify_about_position = match state.reported_nominal_start_time {
                    None => true,
                    Some(reported_nominal_start_time) => {
                        let mut notify = false;

                        if packet_position.skipped {
                            if let Some(ahead) = new_stream_position
                                .checked_sub(Duration::from_millis(expected_position_ms as u64))
                            {
                                notify |= ahead >= Duration::from_secs(1)
                            }
                        }

                        if let Some(lag) = now.checked_duration_since(reported_nominal_start_time) {
                            if let Some(lag) = lag.checked_sub(new_stream_position) {
                                notify |= lag >= Duration::from_secs(1)
                            }
                        }

                        notify
                    }
                };

                if notify_about_position {
                    state.reported_nominal_start_time = now.checked_sub(new_stream_position);
                    self.send_event(PlayerEvent::PositionCorrection {
                        play_request_id: state.play_request_id,
                        track_id: state.track_id,
                        position_ms: new_stream_position_ms,
                    });
                }

                if let Some(interval) = self.config.position_update_interval {
                    let last_progress_update_since_ms =
                        now.duration_since(self.last_progress_update);

                    if last_progress_update_since_ms > interval {
                        self.last_progress_update = now;
                        self.send_event(PlayerEvent::PositionChanged {
                            play_request_id: state.play_request_id,
                            track_id: state.track_id,
                            position_ms: new_stream_position_ms,
                        });
                    }
                }
            }
        }

        let normalisation_factor = state.normalisation_factor;
        self.state = Some(PlayerState::Playing(state));

        self.handle_packet(result, normalisation_factor);
    }

    fn suggest_preload(state: Option<&PlayerState>) -> bool {
        match state {
            Some(PlayerState::Playing(PlayingState {
                duration_ms,
                stream_position_ms,
                stream_loader_controller,
                suggested_to_preload_next_track,
                ..
            }))
            | Some(PlayerState::Paused(PausedState {
                duration_ms,
                stream_position_ms,
                stream_loader_controller,
                suggested_to_preload_next_track,
                ..
            })) => {
                (!*suggested_to_preload_next_track)
                    && ((*duration_ms as i64 - *stream_position_ms as i64)
                        < PRELOAD_NEXT_TRACK_BEFORE_END_DURATION_MS as i64)
                    && stream_loader_controller.range_to_end_available()
            }
            _ => false,
        }
    }

    fn handle_preload_suggestion(&mut self) {
        let event = match self.state.as_mut() {
            Some(PlayerState::Playing(PlayingState {
                track_id,
                play_request_id,
                suggested_to_preload_next_track,
                ..
            }))
            | Some(PlayerState::Paused(PausedState {
                track_id,
                play_request_id,
                suggested_to_preload_next_track,
                ..
            })) => {
                *suggested_to_preload_next_track = true;
                Some(PlayerEvent::TimeToPreloadNextTrack {
                    track_id: *track_id,
                    play_request_id: *play_request_id,
                })
            }
            _ => None,
        };

        // we can't send the event inside the match because we already have a mutable access
        if let Some(evt) = event {
            self.send_event(evt);
        }
    }
}
