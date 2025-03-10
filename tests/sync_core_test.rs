use tempfile::NamedTempFile;
use std::path::Path;
use std::future::Future;

use tts_sync::{
    sync::core::SyncCore,
    progress::ProgressTracker,
    tts::{TtsProvider, TtsSegment},
    error::Result,
};

// Мок для TtsProvider для тестирования
struct MockTtsProvider;

impl TtsProvider for MockTtsProvider {
    fn generate_speech(&self, _text: &str) -> impl Future<Output = Result<Vec<u8>>> {
        async move {
            // Возвращаем пустые данные для тестирования
            Ok(vec![0u8; 1000])
        }
    }
    
    fn generate_segment(&self, text: &str, target_duration: f64) -> impl Future<Output = Result<TtsSegment>> {
        async move {
            // Создаем тестовый сегмент с синусоидальным сигналом для более реалистичного тестирования
            let sample_rate = 44100;
            let num_samples = (sample_rate as f64 * target_duration) as usize;
            let mut samples = Vec::with_capacity(num_samples);
            
            // Генерируем синусоидальный сигнал с частотой 440 Гц
            for i in 0..num_samples {
                let t = i as f64 / sample_rate as f64;
                let amplitude = 0.5;
                let frequency = 440.0; // Гц
                let sample = amplitude * (2.0 * std::f64::consts::PI * frequency * t).sin();
                samples.push(sample as f32);
            }
            
            let audio_data = samples.iter().map(|&x| (x * 127.0) as u8).collect();
            
            Ok(TtsSegment {
                text: text.to_string(),
                audio_data,
                duration: Some(target_duration),
                target_duration,
                stretch_factor: None,
            })
        }
    }

    fn generate_speech_to_file<P: AsRef<Path>>(&self, _text: &str, path: P) -> impl Future<Output = Result<()>> {
        async move {
            // Для тестов просто создаем пустой файл
            std::fs::File::create(path)?;
            Ok(())
        }
    }
}

#[tokio::test]
async fn test_sync_core_creation() {
    let progress_tracker = ProgressTracker::new();
    let sync_core = SyncCore::new(progress_tracker, 44100, 1, true);
    
    // Проверяем, что ядро создано успешно
    assert!(sync_core.synchronize("nonexistent.vtt", 10.0, &MockTtsProvider).await.is_err());
}

#[tokio::test]
async fn test_sync_core_with_empty_subtitles() {
    let progress_tracker = ProgressTracker::new();
    let sync_core = SyncCore::new(progress_tracker, 44100, 1, true);
    
    // Создаем пустой файл с субтитрами
    let temp_file = NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), "").unwrap();
    
    // Проверяем, что синхронизация с пустыми субтитрами возвращает ошибку
    assert!(sync_core.synchronize(temp_file.path().to_str().unwrap(), 10.0, &MockTtsProvider).await.is_err());
}

#[tokio::test]
async fn test_sync_core_with_valid_subtitles() {
    let progress_tracker = ProgressTracker::new();
    let sync_core = SyncCore::new(progress_tracker, 44100, 1, true);
    
    // Создаем файл с валидными субтитрами
    let temp_file = NamedTempFile::new().unwrap();
    let vtt_content = "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nTest subtitle\n";
    std::fs::write(temp_file.path(), vtt_content).unwrap();
    
    // Проверяем, что синхронизация с валидными субтитрами возвращает аудио трек
    let result = sync_core.synchronize(temp_file.path().to_str().unwrap(), 10.0, &MockTtsProvider).await;
    assert!(result.is_ok());
    
    let audio_track = result.unwrap();
    assert_eq!(audio_track.sample_rate, 44100);
    assert_eq!(audio_track.channels, 1);
}
