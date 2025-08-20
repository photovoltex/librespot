use crate::player::{PlayerEvent, PlayerEventListener};
use tokio::sync::oneshot;

pub(super) struct AwaitEndOfTrack(Option<oneshot::Sender<()>>);
impl AwaitEndOfTrack {
    pub fn new() -> (Self, oneshot::Receiver<()>) {
        let (sender, receiver) = oneshot::channel();
        (Self(Some(sender)), receiver)
    }
}

impl PlayerEventListener for AwaitEndOfTrack {
    fn is_valid(&self) -> bool {
        self.0.is_some()
    }

    fn on_event(&mut self, event: &PlayerEvent)
    where
        Self: Sized,
    {
        if matches!(
            event,
            PlayerEvent::EndOfTrack { .. } | PlayerEvent::Stopped { .. }
        ) {
            if let Some(oneshot) = self.0.take() {
                _ = oneshot.send(())
            }
        }
    }
}
