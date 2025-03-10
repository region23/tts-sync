use crate::error::{Error, Result};
use std::path::Path;
use crate::logging::{log_debug, log_info, log_warning};

/// Аудио данные
#[derive(Debug, Clone)]
pub struct AudioData {
    /// Аудио сэмплы (моно)
    pub samples: Vec<f32>,
    /// Частота дискретизации
    pub sample_rate: u32,
    /// Количество каналов
    pub channels: u16,
}

impl AudioData {
    /// Создает новые аудио данные
    pub fn new(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        Self {
            samples,
            sample_rate,
            channels,
        }
    }

    /// Возвращает длительность аудио в секундах
    pub fn duration(&self) -> f64 {
        self.samples.len() as f64 / self.sample_rate as f64 / self.channels as f64
    }

    /// Возвращает количество сэмплов
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Проверяет, пусты ли аудио данные
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Нормализует громкость аудио
    pub fn normalize(&mut self, target_peak: f32) {
        if self.is_empty() {
            return;
        }

        let max_amplitude: f32 = self.samples.iter()
            .fold(0.0f32, |max, &sample| max.max(sample.abs()));

        if max_amplitude > 0.0 {
            let gain = target_peak / max_amplitude;
            for sample in &mut self.samples {
                *sample *= gain;
            }
        }
    }

    /// Изменяет частоту дискретизации аудио
    pub fn resample(&self, new_sample_rate: u32) -> Result<Self> {
        if self.sample_rate == new_sample_rate {
            return Ok(self.clone());
        }

        // Здесь будет использоваться библиотека rubato для ресемплинга
        // Реализация будет добавлена позже
        Err(Error::AudioProcessing("Resampling not implemented yet".to_string()))
    }

    /// Загружает аудио данные из файла
    pub fn from_file<P: AsRef<Path>>(_path: P) -> Result<Self> {
        // Здесь будет использоваться библиотека symphonia для загрузки аудио
        // Реализация будет добавлена позже
        Err(Error::AudioProcessing("Loading from file not implemented yet".to_string()))
    }

    /// Сохраняет аудио данные в файл
    pub fn to_file<P: AsRef<Path>>(&self, _path: P) -> Result<()> {
        // Здесь будет использоваться библиотека hound для сохранения аудио
        // Реализация будет добавлена позже
        Err(Error::AudioProcessing("Saving to file not implemented yet".to_string()))
    }

    /// Применяет компрессию динамического диапазона
    pub fn apply_compression(
        &self,
        threshold: f32,
        ratio: f32,
        attack: f32,
        release: f32,
        makeup_gain: f32,
    ) -> Result<Self> {
        if self.is_empty() {
            return Ok(self.clone());
        }

        let mut result = self.clone();
        let mut envelope = 0.0f32;
        let attack_coeff = (-1.0 / (attack * self.sample_rate as f32 / 1000.0)).exp();
        let release_coeff = (-1.0 / (release * self.sample_rate as f32 / 1000.0)).exp();

        for sample in &mut result.samples {
            // Вычисляем огибающую
            let input_level = sample.abs();
            if input_level > envelope {
                envelope = attack_coeff * envelope + (1.0 - attack_coeff) * input_level;
            } else {
                envelope = release_coeff * envelope + (1.0 - release_coeff) * input_level;
            }

            // Применяем компрессию
            if envelope > threshold {
                let gain_reduction = (threshold + (envelope - threshold) / ratio) / envelope;
                *sample *= gain_reduction;
            }
        }

        // Применяем makeup gain
        for sample in &mut result.samples {
            *sample *= makeup_gain;
        }

        Ok(result)
    }

    /// Применяет эквализацию
    pub fn apply_equalization(
        &self,
        _low_gain: f32,
        _mid_gain: f32,
        _high_gain: f32,
        _low_freq: f32,
        _high_freq: f32,
    ) -> Result<Self> {
        if self.is_empty() {
            return Ok(self.clone());
        }

        // Здесь будет реализация эквализации с использованием FIR фильтров
        // Пока просто возвращаем копию
        Ok(self.clone())
    }

    /// Нормализует громкость в дБ
    pub fn normalize_db(&self, target_db: f32) -> Self {
        if self.is_empty() {
            return self.clone();
        }

        let mut result = self.clone();
        let target_amplitude = 10.0f32.powf(target_db / 20.0);
        result.normalize(target_amplitude);
        result
    }
}

/// Сегмент аудио
#[derive(Debug, Clone)]
pub struct AudioSegment {
    /// Аудио данные
    pub audio: AudioData,
    /// Время начала сегмента в секундах
    pub start_time: f64,
    /// Время окончания сегмента в секундах
    pub end_time: f64,
    /// Текст сегмента
    pub text: String,
}

impl AudioSegment {
    /// Создает новый сегмент аудио
    pub fn new(audio: AudioData, start_time: f64, end_time: f64, text: String) -> Self {
        Self {
            audio,
            start_time,
            end_time,
            text,
        }
    }

    /// Возвращает длительность сегмента в секундах
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }

    /// Изменяет темп сегмента без изменения высоты тона
    pub fn adjust_tempo(&self, tempo_factor: f64) -> Result<Self> {
        if tempo_factor <= 0.0 {
            return Err(Error::InvalidParameters(format!("Invalid tempo factor: {}", tempo_factor)));
        }

        if (tempo_factor - 1.0).abs() < 0.01 {
            return Ok(self.clone());
        }

        // Здесь будет использоваться библиотека rubato для изменения темпа
        // Реализация будет добавлена позже
        Err(Error::AudioProcessing("Tempo adjustment not implemented yet".to_string()))
    }
}

/// Аудио трек
#[derive(Debug, Clone)]
pub struct AudioTrack {
    /// Сегменты аудио
    pub segments: Vec<AudioSegment>,
    /// Частота дискретизации
    pub sample_rate: u32,
    /// Количество каналов
    pub channels: u16,
}

impl AudioTrack {
    /// Создает новый пустой аудио трек
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            segments: Vec::new(),
            sample_rate,
            channels,
        }
    }

    /// Добавляет сегмент в трек
    pub fn add_segment(&mut self, segment: AudioSegment) {
        self.segments.push(segment);
    }

    /// Возвращает количество сегментов
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Проверяет, пуст ли трек
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Сортирует сегменты по времени начала
    pub fn sort_by_start_time(&mut self) {
        self.segments.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
    }

    /// Объединяет все сегменты в один аудио файл
    pub fn merge(&self) -> Result<AudioData> {
        if self.is_empty() {
            log_warning("Попытка объединить пустой аудио трек");
            return Ok(AudioData::new(Vec::new(), self.sample_rate, self.channels));
        }

        // Определяем общую длительность
        let min_start = self.segments.iter()
            .map(|s| s.start_time)
            .fold(f64::INFINITY, f64::min);
            
        let max_end = self.segments.iter()
            .map(|s| s.end_time)
            .fold(0.0, f64::max);
            
        let total_duration = max_end - min_start;
        let total_samples = (total_duration * self.sample_rate as f64 * self.channels as f64) as usize;
        
        log_debug(&format!("Объединение {} аудио сегментов, общая длительность: {:.2}с, всего сэмплов: {}", 
            self.segments.len(), total_duration, total_samples));
        
        if total_samples == 0 {
            log_warning("Объединение привело к нулевому количеству сэмплов");
            return Ok(AudioData::new(Vec::new(), self.sample_rate, self.channels));
        }
        
        let mut merged_samples = vec![0.0; total_samples];
        
        // Объединяем сегменты
        for (i, segment) in self.segments.iter().enumerate() {
            let start_sample = ((segment.start_time - min_start) * self.sample_rate as f64 * self.channels as f64) as usize;
            let num_samples = segment.audio.samples.len();
            
            log_debug(&format!("Сегмент {}/{}: старт: {:.2}с, длительность: {:.2}с, сэмплов: {}", 
                i + 1, self.segments.len(), segment.start_time, segment.audio.duration(), num_samples));
            
            if num_samples == 0 {
                log_warning(&format!("Сегмент {}/{} не содержит сэмплов", i + 1, self.segments.len()));
                continue;
            }
            
            for (j, &sample) in segment.audio.samples.iter().enumerate() {
                let pos = start_sample + j;
                if pos < merged_samples.len() {
                    merged_samples[pos] = sample;
                } else {
                    log_warning(&format!("Выход за пределы буфера при объединении сегмента {}/{}: позиция {} >= {}", 
                        i + 1, self.segments.len(), pos, merged_samples.len()));
                    break;
                }
            }
        }
        
        let result = AudioData::new(merged_samples, self.sample_rate, self.channels);
        log_info(&format!("Успешно объединено {} сегментов в один аудио файл длительностью {:.2}с", 
            self.segments.len(), result.duration()));
        
        Ok(result)
    }
}

impl Default for AudioTrack {
    fn default() -> Self {
        Self::new(44100, 1) // Стандартные значения по умолчанию
    }
}

impl AudioTrack {
    /// Нормализует громкость всех сегментов
    pub fn normalize_volume(&mut self) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }

        // Сначала объединяем все сегменты
        let merged_audio = self.merge()?;
        
        // Находим максимальную амплитуду
        let max_amplitude: f32 = merged_audio.samples.iter()
            .fold(0.0f32, |max, &sample| max.max(sample.abs()));
            
        if max_amplitude > 0.0 {
            // Нормализуем каждый сегмент
            for segment in &mut self.segments {
                segment.audio.normalize(1.0);
            }
        }
        
        Ok(())
    }
}
