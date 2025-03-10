use crate::error::{Error, Result, ErrorType};
use crate::vtt::{SubtitleTrack, VttParser};
use crate::tts::{TtsProvider, TtsSegment};
use crate::audio::{
    AudioData, AudioSegment, AudioTrack,
    AudioAnalyzer, TempoAdjuster,
    TempoAlgorithm
};
use crate::progress::ProgressTracker;
use crate::logging::{log_debug, log_info, log_error, log_warning};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use std::collections::HashMap;
use std::process::Command;

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
        
        for (i, subtitle) in subtitles.iter().enumerate() {
            // Обновляем прогресс
            self.progress_tracker.update(
                current_progress,
                &format!("Генерация TTS {}/{}", i + 1, subtitles.len())
            )?;
            
            log_debug(&format!("Обработка сегмента {}/{}: '{}' (длительность: {:.2}с)",
                i + 1, subtitles.len(), subtitle.text, subtitle.duration()));
            
            // Проверяем, есть ли сегмент в кэше
            let segment = if let Some(cached_segment) = segments_cache.get(&subtitle.text) {
                log_debug(&format!("Использован кэшированный TTS для сегмента {}/{}", i + 1, subtitles.len()));
                cached_segment.clone()
            } else {
                // Если нет в кэше, генерируем новый
                log_debug(&format!("Генерация нового TTS для сегмента {}/{}", i + 1, subtitles.len()));
                let start = std::time::Instant::now();
                let segment = tts_provider.generate_segment(&subtitle.text, subtitle.duration()).await?;
                let duration = start.elapsed();
                log_debug(&format!("TTS сегмент {}/{} сгенерирован за {:.2?}", i + 1, subtitles.len(), duration));
                
                // Добавляем в кэш
                segments_cache.insert(subtitle.text.clone(), segment.clone());
                segment
            };
            
            tts_segments.push(segment);
            current_progress += progress_step;
        }
        
        log_info(&format!("Сгенерировано {} TTS сегментов, из них уникальных: {}", 
            tts_segments.len(), segments_cache.len()));
        
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
        
        for (i, (_segment, subtitle)) in tts_segments.iter().zip(subtitles.iter()).enumerate() {
            // Обновляем прогресс
            self.progress_tracker.update(
                current_progress,
                &format!("Анализ и корректировка сегмента {}/{}", i + 1, tts_segments.len())
            )?;
            
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
            
            // Создаем аудио сегмент
            let audio_segment = AudioSegment::new(
                adjusted_audio,
                subtitle.start_time,
                subtitle.end_time,
                subtitle.text.clone()
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
        log_debug(&format!("Сохранение аудио файла: {} сегментов, {} сэмплов", num_segments, total_samples));
        
        if num_segments == 0 || total_samples == 0 {
            log_error::<(), _>(
                &Error::new(ErrorType::AudioProcessingError, "Аудио трек пуст или не содержит сэмплов"),
                "Ошибка при сохранении аудио"
            )?;
            return Err(Error::new(ErrorType::AudioProcessingError, "Аудио трек пуст или не содержит сэмплов"));
        }
        
        // Определяем формат по расширению файла
        let ext = path.split('.').last().unwrap_or("mp3").to_lowercase();
        
        // Всегда сначала сохраняем в WAV, так как с ним проще работать
        let temp_wav_path = format!("{}.temp.wav", path);
        self.write_wav_file(&merged_audio, &temp_wav_path).await?;
        
        match ext.as_str() {
            "mp3" => {
                // Используем ffmpeg если доступен для конвертации WAV в MP3
                log_debug("Конвертация WAV в MP3 с помощью ffmpeg");
                
                let result = Command::new("ffmpeg")
                    .arg("-y") // Перезаписать выходной файл без вопросов
                    .arg("-i")
                    .arg(&temp_wav_path)
                    .arg("-codec:a")
                    .arg("libmp3lame")
                    .arg("-qscale:a")
                    .arg("2") // Высокое качество (0-9, где 0 - лучшее)
                    .arg(path)
                    .output();
                
                match result {
                    Ok(output) => {
                        if !output.status.success() {
                            // Если ffmpeg не удался, используем fallback метод
                            log_warning("ffmpeg не удалось выполнить конвертацию, использую альтернативный метод");
                            self.fallback_mp3_conversion(&temp_wav_path, path).await?;
                        } else {
                            log_debug(&format!("Успешно сконвертировано в MP3 с помощью ffmpeg: {}", path));
                        }
                    },
                    Err(e) => {
                        // Если ffmpeg не найден или не работает, используем fallback метод
                        log_warning(&format!("ffmpeg не найден или не работает ({}), использую альтернативный метод", e));
                        self.fallback_mp3_conversion(&temp_wav_path, path).await?;
                    }
                }
            },
            "wav" => {
                // WAV уже готов, просто переименовываем или копируем файл
                if temp_wav_path != path {
                    tokio::fs::copy(&temp_wav_path, path).await.map_err(|e| Error::new(
                        ErrorType::Io,
                        &format!("Не удалось скопировать WAV файл: {}", e)
                    ))?;
                }
                log_debug(&format!("Сохранен WAV файл: {}", path));
            },
            "ogg" => {
                // Используем ffmpeg если доступен для конвертации WAV в OGG
                log_debug("Конвертация WAV в OGG с помощью ffmpeg");
                
                let result = Command::new("ffmpeg")
                    .arg("-y")
                    .arg("-i")
                    .arg(&temp_wav_path)
                    .arg("-codec:a")
                    .arg("libvorbis")
                    .arg("-q:a")
                    .arg("4") // Качество (0-10, где 10 - лучшее)
                    .arg(path)
                    .output();
                
                match result {
                    Ok(output) => {
                        if !output.status.success() {
                            log_warning("ffmpeg не удалось выполнить конвертацию в OGG, сохраняю как WAV");
                            tokio::fs::copy(&temp_wav_path, path).await.map_err(|e| Error::new(
                                ErrorType::Io,
                                &format!("Не удалось скопировать WAV файл: {}", e)
                            ))?;
                        } else {
                            log_debug(&format!("Успешно сконвертировано в OGG с помощью ffmpeg: {}", path));
                        }
                    },
                    Err(e) => {
                        log_warning(&format!("ffmpeg не найден или не работает ({}), сохраняю как WAV", e));
                        tokio::fs::copy(&temp_wav_path, path).await.map_err(|e| Error::new(
                            ErrorType::Io,
                            &format!("Не удалось скопировать WAV файл: {}", e)
                        ))?;
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
        let _ = tokio::fs::remove_file(&temp_wav_path).await;
        
        Ok(())
    }
    
    /// Запасной метод для конвертации в MP3, если ffmpeg недоступен
    async fn fallback_mp3_conversion(&self, wav_path: &str, mp3_path: &str) -> Result<()> {
        // В реальном приложении здесь можно использовать встроенную библиотеку для кодирования MP3
        // Например, lame-rs, minimp3, etc.
        // В этом примере просто копируем WAV в MP3 и добавляем предупреждение
        log_warning("Используется резервный метод конвертации WAV в MP3 - файл может быть несовместим с плеерами");
        log_warning("Рекомендуется установить ffmpeg для правильной конвертации");
        
        // Создаем простой заголовок MP3 (это НЕ правильный MP3, но лучше чем ничего)
        let mut mp3_file = File::create(mp3_path).await.map_err(|e| Error::new(
            ErrorType::Io,
            &format!("Не удалось создать MP3 файл: {}", e)
        ))?;
        
        // Вместо копирования всего WAV как раньше, попробуем извлечь только PCM данные (пропуская WAV заголовок)
        let wav_data = tokio::fs::read(wav_path).await.map_err(|e| Error::new(
            ErrorType::Io,
            &format!("Не удалось прочитать WAV файл: {}", e)
        ))?;
        
        // Пропускаем WAV заголовок (44 байта) и записываем только данные
        if wav_data.len() > 44 {
            let pcm_data = &wav_data[44..];
            
            // Создаем простейший "необработанный" MP3 заголовок (не настоящий MP3)
            let mut mp3_header = vec![
                // ID3v2 заголовок
                b'I', b'D', b'3', // Идентификатор
                0x03, 0x00,       // Версия
                0x00,             // Флаги
                0x00, 0x00, 0x00, 0x0A, // Размер (10 байт)
                
                // Простой фрейм (текстовый)
                b'T', b'I', b'T', b'2', // Идентификатор фрейма (название)
                0x00, 0x00, 0x00, 0x01, // Размер (1 байт)
                0x00, 0x00,       // Флаги
                0x00              // Пустое значение
            ];
            
            // Записываем заголовок
            mp3_file.write_all(&mp3_header).await.map_err(|e| Error::new(
                ErrorType::Io,
                &format!("Не удалось записать MP3 заголовок: {}", e)
            ))?;
            
            // Записываем PCM данные (это не настоящий MP3, но может помочь отладить проблему)
            mp3_file.write_all(pcm_data).await.map_err(|e| Error::new(
                ErrorType::Io,
                &format!("Не удалось записать аудио данные: {}", e)
            ))?;
            
            log_debug(&format!("Записано {} байт в MP3 файл {} (резервный метод)", 
                mp3_header.len() + pcm_data.len(), mp3_path));
        } else {
            log_error::<(), _>(
                &Error::new(ErrorType::AudioProcessingError, "WAV файл слишком мал или повреждён"),
                "Ошибка при конвертации WAV в MP3"
            )?;
        }
        
        Ok(())
    }

    /// Записывает данные аудио в формате WAV
    async fn write_wav_file(&self, audio_data: &AudioData, path: &str) -> Result<()> {
        let mut file = File::create(path).await.map_err(|e| Error::new(
            ErrorType::Io,
            &format!("Не удалось создать WAV файл: {}", e)
        ))?;
        
        let total_samples = audio_data.samples.len();
        let num_channels = audio_data.channels;
        let sample_rate = audio_data.sample_rate;
        
        // Создаем заголовок WAV
        let data_size = (total_samples * 2) as u32; // 16-bit PCM, 2 байта на сэмпл
        let file_size = data_size + 36; // 44 байта заголовка - 8 байтов
        
        let mut header = vec![
            // RIFF chunk
            b'R', b'I', b'F', b'F',
            (file_size & 0xFF) as u8, ((file_size >> 8) & 0xFF) as u8, ((file_size >> 16) & 0xFF) as u8, ((file_size >> 24) & 0xFF) as u8,
            b'W', b'A', b'V', b'E',
            
            // fmt subchunk
            b'f', b'm', b't', b' ',
            16, 0, 0, 0, // размер подчанка fmt (16 байтов)
            1, 0, // аудио формат (1 = PCM)
            (num_channels & 0xFF) as u8, ((num_channels >> 8) & 0xFF) as u8, // количество каналов
            (sample_rate & 0xFF) as u8, ((sample_rate >> 8) & 0xFF) as u8, ((sample_rate >> 16) & 0xFF) as u8, ((sample_rate >> 24) & 0xFF) as u8, // частота дискретизации
        ];
        
        // Вычисляем и добавляем байт рейт и блок выравнивания
        let byte_rate = sample_rate * num_channels as u32 * 16 / 8;
        let block_align = num_channels as u16 * 16 / 8;
        
        header.extend_from_slice(&[
            // байт рейт = SampleRate * NumChannels * BitsPerSample / 8
            (byte_rate & 0xFF) as u8, ((byte_rate >> 8) & 0xFF) as u8, ((byte_rate >> 16) & 0xFF) as u8, ((byte_rate >> 24) & 0xFF) as u8,
            
            // блок выравнивания = NumChannels * BitsPerSample / 8
            (block_align & 0xFF) as u8, ((block_align >> 8) & 0xFF) as u8,
            
            16, 0, // биты на сэмпл
            
            // data subchunk
            b'd', b'a', b't', b'a',
            (data_size & 0xFF) as u8, ((data_size >> 8) & 0xFF) as u8, ((data_size >> 16) & 0xFF) as u8, ((data_size >> 24) & 0xFF) as u8,
        ]);
        
        // Записываем заголовок
        file.write_all(&header).await.map_err(|e| Error::new(
            ErrorType::Io,
            &format!("Не удалось записать заголовок WAV: {}", e)
        ))?;
        
        // Конвертируем float сэмплы в 16-bit PCM и записываем их
        let mut pcm_data = Vec::with_capacity(total_samples * 2);
        for &sample in &audio_data.samples {
            // Преобразуем float в int16, важно нормализовать значения правильно
            let pcm_sample = (sample.max(-1.0).min(1.0) * 32767.0) as i16;
            
            // Записываем в порядке little-endian (младший байт, затем старший)
            pcm_data.push((pcm_sample & 0xFF) as u8);
            pcm_data.push(((pcm_sample >> 8) & 0xFF) as u8);
        }
        
        file.write_all(&pcm_data).await.map_err(|e| Error::new(
            ErrorType::Io,
            &format!("Не удалось записать аудио данные: {}", e)
        ))?;
        
        log_debug(&format!("Записано {} байт в WAV файл {}", header.len() + pcm_data.len(), path));
        
        Ok(())
    }
}
