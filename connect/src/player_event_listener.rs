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

pub(crate) struct SpircPlayerEventListener {
    play_request_id: Option<u64>,
    sender: mpsc::UnboundedSender<SpircPlayerEvent>,
}

impl SpircPlayerEventListener {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<SpircPlayerEvent>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (
            SpircPlayerEventListener {
                sender,
                play_request_id: None,
            },
            receiver,
        )
    }

    fn evaluate_events(&mut self, event: &PlayerEvent) -> Vec<SpircPlayerEvent> {
        let mut events = Vec::with_capacity(2);

        match event {
            PlayerEvent::TrackChanged { audio_item } => {
                events.push(SpircPlayerEvent::UpdateDuration(audio_item.duration_ms));
                events.push(SpircPlayerEvent::UpdateState);
                return events;
            }
            PlayerEvent::PlayRequestIdChanged { play_request_id } => {
                self.play_request_id = Some(*play_request_id);
                return events;
            }
            // we only process events if the play_request_id matches. If it doesn't, it is
            // an event that belongs to a previous track and only arrives now due to a race
            // condition. In this case we have updated the state already and don't want to
            // mess with it.
            _ if matches! {
                (event.get_play_request_id(), self.play_request_id),
                (Some(event_id), Some(current_id)) if event_id == current_id
            } => {}
            _ => return events,
        };

        match event {
            PlayerEvent::EndOfTrack { .. } => events.push(SpircPlayerEvent::HandleNext),
            PlayerEvent::Loading { .. } => {
                events.push(SpircPlayerEvent::UpdatePosition {
                    position: None,
                    as_nominal: false,
                    is_playing: false,
                });
            }
            PlayerEvent::Seeked { position_ms, .. } => {
                trace!("==> Seeked");
                events.push(SpircPlayerEvent::UpdatePosition {
                    position: Some(*position_ms),
                    as_nominal: false,
                    is_playing: false,
                });
            }
            PlayerEvent::Playing { position_ms, .. }
            | PlayerEvent::PositionCorrection { position_ms, .. } => {
                trace!("==> Playing");
                events.push(SpircPlayerEvent::UpdatePosition {
                    position: Some(*position_ms),
                    as_nominal: true,
                    is_playing: true,
                });
            }
            PlayerEvent::Paused {
                position_ms: new_position_ms,
                ..
            } => {
                trace!("==> Paused");
                events.push(SpircPlayerEvent::UpdatePosition {
                    position: Some(*new_position_ms),
                    as_nominal: true,
                    is_playing: false,
                });
            }
            PlayerEvent::Stopped { .. } => {
                trace!("==> Stopped");
                events.push(SpircPlayerEvent::UpdatePlayStatus(SpircPlayStatus::Stopped));
            }
            PlayerEvent::TimeToPreloadNextTrack { .. } => {
                events.push(SpircPlayerEvent::HandlePreloadNext);
                return events;
            }
            PlayerEvent::Unavailable { track_id, .. } => {
                events.push(SpircPlayerEvent::HandleUnavailable {
                    uri: track_id.to_uri(),
                });
            }
            _ => return events,
        }

        events.push(SpircPlayerEvent::UpdateState);
        events
    }
}

impl PlayerEventListener for SpircPlayerEventListener {
    fn is_valid(&self) -> bool {
        !self.sender.is_closed()
    }

    fn on_event(&mut self, event: &PlayerEvent)
    where
        Self: Sized,
    {
        for event in self.evaluate_events(event) {
            if let Err(why) = self.sender.send(event) {
                error!("couldn't send event to spirc: {why}")
            }
        }
    }
}
