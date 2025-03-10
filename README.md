# TTS-Sync

[![Crates.io](https://img.shields.io/crates/v/tts-sync.svg)](https://crates.io/crates/tts-sync)
[![Documentation](https://docs.rs/tts-sync/badge.svg)](https://docs.rs/tts-sync)
[![License](https://img.shields.io/crates/l/tts-sync.svg)](LICENSE)

Библиотека для синхронизации синтеза речи (TTS) с видео и субтитрами, с продвинутой обработкой аудио и настройкой темпа.

## Обновление до версии 0.1.2 🚀

В версии 0.1.2 исправлены критические баги:
- 🔊 Исправлена проблема пустых аудиофайлов - теперь аудио корректно сохраняется и воспроизводится
- 🔄 Добавлено кэширование TTS-сегментов для предотвращения дублирования запросов
- 📊 Улучшено логирование для более простой отладки
- 🔧 Добавлена корректная поддержка MP3, WAV и OGG форматов с использованием ffmpeg

**Полный список изменений можно найти в [CHANGELOG.md](CHANGELOG.md)**

## Особенности

- 🔊 Генерация высококачественного TTS через OpenAI API
- 🔄 Синхронизация аудио с субтитрами и видео
- ⏱️ Адаптивная настройка темпа для точного совпадения с длительностью субтитров
- 🎛️ Продвинутая обработка аудио:
  - Нормализация громкости
  - Компрессия динамического диапазона
  - Эквализация
- 📝 Поддержка VTT субтитров
- 🎯 Сохранение естественного звучания речи при изменении темпа
- 🧪 Настраиваемые параметры для разных сценариев использования

## Установка

Добавьте библиотеку в ваш проект:

```toml
[dependencies]
tts-sync = "0.1.2"
```

## Требования системы

- **ffmpeg**: Для оптимального качества MP3 и OGG файлов рекомендуется установить ffmpeg
- **Rust 1.56+**: Для компиляции библиотеки
- **OpenAI API ключ**: Для доступа к OpenAI TTS API

### Установка ffmpeg
- **macOS**: `brew install ffmpeg`
- **Ubuntu/Debian**: `sudo apt install ffmpeg`
- **Windows**: Скачайте с [официального сайта](https://ffmpeg.org/download.html) или установите через `choco install ffmpeg`

## Использование

## Features

- **Precise Timing Alignment**: Synchronize translated subtitles with dubbed audio
- **Adaptive Tempo Adjustment**: Intelligently adjust speech tempo while preserving natural pauses
- **Advanced Audio Processing**: Apply compression, equalization, and volume normalization
- **Multiple Tempo Algorithms**: Choose between quality (Sinc), balanced (FIR), or speed (Linear)
- **Progress Tracking**: Asynchronous processing with detailed progress reporting
- **Language Agnostic**: Support for any source and destination languages
- **OpenAI TTS Integration**: High-quality speech generation using OpenAI's TTS API

## Quick Start

```rust
use tts_sync::{TtsSync, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Get API key from environment variables
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    
    // Create TtsSync instance with default settings and progress tracking
    let tts_sync = TtsSync::default()
        .with_progress_callback(Box::new(|progress, status| {
            println!("Progress: {:.1}%, Status: {}", progress, status);
            Ok(())
        }));
    
    // Synchronize TTS with video and subtitles
    let output_path = tts_sync.synchronize(
        "subtitles.vtt",
        120.0, // video duration in seconds
        &api_key
    ).await?;
    
    println!("Audio saved to: {}", output_path);
    
    Ok(())
}
```

## Advanced Usage

### Custom Settings

```rust
use tts_sync::{TtsSync, SyncOptions, AudioFormat, TempoAlgorithm, Result};
use log::LevelFilter;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    
    // Create custom settings
    let options = SyncOptions {
        voice: "nova".to_string(),
        output_format: AudioFormat::Wav,
        sample_rate: 48000,
        max_segment_duration: 5.0,
        normalize_volume: true,
        apply_compression: true,
        apply_equalization: true,
        tempo_algorithm: TempoAlgorithm::Sinc,
        preserve_pauses: true,
        
        // Compression parameters
        compression_threshold: -20.0,
        compression_ratio: 4.0,
        compression_attack: 10.0,
        compression_release: 100.0,
        compression_makeup_gain: 6.0,
        
        // Equalization parameters
        eq_low_gain: 3.0,
        eq_mid_gain: 0.0,
        eq_high_gain: 2.0,
        eq_low_freq: 300.0,
        eq_high_freq: 3000.0,
        
        // Volume normalization target
        normalization_target_db: -3.0,
        
        log_level: LevelFilter::Debug,
    };
    
    // Create TtsSync with custom settings
    let tts_sync = TtsSync::new(options);
    
    // Synchronize TTS with video and subtitles
    let output_path = tts_sync.synchronize(
        "subtitles.vtt",
        120.0,
        &api_key
    ).await?;
    
    println!("Audio saved to: {}", output_path);
    
    Ok(())
}
```

### Fluent Interface

```rust
use tts_sync::{TtsSync, TempoAlgorithm, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    
    // Create TtsSync with fluent interface
    let tts_sync = TtsSync::default()
        .with_tempo_algorithm(TempoAlgorithm::Sinc)
        .with_compression(true)
        .with_equalization(true)
        .with_volume_normalization(true)
        .with_preserve_pauses(true);
    
    // Synchronize TTS with video and subtitles
    let output_path = tts_sync.synchronize(
        "subtitles.vtt",
        120.0,
        &api_key
    ).await?;
    
    println!("Audio saved to: {}", output_path);
    
    Ok(())
}
```

## Documentation

For detailed documentation, see the [User Guide](guide.md) or the [API Documentation](https://docs.rs/tts-sync).

## Integration with Tauri and Vue 3

TTS-Sync can be easily integrated with Tauri and Vue 3 applications. See the [Integration Guide](guide.md#интеграция-с-tauri-и-vue-3) for details.

## License

This project is licensed under either of:

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
