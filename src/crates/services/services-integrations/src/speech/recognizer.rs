use super::types::{SpeechRecognizerKind, SpeechTranscribeRequest, SpeechTranscriptionResult};
use super::AlexResult;
use async_trait::async_trait;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(super) struct SpeechRecognizerWarmupRequest {
    pub model_dir: PathBuf,
    pub recognizer: SpeechRecognizerKind,
    pub language: String,
}

#[async_trait]
pub(super) trait SpeechRecognizer: Send + Sync {
    async fn warmup(&self, request: SpeechRecognizerWarmupRequest) -> AlexResult<()>;

    async fn unload(&self) -> AlexResult<()>;

    async fn transcribe(
        &self,
        request: SpeechTranscribeRequest,
    ) -> AlexResult<SpeechTranscriptionResult>;
}
