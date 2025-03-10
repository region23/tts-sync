pub mod error;
pub mod vtt;
pub mod tts;
pub mod audio;
pub mod sync;
pub mod progress;
pub mod logging;

pub use error::{Error, Result, ErrorType};
pub use logging::{
    setup_logging, setup_test_logging, log_error, log_warning, log_info, log_debug, log_trace
};
pub use vtt::{Subtitle, SubtitleTrack, VttParser};
pub use tts::{
    TtsProvider, OpenAiTts, TtsOptions, TtsSegment,
    OpenAiVoice, OpenAiTtsModel, OpenAiAudioFormat
};
pub use audio::{
    AudioData, AudioSegment, AudioTrack,
    AudioAnalyzer, AudioAnalysis, SegmentAnalysis, SilenceSegment,
    TempoAdjuster, AudioSynchronizer, AudioProcessor
};
pub use progress::{ProgressTracker, ProgressCallback, ChildProgressTracker};
pub use sync::core::SyncCore;

/// Форматы выходного аудио файла
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// MP3 формат
    Mp3,
    /// WAV формат
    Wav,
    /// OGG формат
    Ogg,
}

/// Алгоритмы изменения темпа
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempoAlgorithm {
    /// Алгоритм на основе sinc интерполяции (высокое качество, медленнее)
    Sinc,
    /// Алгоритм на основе FIR фильтра (среднее качество, быстрее)
    Fir,
    /// Алгоритм на основе линейной интерполяции (низкое качество, очень быстрый)
    Linear,
}

/// Настройки для синхронизации TTS с видео и субтитрами
#[derive(Debug, Clone)]
pub struct SyncOptions {
    /// Голос для TTS
    pub voice: String,
    
    /// Формат выходного аудио файла
    pub output_format: AudioFormat,
    
    /// Частота дискретизации выходного аудио
    pub sample_rate: u32,
    
    /// Максимальная длительность сегмента в секундах
    pub max_segment_duration: f64,
    
    /// Применять ли нормализацию громкости
    pub normalize_volume: bool,
    
    /// Применять ли компрессию динамического диапазона
    pub apply_compression: bool,
    
    /// Применять ли эквализацию
    pub apply_equalization: bool,
    
    /// Алгоритм изменения темпа
    pub tempo_algorithm: TempoAlgorithm,
    
    /// Сохранять ли паузы при адаптивном изменении темпа
    pub preserve_pauses: bool,
    
    /// Параметры компрессии
    pub compression_threshold: f32,
    pub compression_ratio: f32,
    pub compression_attack: f32,
    pub compression_release: f32,
    pub compression_makeup_gain: f32,
    
    /// Параметры эквализации
    pub eq_low_gain: f32,
    pub eq_mid_gain: f32,
    pub eq_high_gain: f32,
    pub eq_low_freq: f32,
    pub eq_high_freq: f32,
    
    /// Целевой уровень нормализации громкости в дБ
    pub normalization_target_db: f32,
    
    /// Уровень логирования
    pub log_level: log::LevelFilter,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            voice: "alloy".to_string(),
            output_format: AudioFormat::Mp3,
            sample_rate: 44100,
            max_segment_duration: 10.0,
            normalize_volume: true,
            apply_compression: false,
            apply_equalization: false,
            tempo_algorithm: TempoAlgorithm::Sinc,
            preserve_pauses: true,
            
            // Параметры компрессии по умолчанию
            compression_threshold: -20.0,
            compression_ratio: 4.0,
            compression_attack: 10.0,
            compression_release: 100.0,
            compression_makeup_gain: 6.0,
            
            // Параметры эквализации по умолчанию
            eq_low_gain: 2.0,
            eq_mid_gain: 0.0,
            eq_high_gain: 1.0,
            eq_low_freq: 300.0,
            eq_high_freq: 3000.0,
            
            // Целевой уровень нормализации громкости
            normalization_target_db: -3.0,
            
            log_level: log::LevelFilter::Info,
        }
    }
}

/// Основной интерфейс для синхронизации TTS с видео и субтитрами
pub struct TtsSync {
    options: SyncOptions,
    progress_tracker: ProgressTracker,
}

impl TtsSync {
    /// Создает новый экземпляр TtsSync с заданными настройками
    pub fn new(options: SyncOptions) -> Self {
        #[cfg(test)]
        {
            setup_test_logging(options.log_level);
        }
        #[cfg(not(test))]
        {
            setup_logging(options.log_level);
        }
        
        log_info(&format!("Создан новый экземпляр TtsSync с настройками: {:?}", options));
        
        Self { 
            options,
            progress_tracker: ProgressTracker::new(),
        }
    }
    
    /// Создает новый экземпляр TtsSync с настройками по умолчанию
    pub fn default() -> Self {
        Self::new(SyncOptions::default())
    }
    
    /// Устанавливает функцию обратного вызова для отслеживания прогресса
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        log_debug("Установлена функция обратного вызова для отслеживания прогресса");
        self.progress_tracker = ProgressTracker::with_callback(callback);
        self
    }
    
    /// Устанавливает алгоритм изменения темпа
    pub fn with_tempo_algorithm(mut self, algorithm: TempoAlgorithm) -> Self {
        log_debug(&format!("Установлен алгоритм изменения темпа: {:?}", algorithm));
        self.options.tempo_algorithm = algorithm;
        self
    }
    
    /// Устанавливает применение компрессии
    pub fn with_compression(mut self, apply_compression: bool) -> Self {
        log_debug(&format!("Установлено применение компрессии: {}", apply_compression));
        self.options.apply_compression = apply_compression;
        self
    }
    
    /// Устанавливает применение эквализации
    pub fn with_equalization(mut self, apply_equalization: bool) -> Self {
        log_debug(&format!("Установлено применение эквализации: {}", apply_equalization));
        self.options.apply_equalization = apply_equalization;
        self
    }
    
    /// Устанавливает нормализацию громкости
    pub fn with_volume_normalization(mut self, normalize_volume: bool) -> Self {
        log_debug(&format!("Установлена нормализация громкости: {}", normalize_volume));
        self.options.normalize_volume = normalize_volume;
        self
    }
    
    /// Устанавливает сохранение пауз при адаптивном изменении темпа
    pub fn with_preserve_pauses(mut self, preserve_pauses: bool) -> Self {
        log_debug(&format!("Установлено сохранение пауз: {}", preserve_pauses));
        self.options.preserve_pauses = preserve_pauses;
        self
    }
    
    /// Синхронизирует TTS с видео и субтитрами
    pub async fn synchronize(
        &self,
        vtt_path: &str,
        video_duration: f64,
        api_key: &str,
    ) -> Result<String> {
        log_info(&format!("Начало синхронизации TTS для файла: {}", vtt_path));
        
        // Создаем TTS провайдер
        let tts_options = TtsOptions {
            model: OpenAiTtsModel::Tts1Hd,
            voice: OpenAiVoice::from_str(&self.options.voice)?,
            speed: 1.0,
            response_format: OpenAiAudioFormat::Mp3,
        };
        
        let tts_provider = OpenAiTts::new(api_key.to_string(), tts_options);
        
        // Создаем ядро синхронизации с расширенными параметрами
        let sync_core = SyncCore::new(
            self.progress_tracker.clone(),
            self.options.sample_rate,
            1, // Моно аудио
            self.options.normalize_volume,
        );
        
        // Синхронизируем TTS с субтитрами
        let mut audio_track = match sync_core.synchronize(vtt_path, video_duration, &tts_provider).await {
            Ok(track) => track,
            Err(e) => {
                log_error::<(), _>(&e, "Ошибка при синхронизации TTS")?;
                return Err(e);
            }
        };
        
        // Применяем дополнительную обработку аудио, если требуется
        if self.options.apply_compression || self.options.apply_equalization || self.options.normalize_volume {
            self.progress_tracker.update(90.0, "Применение аудио эффектов")?;
            
            // Объединяем все сегменты в один аудио файл для обработки
            let merged_audio = audio_track.merge()?;
            
            // Применяем компрессию, если включена
            let processed_audio = if self.options.apply_compression {
                log_info("Применение компрессии динамического диапазона");
                merged_audio.apply_compression(
                    self.options.compression_threshold,
                    self.options.compression_ratio,
                    self.options.compression_attack,
                    self.options.compression_release,
                    self.options.compression_makeup_gain
                )?
            } else {
                merged_audio
            };
            
            // Применяем эквализацию, если включена
            let processed_audio = if self.options.apply_equalization {
                log_info("Применение эквализации");
                processed_audio.apply_equalization(
                    self.options.eq_low_gain,
                    self.options.eq_mid_gain,
                    self.options.eq_high_gain,
                    self.options.eq_low_freq,
                    self.options.eq_high_freq
                )?
            } else {
                processed_audio
            };
            
            // Нормализуем громкость, если включена
            let processed_audio = if self.options.normalize_volume {
                log_info("Нормализация громкости");
                processed_audio.normalize_db(self.options.normalization_target_db)
            } else {
                processed_audio
            };
            
            // Создаем новый аудио трек с одним сегментом
            audio_track = AudioTrack::default();
            audio_track.add_segment(AudioSegment::new(
                processed_audio.clone(),
                0.0,
                processed_audio.duration(),
                String::new()
            ));
        }
        
        // Генерируем имя выходного файла
        let output_path = format!("{}.{}", vtt_path.replace(".vtt", "_tts"), 
            match self.options.output_format {
                AudioFormat::Mp3 => "mp3",
                AudioFormat::Wav => "wav",
                AudioFormat::Ogg => "ogg",
            }
        );
        
        // Сохраняем аудио в файл
        self.progress_tracker.update(95.0, "Сохранение аудио файла")?;
        match sync_core.save_to_file(&audio_track, &output_path).await {
            Ok(_) => {
                log_info(&format!("Аудио успешно сохранено в файл: {}", output_path));
                self.progress_tracker.update(100.0, "Синхронизация завершена")?;
                Ok(output_path)
            },
            Err(e) => {
                log_error::<(), _>(&e, &format!("Ошибка при сохранении аудио в файл: {}", output_path))?;
                Err(e)
            }
        }
    }
    
    /// Синхронизирует TTS с видео и субтитрами, возвращая аудио данные
    pub async fn synchronize_to_memory(
        &self,
        vtt_path: &str,
        video_duration: f64,
        api_key: &str,
    ) -> Result<Vec<f32>> {
        log_info(&format!("Начало синхронизации TTS в память для файла: {}", vtt_path));
        
        // Создаем TTS провайдер
        let tts_options = TtsOptions {
            model: OpenAiTtsModel::Tts1Hd,
            voice: OpenAiVoice::from_str(&self.options.voice)?,
            speed: 1.0,
            response_format: OpenAiAudioFormat::Mp3,
        };
        
        let tts_provider = OpenAiTts::new(api_key.to_string(), tts_options);
        
        // Создаем ядро синхронизации
        let sync_core = SyncCore::new(
            self.progress_tracker.clone(),
            self.options.sample_rate,
            1, // Моно аудио
            self.options.normalize_volume,
        );
        
        // Синхронизируем TTS с субтитрами
        let audio_track = match sync_core.synchronize(vtt_path, video_duration, &tts_provider).await {
            Ok(track) => track,
            Err(e) => {
                log_error::<(), _>(&e, "Ошибка при синхронизации TTS")?;
                return Err(e);
            }
        };
        
        // Объединяем все сегменты в один аудио файл
        match audio_track.merge() {
            Ok(merged_audio) => {
                log_info(&format!("Аудио успешно синхронизировано, длительность: {} секунд", merged_audio.duration()));
                Ok(merged_audio.samples)
            },
            Err(e) => {
                log_error::<(), _>(&e, "Ошибка при объединении аудио сегментов")?;
                Err(e)
            }
        }
    }
}
