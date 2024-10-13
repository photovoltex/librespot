use crate::deserialize_with::*;
use librespot_protocol::player::{
    Context, ContextPlayerOptionOverrides, PlayOrigin, ProvidedTrack, TransferState,
};
use serde::Deserialize;
use serde_json::Value;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Deserialize)]
pub struct Request {
    pub message_id: u32,
    // todo: did only send target_alias_id: null so far, maybe we just ignore it, will see
    // pub target_alias_id: Option<()>,
    pub sent_by_device_id: String,
    pub command: RequestCommand,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "endpoint", rename_all = "snake_case")]
pub enum RequestCommand {
    Transfer(TransferCommand),
    #[serde(deserialize_with = "boxed")]
    Play(Box<PlayCommand>),
    Pause(PauseCommand),
    SeekTo(SeekToCommand),
    SkipNext(SkipNextCommand),
    SetShufflingContext(SetShufflingCommand),
    AddToQueue(AddToQueueCommand),
    SetQueue(SetQueueCommand),
    // commands that don't send any context (at least not usually...)
    SkipPrev(GenericCommand),
    Resume(GenericCommand),
    // catch unknown commands, so that we can implement them later
    #[serde(untagged)]
    Unknown(Value),
}

impl Display for RequestCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "endpoint: {}",
            match self {
                RequestCommand::Transfer(_) => "transfer",
                RequestCommand::Play(_) => "play",
                RequestCommand::Pause(_) => "pause",
                RequestCommand::SeekTo(_) => "seek_to",
                RequestCommand::SetShufflingContext(_) => "set_shuffling_context",
                RequestCommand::AddToQueue(_) => "add_to_queue",
                RequestCommand::SetQueue(_) => "set_queue",
                RequestCommand::SkipNext(_) => "skip_next",
                RequestCommand::SkipPrev(_) => "skip_prev",
                RequestCommand::Resume(_) => "resume",
                RequestCommand::Unknown(json) => {
                    json.as_object()
                        .and_then(|obj| obj.get("endpoint").map(|v| v.as_str()))
                        .flatten()
                        .unwrap_or("???")
                }
            }
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct TransferCommand {
    #[serde(default, deserialize_with = "base64_proto")]
    pub data: Option<TransferState>,
    pub options: TransferOptions,
    pub from_device_identifier: String,
    pub logging_params: LoggingParams,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlayCommand {
    #[serde(deserialize_with = "json_proto")]
    pub context: Context,
    #[serde(deserialize_with = "json_proto")]
    pub play_origin: PlayOrigin,
    pub options: PlayOptions,
    pub logging_params: LoggingParams,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PauseCommand {
    // does send options with it, but seems to be empty, investigate which options are send here
    pub logging_params: LoggingParams,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SeekToCommand {
    pub value: u32,
    pub position: u32,
    pub logging_params: LoggingParams,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SkipNextCommand {
    #[serde(default, deserialize_with = "option_json_proto")]
    pub track: Option<ProvidedTrack>,
    pub logging_params: LoggingParams,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SetShufflingCommand {
    pub value: bool,
    pub logging_params: LoggingParams,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AddToQueueCommand {
    #[serde(deserialize_with = "json_proto")]
    pub track: ProvidedTrack,
    pub logging_params: LoggingParams,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SetQueueCommand {
    #[serde(deserialize_with = "vec_json_proto")]
    pub next_tracks: Vec<ProvidedTrack>,
    #[serde(deserialize_with = "vec_json_proto")]
    pub prev_tracks: Vec<ProvidedTrack>,
    // this queue revision is actually the last revision, so using it will not update the web ui
    // might be that internally they use the last revision to create the next revision
    pub queue_revision: String,
    pub logging_params: LoggingParams,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GenericCommand {
    pub logging_params: LoggingParams,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TransferOptions {
    pub restore_paused: String,
    pub restore_position: String,
    pub restore_track: String,
    pub retain_session: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlayOptions {
    pub skip_to: SkipTo,
    #[serde(default, deserialize_with = "option_json_proto")]
    pub player_option_overrides: Option<ContextPlayerOptionOverrides>,
    pub license: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SkipTo {
    pub track_uid: String,
    pub track_uri: String,
    pub track_index: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoggingParams {
    pub interaction_ids: Option<Vec<String>>,
    pub device_identifier: Option<String>,
    pub command_initiated_time: Option<i64>,
    pub page_instance_ids: Option<Vec<String>>,
    pub command_id: Option<String>,
}