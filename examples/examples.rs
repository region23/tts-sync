use tts_sync::{Result, tts::{TtsProvider, TtsSegment}, vtt::Subtitle};

/// Создает пример TTS сегмента для тестирования
pub async fn create_example_tts_segment<P: TtsProvider>(
    provider: &P,
    subtitle: &Subtitle,
) -> Result<TtsSegment> {
    provider.generate_segment(&subtitle.text, subtitle.duration()).await
}

/// Создает примеры использования библиотеки
pub async fn create_examples() -> Result<()> {
    // Примеры будут добавлены позже
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    create_examples().await
}
