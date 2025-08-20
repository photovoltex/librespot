use crate::model::SpircPlayStatus;
use librespot_core::Error;
use librespot_playback::player::{PlayerEvent, PlayerEventListener};
use tokio::sync::mpsc;

pub(super) enum SpircPlayerEvent {
    UpdateDuration(u32),
    UpdatePosition {
        position: Option<u32>,
        as_nominal: bool,
        is_playing: bool,
    },
    UpdatePlayStatus(SpircPlayStatus),
    UpdateState,
    HandleNext,
    HandlePreloadNext,
    HandleUnavailable {
        uri: Result<String, Error>,
    },
}

pub(crate) struct SpircPlayerEventForwarder {
    play_request_id: Option<u64>,
    sender: mpsc::UnboundedSender<SpircPlayerEvent>,
}

impl SpircPlayerEventForwarder {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<SpircPlayerEvent>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (
            SpircPlayerEventForwarder {
                sender,
                play_request_id: None,
            },
            receiver,
        )
    }

    fn evaluate_events(&mut self, event: &PlayerEvent) -> Option<Vec<SpircPlayerEvent>> {
        match event {
            PlayerEvent::TrackChanged { audio_item } => {
                return Some(vec![
                    SpircPlayerEvent::UpdateDuration(audio_item.duration_ms),
                    SpircPlayerEvent::UpdateState,
                ]);
            }
            PlayerEvent::PlayRequestIdChanged { play_request_id } => {
                self.play_request_id = Some(*play_request_id);
                return None;
            }
            // we only process events if the play_request_id matches. If it doesn't, it is
            // an event that belongs to a previous track and only arrives now due to a race
            // condition. In this case we have updated the state already and don't want to
            // mess with it.
            _ if matches! {
                (event.get_play_request_id(), self.play_request_id),
                (Some(event_id), Some(current_id)) if event_id == current_id
            } => {}
            _ => return None,
        };

        let events = match event {
            PlayerEvent::EndOfTrack { .. } => vec![SpircPlayerEvent::HandleNext],
            PlayerEvent::TimeToPreloadNextTrack { .. } => vec![SpircPlayerEvent::HandlePreloadNext],
            PlayerEvent::Unavailable { track_id, .. } => {
                vec![SpircPlayerEvent::HandleUnavailable {
                    uri: track_id.to_uri(),
                }]
            }
            PlayerEvent::Loading { .. } => {
                vec![
                    SpircPlayerEvent::UpdatePosition {
                        position: None,
                        as_nominal: false,
                        is_playing: false,
                    },
                    SpircPlayerEvent::UpdateState,
                ]
            }
            PlayerEvent::Seeked { position_ms, .. } => {
                trace!("==> Seeked");
                vec![
                    SpircPlayerEvent::UpdatePosition {
                        position: Some(*position_ms),
                        as_nominal: false,
                        is_playing: false,
                    },
                    SpircPlayerEvent::UpdateState,
                ]
            }
            PlayerEvent::Playing { position_ms, .. }
            | PlayerEvent::PositionCorrection { position_ms, .. } => {
                trace!("==> Playing");
                vec![
                    SpircPlayerEvent::UpdatePosition {
                        position: Some(*position_ms),
                        as_nominal: true,
                        is_playing: true,
                    },
                    SpircPlayerEvent::UpdateState,
                ]
            }
            PlayerEvent::Paused {
                position_ms: new_position_ms,
                ..
            } => {
                trace!("==> Paused");
                vec![
                    SpircPlayerEvent::UpdatePosition {
                        position: Some(*new_position_ms),
                        as_nominal: true,
                        is_playing: false,
                    },
                    SpircPlayerEvent::UpdateState,
                ]
            }
            PlayerEvent::Stopped { .. } => {
                trace!("==> Stopped");
                vec![
                    SpircPlayerEvent::UpdatePlayStatus(SpircPlayStatus::Stopped),
                    SpircPlayerEvent::UpdateState,
                ]
            }
            _ => return None,
        };

        Some(events)
    }
}

impl PlayerEventListener for SpircPlayerEventForwarder {
    fn is_valid(&self) -> bool {
        !self.sender.is_closed()
    }

    fn on_event(&mut self, event: &PlayerEvent)
    where
        Self: Sized,
    {
        if let Some(events) = self.evaluate_events(event) {
            for event in events {
                if let Err(why) = self.sender.send(event) {
                    error!("couldn't send event to spirc: {why}")
                }
            }
        }
    }
}
