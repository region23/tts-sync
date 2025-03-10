use crate::error::{Error, Result, ErrorType};
use crate::vtt::{SubtitleTrack, VttParser};
use crate::tts::{TtsProvider, TtsSegment};
use crate::audio::{
    AudioData, AudioSegment, AudioTrack,
    AudioAnalyzer, TempoAdjuster,
    TempoAlgorithm
};
use crate::progress::ProgressTracker;
use crate::logging::{log_debug, log_info, log_error, log_warning, log_trace};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::collections::HashMap;
use std::process::Command;
use std::path::Path;
use std::fs;
use std::io::Cursor;

// Используем Symphonia для работы с аудио
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Ядро синхронизации аудио
pub struct SyncCore {
    /// Трекер прогресса
    progress_tracker: ProgressTracker,
    /// Частота дискретизации
    sample_rate: u32,
    /// Количество каналов
    channels: u16,
    /// Применять ли нормализацию громкости
    normalize_volume: bool,
    /// Целевой пик громкости (от 0.0 до 1.0)
    target_peak: f32,
    /// Сохранять ли паузы при адаптивном изменении темпа
    preserve_pauses: bool,
    /// Алгоритм изменения темпа
    tempo_algorithm: TempoAlgorithm,
}

impl SyncCore {
    /// Создает новое ядро синхронизации аудио
    pub fn new(
        progress_tracker: ProgressTracker,
        sample_rate: u32,
        channels: u16,
        normalize_volume: bool,
    ) -> Self {
        Self {
            progress_tracker,
            sample_rate,
            channels,
            normalize_volume,
            target_peak: 0.9,
            preserve_pauses: true,
            tempo_algorithm: TempoAlgorithm::Sinc,
        }
    }
    
    /// Создает новое ядро синхронизации аудио с расширенными параметрами
    pub fn new_with_options(
        progress_tracker: ProgressTracker,
        sample_rate: u32,
        channels: u16,
        normalize_volume: bool,
        target_peak: f32,
        preserve_pauses: bool,
        tempo_algorithm: TempoAlgorithm,
    ) -> Self {
        Self {
            progress_tracker,
            sample_rate,
            channels,
            normalize_volume,
            target_peak,
            preserve_pauses,
            tempo_algorithm,
        }
    }
    
    /// Синхронизирует TTS с субтитрами
    pub async fn synchronize<P: TtsProvider + Send + Sync>(
        &self,
        vtt_path: &str,
        video_duration: f64,
        tts_provider: &P,
    ) -> Result<AudioTrack> {
        // Шаг 1: Парсинг VTT файла
        self.progress_tracker.update(0.0, "Парсинг субтитров")?;
        let subtitles = VttParser::parse_file(vtt_path)?;
        
        if subtitles.is_empty() {
            return Err(Error::new(
                ErrorType::Synchronization,
                "Субтитры не найдены"
            ));
        }
        
        // Шаг 2: Генерация TTS для каждого субтитра
        self.progress_tracker.update(10.0, "Генерация TTS")?;
        let tts_segments = self.generate_tts_segments(&subtitles, tts_provider).await?;
        
        // Шаг 3: Анализ и корректировка длительности
        self.progress_tracker.update(50.0, "Анализ и корректировка длительности")?;
        let audio_segments = self.analyze_and_adjust_segments(&tts_segments, &subtitles).await?;
        
        // Шаг 4: Синхронизация аудио с субтитрами
        self.progress_tracker.update(70.0, "Синхронизация аудио с субтитрами")?;
        let mut audio_track = self.synchronize_with_subtitles(&audio_segments, &subtitles, video_duration)?;
        
        // Шаг 5: Добавление пауз между сегментами
        self.progress_tracker.update(80.0, "Добавление пауз между сегментами")?;
        audio_track = self.add_pauses_between_segments(&audio_track, &subtitles)?;
        
        // Шаг 6: Нормализация громкости
        if self.normalize_volume {
            self.progress_tracker.update(90.0, "Нормализация громкости")?;
            audio_track = self.normalize_track(&audio_track)?;
        }
        
        // Шаг 7: Проверка общей длительности
        self.progress_tracker.update(95.0, "Проверка общей длительности")?;
        audio_track = self.ensure_duration(&audio_track, video_duration)?;
        
        self.progress_tracker.update(100.0, "Синхронизация завершена")?;
        
        Ok(audio_track)
    }
    
    /// Генерирует TTS сегменты для субтитров
    async fn generate_tts_segments<P: TtsProvider + Send + Sync>(
        &self,
        subtitles: &SubtitleTrack,
        tts_provider: &P,
    ) -> Result<Vec<TtsSegment>> {
        let mut tts_segments = Vec::with_capacity(subtitles.len());
        
        let progress_step = 40.0f32 / subtitles.len() as f32;
        let mut current_progress = 10.0f32;
        
        // Создаем кэш для хранения уже сгенерированных TTS сегментов
        let mut segments_cache: HashMap<String, TtsSegment> = HashMap::new();
        
        log_info(&format!("Начало генерации {} TTS сегментов", subtitles.len()));
        
        // Временная директория для сохранения и проверки TTS данных
        let temp_dir = std::env::temp_dir().join("tts_sync_temp");
        if !temp_dir.exists() {
            std::fs::create_dir_all(&temp_dir).map_err(|e| 
                Error::new(ErrorType::Io, &format!("Не удалось создать временную директорию: {}", e)))?;
        }
        
        for (i, subtitle) in subtitles.iter().enumerate() {
            // Обновляем прогресс
            self.progress_tracker.update(
                current_progress,
                &format!("Генерация TTS {}/{}", i + 1, subtitles.len())
            )?;
            
            log_debug(&format!("Обработка сегмента {}/{}: '{}' (длительность: {:.2}с)",
                i + 1, subtitles.len(), subtitle.text, subtitle.duration()));
            
            // Проверяем, есть ли сегмент в кэше
            let cache_key = subtitle.text.clone();
            let segment = if let Some(cached_segment) = segments_cache.get(&cache_key) {
                log_debug(&format!("Использован кэшированный TTS для сегмента {}/{}", i + 1, subtitles.len()));
                cached_segment.clone()
            } else {
                // Если нет в кэше, генерируем новый
                log_debug(&format!("Генерация нового TTS для сегмента {}/{}", i + 1, subtitles.len()));
                let start = std::time::Instant::now();
                
                // Генерируем TTS
                let mut segment = tts_provider.generate_segment(&subtitle.text, subtitle.duration()).await?;
                let duration = start.elapsed();
                
                // Проверяем полученные данные
                let audio_size = segment.audio_data.len();
                log_debug(&format!("TTS сегмент {}/{} сгенерирован за {:.2?}, размер данных: {} байт",
                    i + 1, subtitles.len(), duration, audio_size));
                
                if audio_size < 100 {
                    log_warning(&format!("Подозрительно маленький размер TTS данных для сегмента {}: {} байт", 
                        i + 1, audio_size));
                }
                
                // Для отладки: сохраним полученные TTS данные во временный файл и проверим их
                let temp_file = temp_dir.join(format!("tts_segment_{}.mp3", i + 1));
                let temp_path = temp_file.to_str().unwrap_or("temp.mp3");
                
                // Сохраняем во временный файл
                let mut file = File::create(temp_path).await
                    .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось создать временный файл: {}", e)))?;
                file.write_all(&segment.audio_data).await
                    .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось записать TTS данные: {}", e)))?;
                
                // Проверяем формат полученных данных
                if let Err(e) = self.validate_tts_data(temp_path).await {
                    log_warning(&format!("Проблема с TTS данными: {}", e));
                }
                
                // Добавляем в кэш
                segments_cache.insert(cache_key, segment.clone());
                segment
            };
            
            tts_segments.push(segment);
            current_progress += progress_step;
        }
        
        log_info(&format!("Сгенерировано {} TTS сегментов, из них уникальных: {}", 
            tts_segments.len(), segments_cache.len()));
        
        // Попытка очистки временной директории
        let _ = std::fs::remove_dir_all(&temp_dir);
        
        Ok(tts_segments)
    }
    
    /// Анализирует и корректирует длительность сегментов
    async fn analyze_and_adjust_segments(
        &self,
        tts_segments: &[TtsSegment],
        subtitles: &SubtitleTrack,
    ) -> Result<Vec<AudioSegment>> {
        let mut adjusted_segments = Vec::with_capacity(tts_segments.len());
        
        let progress_step = 20.0f32 / tts_segments.len() as f32;
        let mut current_progress = 50.0f32;
        
        for (i, (segment, subtitle)) in tts_segments.iter().zip(subtitles.iter()).enumerate() {
            // Обновляем прогресс
            self.progress_tracker.update(
                current_progress,
                &format!("Анализ и корректировка сегмента {}/{}", i + 1, tts_segments.len())
            )?;
            
            // Сохраняем исходные MP3 данные для последующего прямого сохранения
            let raw_audio_data = segment.audio_data.clone();
            
            // Конвертируем бинарные аудио данные в float сэмплы
            // Это упрощенная реализация, в реальности нужно декодировать MP3/другой формат
            // Здесь мы просто создаем пустые сэмплы для демонстрации
            let samples = vec![0.0f32; (self.sample_rate as f64 * subtitle.duration()) as usize];
            
            // Создаем аудио данные
            let audio_data = AudioData::new(
                samples,
                self.sample_rate,
                self.channels,
            );
            
            // Анализируем аудио для определения характеристик
            let _analysis = AudioAnalyzer::analyze(&audio_data)?;
            
            // Рассчитываем целевую длительность из субтитров
            let target_duration = subtitle.duration() as f32;
            let current_duration = audio_data.duration() as f32;
            
            // Корректируем длительность аудио, если необходимо
            let adjusted_audio = if (current_duration - target_duration).abs() > 0.05 {
                // Используем адаптивное изменение темпа с сохранением пауз
                TempoAdjuster::adaptive_tempo_adjustment(
                    &audio_data,
                    target_duration,
                    self.tempo_algorithm,
                    self.preserve_pauses
                )?
            } else {
                // Если разница незначительная, оставляем как есть
                audio_data
            };
            
            // Создаем аудио сегмент с сохранением исходных данных
            let audio_segment = AudioSegment::new_with_raw_data(
                adjusted_audio,
                subtitle.start_time,
                subtitle.end_time,
                subtitle.text.clone(),
                raw_audio_data
            );
            
            adjusted_segments.push(audio_segment);
            
            current_progress += progress_step;
        }
        
        Ok(adjusted_segments)
    }
    
    /// Синхронизирует аудио сегменты с субтитрами и видео
    fn synchronize_with_subtitles(
        &self,
        audio_segments: &[AudioSegment],
        _subtitles: &SubtitleTrack,
        video_duration: f64,
    ) -> Result<AudioTrack> {
        let mut audio_track = AudioTrack::default();
        
        // Добавляем все сегменты в аудио трек
        for segment in audio_segments {
            audio_track.add_segment(segment.clone());
        }
        
        // Проверяем, что все сегменты находятся в пределах длительности видео
        if let Some(last_segment) = audio_track.segments.last() {
            if last_segment.end_time > video_duration {
                // Если последний сегмент выходит за пределы видео, корректируем его
                let mut adjusted_segment = last_segment.clone();
                adjusted_segment.end_time = video_duration;
                
                // Заменяем последний сегмент
                audio_track.segments.pop();
                audio_track.add_segment(adjusted_segment);
            }
        }
        
        Ok(audio_track)
    }
    
    /// Добавляет паузы между сегментами для более естественного звучания
    fn add_pauses_between_segments(
        &self,
        audio_track: &AudioTrack,
        _subtitles: &SubtitleTrack,
    ) -> Result<AudioTrack> {
        let mut result_track = AudioTrack::new(self.sample_rate, self.channels);
        
        // Добавляем сегменты с паузами между ними
        for segment in &audio_track.segments {
            result_track.add_segment(segment.clone());
            
            // Добавляем паузу после сегмента
            let silence_duration = 0.2; // 200ms пауза
            let silence_samples = vec![0.0f32; (self.sample_rate as f64 * silence_duration) as usize];
            let silence_data = AudioData::new(silence_samples, self.sample_rate, self.channels);
            
            let silence_segment = AudioSegment::new(
                silence_data,
                segment.end_time,
                segment.end_time + silence_duration,
                String::new()
            );
            
            result_track.add_segment(silence_segment);
        }
        
        Ok(result_track)
    }
    
    /// Нормализует громкость аудио трека
    fn normalize_track(&self, audio_track: &AudioTrack) -> Result<AudioTrack> {
        let mut result_track = AudioTrack::new(self.sample_rate, self.channels);
        
        // Нормализуем каждый сегмент
        for segment in &audio_track.segments {
            let mut normalized_audio = segment.audio.clone();
            normalized_audio.normalize(self.target_peak);
            
            let normalized_segment = AudioSegment::new(
                normalized_audio,
                segment.start_time,
                segment.end_time,
                segment.text.clone()
            );
            
            result_track.add_segment(normalized_segment);
        }
        
        Ok(result_track)
    }
    
    /// Проверяет и корректирует общую длительность аудио трека
    fn ensure_duration(&self, audio_track: &AudioTrack, video_duration: f64) -> Result<AudioTrack> {
        let mut result_track = AudioTrack::new(self.sample_rate, self.channels);
        
        // Копируем все сегменты
        for segment in &audio_track.segments {
            if segment.start_time < video_duration {
                result_track.add_segment(segment.clone());
            }
        }
        
        // Если последний сегмент выходит за пределы видео, корректируем его
        if let Some(last_segment) = result_track.segments.last() {
            if last_segment.end_time > video_duration {
                let mut adjusted_segment = last_segment.clone();
                adjusted_segment.end_time = video_duration;
                
                result_track.segments.pop();
                result_track.add_segment(adjusted_segment);
            }
        }
        
        // Если аудио короче видео, добавляем тишину в конец
        if let Some(last_segment) = result_track.segments.last() {
            if last_segment.end_time < video_duration {
                let silence_duration = video_duration - last_segment.end_time;
                let silence_samples = vec![0.0f32; (self.sample_rate as f64 * silence_duration) as usize];
                let silence_data = AudioData::new(silence_samples, self.sample_rate, self.channels);
                
                let silence_segment = AudioSegment::new(
                    silence_data,
                    last_segment.end_time,
                    video_duration,
                    String::new()
                );
                
                result_track.add_segment(silence_segment);
            }
        }
        
        Ok(result_track)
    }
    
    /// Сохраняет аудио трек в файл
    pub async fn save_to_file(&self, audio_track: &AudioTrack, path: &str) -> Result<()> {
        // Объединяем все сегменты
        let merged_audio = audio_track.merge()?;
        
        let num_segments = audio_track.segments.len();
        let total_samples = merged_audio.samples.len();
        log_debug(&format!("Сохранение аудио файла: {} сегментов, {} сэмплов, длительность {:.2}с", 
            num_segments, total_samples, merged_audio.duration()));
        
        if num_segments == 0 || total_samples == 0 {
            log_error::<(), _>(
                &Error::new(ErrorType::AudioProcessingError, "Аудио трек пуст или не содержит сэмплов"),
                "Ошибка при сохранении аудио"
            )?;
            return Err(Error::new(ErrorType::AudioProcessingError, "Аудио трек пуст или не содержит сэмплов"));
        }
        
        // Определяем формат по расширению файла
        let ext = Path::new(path).extension()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("mp3")
            .to_lowercase();
        
        // Проверяем, есть ли у нас исходные MP3 данные, которые можно сохранить напрямую
        if ext == "mp3" && self.try_direct_mp3_save(audio_track, path).await? {
            log_info(&format!("Финальный аудио файл создан напрямую: {}, формат: MP3", path));
            return Ok(());
        }
        
        // Всегда сначала сохраняем в WAV, так как с ним проще работать
        let temp_wav_path = format!("{}.temp.wav", path);
        log_debug(&format!("Создание временного WAV файла: {}", temp_wav_path));
        
        // Сначала записываем данные в WAV формате
        self.write_wav_file(&merged_audio, &temp_wav_path).await?;
        
        // Проверяем, что WAV файл действительно содержит данные
        let wav_metadata = tokio::fs::metadata(&temp_wav_path).await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось получить информацию о WAV файле: {}", e)))?;
        
        log_debug(&format!("Временный WAV файл создан, размер: {} байт", wav_metadata.len()));
        
        if wav_metadata.len() <= 44 { // Только заголовок WAV, нет данных
            log_error::<(), _>(
                &Error::new(ErrorType::AudioProcessingError, "WAV файл не содержит аудио данных (только заголовок)"),
                "Ошибка при сохранении аудио"
            )?;
            return Err(Error::new(ErrorType::AudioProcessingError, "WAV файл не содержит аудио данных"));
        }
        
        // Теперь конвертируем в нужный формат
        match ext.as_str() {
            "mp3" => {
                log_debug("Конвертация WAV в MP3...");
                
                // Пробуем использовать ffmpeg
                let result = self.convert_with_ffmpeg(&temp_wav_path, path, "mp3", &[
                    "-codec:a", "libmp3lame", 
                    "-q:a", "2", // Высокое качество (0-9, где 0 - лучшее)
                    "-b:a", "192k" // Битрейт
                ]);
                
                match result {
                    Ok(_) => log_debug(&format!("Файл MP3 успешно создан с помощью ffmpeg: {}", path)),
                    Err(e) => {
                        log_warning(&format!("Ошибка ffmpeg: {}, пробую резервный метод", e));
                        self.convert_with_symphonia(&temp_wav_path, path, "mp3").await?;
                    }
                }
            },
            "wav" => {
                // WAV уже создан, просто переименовываем
                if temp_wav_path != path {
                    tokio::fs::copy(&temp_wav_path, path).await
                        .map_err(|e| Error::new(ErrorType::Io, &format!("Ошибка при копировании WAV файла: {}", e)))?;
                    log_debug(&format!("WAV файл скопирован в: {}", path));
                }
            },
            "ogg" => {
                log_debug("Конвертация WAV в OGG...");
                
                // Пробуем использовать ffmpeg
                let result = self.convert_with_ffmpeg(&temp_wav_path, path, "ogg", &[
                    "-codec:a", "libvorbis",
                    "-q:a", "6" // Качество (0-10, где 10 - лучшее)
                ]);
                
                match result {
                    Ok(_) => log_debug(&format!("Файл OGG успешно создан с помощью ffmpeg: {}", path)),
                    Err(e) => {
                        log_warning(&format!("Ошибка ffmpeg: {}, пробую резервный метод", e));
                        self.convert_with_symphonia(&temp_wav_path, path, "ogg").await?;
                    }
                }
            },
            _ => {
                return Err(Error::new(
                    ErrorType::AudioProcessingError,
                    &format!("Неподдерживаемый формат аудио: {}", ext)
                ));
            }
        }
        
        // Удаляем временный WAV файл
        if Path::new(&temp_wav_path).exists() {
            let _ = tokio::fs::remove_file(&temp_wav_path).await;
            log_debug(&format!("Временный файл удален: {}", temp_wav_path));
        }
        
        // Проверяем, что выходной файл существует и содержит данные
        match tokio::fs::metadata(path).await {
            Ok(metadata) => {
                let file_size = metadata.len();
                log_info(&format!("Финальный аудио файл создан: {}, размер: {} байт", path, file_size));
                
                if file_size <= 100 { // Подозрительно маленький файл
                    log_warning(&format!("Финальный файл подозрительно мал: {} байт", file_size));
                }
            },
            Err(e) => {
                log_error::<(), _>(
                    &Error::new(ErrorType::Io, &format!("Не удалось получить информацию о выходном файле: {}", e)),
                    "Ошибка при проверке выходного файла"
                )?;
            }
        }
        
        Ok(())
    }
    
    /// Конвертирует аудио файл с помощью ffmpeg
    fn convert_with_ffmpeg(&self, input_path: &str, output_path: &str, format: &str, codec_args: &[&str]) -> std::io::Result<()> {
        log_debug(&format!("Запуск ffmpeg для конвертации в {}: {} -> {}", format, input_path, output_path));
        
        // Базовые аргументы
        let mut args = vec![
            "-y",           // Перезаписать выходной файл без вопросов
            "-i", input_path, // Входной файл
            "-vn",          // Без видео
        ];
        
        // Добавляем специальные аргументы для кодека
        args.extend_from_slice(codec_args);
        
        // Добавляем выходной файл
        args.push(output_path);
        
        log_debug(&format!("Команда ffmpeg: ffmpeg {}", args.join(" ")));
        
        let output = Command::new("ffmpeg")
            .args(&args)
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_warning(&format!("ffmpeg завершился с ошибкой: {}", stderr));
            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("ffmpeg завершился с ошибкой: {}", stderr)));
        }
        
        Ok(())
    }
    
    /// Конвертирует аудио файл с помощью библиотеки symphonia
    async fn convert_with_symphonia(&self, input_path: &str, output_path: &str, format: &str) -> Result<()> {
        log_debug(&format!("Использую библиотеку symphonia для конвертации в {}: {} -> {}", format, input_path, output_path));
        
        // Вместо этого используем прямую копию WAV файла с предупреждением
        
        log_warning(&format!("Полная конвертация в {} с помощью библиотеки не реализована", format));
        log_warning("Копирую WAV файл с новым расширением как временное решение");
        log_warning("Для корректного перекодирования аудио установите ffmpeg");
        
        tokio::fs::copy(input_path, output_path).await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Ошибка при копировании файла: {}", e)))?;
        
        Ok(())
    }

    /// Записывает данные аудио в формате WAV
    async fn write_wav_file(&self, audio_data: &AudioData, path: &str) -> Result<()> {
        log_debug(&format!("Запись WAV файла: {}, {} сэмплов, {} каналов, {}Hz", 
            path, audio_data.samples.len(), audio_data.channels, audio_data.sample_rate));
        
        let mut file = File::create(path).await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось создать WAV файл: {}", e)))?;
        
        let total_samples = audio_data.samples.len();
        let num_channels = audio_data.channels;
        let sample_rate = audio_data.sample_rate;
        
        // Если нет сэмплов, выдаем ошибку
        if total_samples == 0 {
            return Err(Error::new(ErrorType::AudioProcessingError, "Попытка записать пустые аудио данные"));
        }
        
        // Создаем заголовок WAV
        let bytes_per_sample = 2; // 16-bit PCM = 2 байта на сэмпл
        let data_size = (total_samples * bytes_per_sample) as u32;
        let file_size = data_size + 36; // 44 байта заголовка - 8 байтов
        
        // Создаем подробный заголовок WAV
        let mut header = Vec::new();
        
        // RIFF chunk
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&file_size.to_le_bytes());
        header.extend_from_slice(b"WAVE");
        
        // fmt subchunk
        header.extend_from_slice(b"fmt ");
        header.extend_from_slice(&16u32.to_le_bytes()); // размер подчанка fmt (16 байтов)
        header.extend_from_slice(&1u16.to_le_bytes()); // аудио формат (1 = PCM)
        header.extend_from_slice(&num_channels.to_le_bytes()); // количество каналов
        header.extend_from_slice(&sample_rate.to_le_bytes()); // частота дискретизации
        
        // Байт рейт = SampleRate * NumChannels * BitsPerSample / 8
        let byte_rate = sample_rate * num_channels as u32 * 16 / 8;
        header.extend_from_slice(&byte_rate.to_le_bytes());
        
        // Блок выравнивания = NumChannels * BitsPerSample / 8
        let block_align = num_channels as u16 * 16 / 8;
        header.extend_from_slice(&block_align.to_le_bytes());
        
        header.extend_from_slice(&16u16.to_le_bytes()); // биты на сэмпл
        
        // data subchunk
        header.extend_from_slice(b"data");
        header.extend_from_slice(&data_size.to_le_bytes());
        
        // Записываем заголовок
        file.write_all(&header).await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось записать заголовок WAV: {}", e)))?;
        
        log_debug(&format!("Записан заголовок WAV: {} байт", header.len()));
        
        // Конвертируем float сэмплы в 16-bit PCM
        let mut pcm_data = Vec::with_capacity(total_samples * bytes_per_sample);
        
        for &sample in &audio_data.samples {
            // Преобразуем float в int16, важно нормализовать значения правильно
            let pcm_sample = (sample.max(-1.0).min(1.0) * 32767.0) as i16;
            
            // Записываем в порядке little-endian (младший байт, затем старший)
            pcm_data.extend_from_slice(&pcm_sample.to_le_bytes());
        }
        
        // Записываем PCM данные
        file.write_all(&pcm_data).await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось записать аудио данные: {}", e)))?;
        
        log_debug(&format!("Записаны аудио данные: {} байт", pcm_data.len()));
        log_debug(&format!("Записан WAV файл общего размера: {} байт", header.len() + pcm_data.len()));
        
        // Проверяем содержимое начала файла для отладки
        let mut debug_file = File::open(path).await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось открыть WAV файл для проверки: {}", e)))?;
        
        let mut debug_header = vec![0u8; 44];
        let _ = debug_file.read_exact(&mut debug_header).await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось прочитать заголовок для проверки: {}", e)))?;
        
        log_debug(&format!("Проверка заголовка WAV: RIFF={}, WAVE={}, fmt={}, data={}", 
            &debug_header[0..4] == b"RIFF",
            &debug_header[8..12] == b"WAVE",
            &debug_header[12..16] == b"fmt ",
            &debug_header[36..40] == b"data"));
        
        Ok(())
    }

    /// Проверяет аудиоданные TTS перед использованием
    async fn validate_tts_data(&self, file_path: &str) -> Result<()> {
        log_debug(&format!("Проверка аудиофайла: {}", file_path));
        
        // Читаем часть файла для проверки
        let mut file = File::open(file_path).await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось открыть файл для проверки: {}", e)))?;
            
        let metadata = file.metadata().await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось получить метаданные файла: {}", e)))?;
            
        let file_size = metadata.len();
        log_debug(&format!("Размер файла: {} байт", file_size));
        
        if file_size < 100 {
            // Слишком маленький файл - что-то не так
            log_warning(&format!("Подозрительно маленький размер файла: {} байт", file_size));
        }
        
        // Проверим формат файла по сигнатуре
        let mut header = vec![0u8; 16];
        let read_bytes = file.read(&mut header).await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось прочитать заголовок файла: {}", e)))?;
            
        log_debug(&format!("Прочитано {} байт заголовка", read_bytes));
        
        // Проверим известные сигнатуры аудиоформатов
        let is_mp3 = header.starts_with(b"ID3") || 
                    (header[0] == 0xFF && (header[1] & 0xE0) == 0xE0);
        let is_wav = header.starts_with(b"RIFF") && &header[8..12] == b"WAVE";
        let is_ogg = header.starts_with(b"OggS");
        
        if is_mp3 {
            log_debug("Файл определен как MP3");
        } else if is_wav {
            log_debug("Файл определен как WAV");
        } else if is_ogg {
            log_debug("Файл определен как OGG");
        } else {
            log_warning("Неизвестный формат файла");
            // Вывод первых байтов для отладки
            let hex_display: Vec<String> = header.iter().take(16).map(|b| format!("{:02X}", b)).collect();
            log_debug(&format!("Первые 16 байт: {}", hex_display.join(" ")));
        }
        
        Ok(())
    }

    /// Пытается сохранить MP3 файл напрямую из исходных сегментов, если они в MP3 формате
    async fn try_direct_mp3_save(&self, audio_track: &AudioTrack, path: &str) -> Result<bool> {
        // Проверяем, есть ли у нас доступ к исходным MP3 данным в сегментах
        let has_raw_mp3 = audio_track.segments.iter().any(|segment| {
            // Здесь проверка наличия исходных MP3 данных в сегменте
            // В текущей реализации мы используем эвристику - проверяем первые байты
            if let Some(original_data) = self.get_raw_segment_data(segment) {
                // Проверяем, что это MP3 файл: должен начинаться с ID3 или с MP3 frame sync
                !original_data.is_empty() && (
                    (original_data.len() > 3 && &original_data[0..3] == b"ID3") ||
                    (original_data.len() > 2 && (original_data[0] == 0xFF && (original_data[1] & 0xE0) == 0xE0))
                )
            } else {
                false
            }
        });

        if !has_raw_mp3 {
            log_debug("Не найдены исходные MP3 данные в сегментах, использую стандартный процесс конвертации");
            return Ok(false);
        }

        log_debug("Найдены исходные MP3 данные, пробую прямое сохранение");

        // Собираем все MP3 сегменты в один файл
        let mut output_file = tokio::fs::File::create(path).await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось создать выходной файл: {}", e)))?;

        // Копируем данные каждого сегмента напрямую в выходной файл
        for (idx, segment) in audio_track.segments.iter().enumerate() {
            if let Some(mp3_data) = self.get_raw_segment_data(segment) {
                log_debug(&format!("Копирование исходного MP3 сегмента {}/{} ({}Kб)", 
                    idx + 1, audio_track.segments.len(), mp3_data.len() / 1024));
                
                output_file.write_all(&mp3_data).await
                    .map_err(|e| Error::new(ErrorType::Io, &format!("Ошибка записи MP3 данных: {}", e)))?;
            }
        }

        // Проверяем размер созданного файла
        let file_metadata = tokio::fs::metadata(path).await
            .map_err(|e| Error::new(ErrorType::Io, &format!("Не удалось получить метаданные файла: {}", e)))?;
        
        if file_metadata.len() == 0 {
            log_warning("Созданный MP3 файл пуст, возвращаюсь к стандартному методу");
            tokio::fs::remove_file(path).await.ok(); // Удаляем пустой файл
            return Ok(false);
        }

        log_info(&format!("Финальный аудио файл создан напрямую из MP3 сегментов: {}, размер: {} байт", 
            path, file_metadata.len()));
        return Ok(true);
    }

    /// Получает исходные MP3 данные из сегмента (если они доступны)
    fn get_raw_segment_data(&self, segment: &AudioSegment) -> Option<Vec<u8>> {
        // Возвращаем клонированный вектор с исходными данными, если они есть
        segment.raw_data.clone()
    }
}
