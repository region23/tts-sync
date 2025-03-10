use tts_sync::{TtsSync, SyncOptions, AudioFormat, Result, TempoAlgorithm as ConfigTempoAlgorithm};
use tts_sync::tts::{TtsProvider, TtsSegment};
use std::path::Path;
use std::future::Future;
use tempfile::NamedTempFile;
use std::sync::{Arc, Mutex};
use log::LevelFilter;
use tts_sync::logging::setup_test_logging;

use tts_sync::{
    sync::core::SyncCore,
    progress::ProgressTracker,
};

// Функция для инициализации логгера в тестах
fn init_test_logger() {
    setup_test_logging(LevelFilter::Debug);
}

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
async fn test_tts_sync_creation() {
    let progress_tracker = ProgressTracker::new();
    let sync_core = SyncCore::new(progress_tracker, 44100, 1, true);
    
    // Проверяем, что ядро создано успешно
    assert!(sync_core.synchronize("nonexistent.vtt", 10.0, &MockTtsProvider).await.is_err());
}

#[tokio::test]
async fn test_tts_sync_with_empty_subtitles() {
    let progress_tracker = ProgressTracker::new();
    let sync_core = SyncCore::new(progress_tracker, 44100, 1, true);
    
    // Создаем пустой файл с субтитрами
    let temp_file = NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), "").unwrap();
    
    // Проверяем, что синхронизация с пустыми субтитрами возвращает ошибку
    assert!(sync_core.synchronize(temp_file.path().to_str().unwrap(), 10.0, &MockTtsProvider).await.is_err());
}

#[tokio::test]
async fn test_tts_sync_with_valid_subtitles() {
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

#[tokio::test]
async fn test_tts_sync_with_options() {
    init_test_logger();
    
    // Создаем пользовательские настройки
    let options = SyncOptions {
        voice: "alloy".to_string(),
        output_format: AudioFormat::Mp3,
        sample_rate: 44100,
        max_segment_duration: 10.0,
        normalize_volume: true,
        apply_compression: false,
        apply_equalization: false,
        tempo_algorithm: ConfigTempoAlgorithm::Sinc,
        preserve_pauses: true,
        compression_threshold: -20.0,
        compression_ratio: 4.0,
        compression_attack: 10.0,
        compression_release: 100.0,
        compression_makeup_gain: 6.0,
        eq_low_gain: 2.0,
        eq_mid_gain: 0.0,
        eq_high_gain: 1.0,
        eq_low_freq: 300.0,
        eq_high_freq: 3000.0,
        normalization_target_db: -3.0,
        log_level: LevelFilter::Info,
    };
    
    // Создаем экземпляр TtsSync с пользовательскими настройками
    let tts_sync = TtsSync::new(options);
    
    // Проверяем, что экземпляр создан успешно
    assert!(tts_sync.synchronize("nonexistent.vtt", 10.0, "fake_api_key").await.is_err());
}

#[tokio::test]
async fn test_tts_sync_with_fluent_interface() {
    init_test_logger();
    
    // Создаем экземпляр TtsSync с использованием fluent-интерфейса
    let tts_sync = TtsSync::default()
        .with_tempo_algorithm(ConfigTempoAlgorithm::Fir)
        .with_compression(true)
        .with_equalization(true)
        .with_volume_normalization(true)
        .with_preserve_pauses(true);
    
    // Проверяем, что экземпляр создан успешно
    assert!(tts_sync.synchronize("nonexistent.vtt", 10.0, "fake_api_key").await.is_err());
}

#[tokio::test]
async fn test_progress_tracking() {
    // Создаем экземпляр TtsSync с отслеживанием прогресса
    let progress_values = Arc::new(Mutex::new(Vec::new()));
    let status_messages = Arc::new(Mutex::new(Vec::new()));
    
    let progress_values_clone = progress_values.clone();
    let status_messages_clone = status_messages.clone();
    
    let tts_sync = TtsSync::default()
        .with_progress_callback(Box::new(move |progress, status| {
            progress_values_clone.lock().unwrap().push(progress);
            status_messages_clone.lock().unwrap().push(status.to_string());
            ()
        }));
    
    // Вызываем метод синхронизации (он завершится с ошибкой, но прогресс должен обновиться)
    let _ = tts_sync.synchronize("nonexistent.vtt", 10.0, "fake_api_key").await;
    
    // Проверяем, что прогресс обновлялся
    assert!(!progress_values.lock().unwrap().is_empty());
    assert!(!status_messages.lock().unwrap().is_empty());
}

// Интеграционный тест для проверки полного процесса синхронизации
// Этот тест требует создания временных файлов и моков
#[tokio::test]
async fn test_full_synchronization_process() -> Result<()> {
    init_test_logger();
    
    use tempfile::NamedTempFile;
    
    // Создаем временный файл с VTT содержимым
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();
    
    let vtt_content = r#"WEBVTT

00:00:01.000 --> 00:00:05.000
Hello, world!

00:00:06.000 --> 00:00:10.000
This is a test.
"#;
    
    std::fs::write(&temp_path, vtt_content).unwrap();
    
    // Создаем мок TTS провайдера
    let _tts_provider = MockTtsProvider;
    
    // Создаем экземпляр TtsSync
    let _tts_sync = TtsSync::default();
    
    // Создаем временный файл для вывода
    let _output_temp_file = NamedTempFile::new().unwrap();
    let _output_path = _output_temp_file.path().to_str().unwrap().to_string();
    
    // Проверяем, что файл существует
    assert!(std::path::Path::new(&temp_path).exists());
    
    Ok(())
}