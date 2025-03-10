use crate::error::Result;
use std::path::Path;
use std::future::Future;

mod openai;

pub use openai::{
    OpenAiTts, TtsOptions, TtsSegment, 
    OpenAiVoice, OpenAiTtsModel, OpenAiAudioFormat
};

/// Интерфейс для TTS провайдеров
pub trait TtsProvider: Send + Sync {
    /// Генерирует TTS для текста
    fn generate_speech(&self, text: &str) -> impl Future<Output = Result<Vec<u8>>>;
    
    /// Генерирует TTS для текста и сохраняет в файл
    fn generate_speech_to_file<P: AsRef<Path>>(&self, text: &str, path: P) -> impl Future<Output = Result<()>>;
    
    /// Генерирует TTS для сегмента субтитров
    fn generate_segment(&self, text: &str, target_duration: f64) -> impl Future<Output = Result<TtsSegment>>;
}

impl TtsProvider for OpenAiTts {
    fn generate_speech(&self, text: &str) -> impl Future<Output = Result<Vec<u8>>> {
        self.generate_speech(text)
    }
    
    fn generate_speech_to_file<P: AsRef<Path>>(&self, text: &str, path: P) -> impl Future<Output = Result<()>> {
        self.generate_speech_to_file(text, path)
    }
    
    fn generate_segment(&self, text: &str, target_duration: f64) -> impl Future<Output = Result<TtsSegment>> {
        self.generate_segment(text, target_duration)
    }
}