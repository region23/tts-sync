use crate::error::{Error, Result};
use crate::audio::models::{AudioData, AudioSegment};

/// Анализатор аудио
pub struct AudioAnalyzer;

impl AudioAnalyzer {
    /// Анализирует аудио данные и возвращает информацию о них
    pub fn analyze(audio: &AudioData) -> Result<AudioAnalysis> {
        if audio.is_empty() {
            return Err(Error::AudioProcessing("Cannot analyze empty audio".to_string()));
        }

        // Вычисляем RMS (Root Mean Square) для определения громкости
        let rms = Self::calculate_rms(&audio.samples);
        
        // Находим пики (максимальную амплитуду)
        let peak = Self::find_peak_amplitude(&audio.samples);
        
        // Определяем паузы в аудио
        let silence_threshold = rms * 0.1; // 10% от RMS как порог тишины
        let silences = Self::detect_silences(&audio.samples, silence_threshold, audio.sample_rate);
        
        // Определяем темп речи (слогов в секунду)
        // Это приблизительная оценка, для точного определения нужен более сложный алгоритм
        let speech_rate = if silences.is_empty() {
            10.0 // Значение по умолчанию, если не удалось определить паузы
        } else {
            // Предполагаем, что между паузами в среднем 2 слога
            let speech_segments = silences.len() + 1;
            (speech_segments * 2) as f32 / audio.duration() as f32
        };
        
        Ok(AudioAnalysis {
            duration: audio.duration(),
            rms,
            peak,
            silences,
            speech_rate,
        })
    }
    
    /// Вычисляет RMS (Root Mean Square) для аудио сэмплов
    fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        
        let sum_squares: f32 = samples.iter()
            .map(|&s| s * s)
            .sum();
            
        (sum_squares / samples.len() as f32).sqrt()
    }
    
    /// Находит максимальную амплитуду в аудио сэмплах
    fn find_peak_amplitude(samples: &[f32]) -> f32 {
        samples.iter()
            .fold(0.0, |max, &s| max.max(s.abs()))
    }
    
    /// Определяет паузы в аудио
    fn detect_silences(samples: &[f32], threshold: f32, sample_rate: u32) -> Vec<SilenceSegment> {
        let min_silence_samples = (0.1 * sample_rate as f64) as usize; // Минимум 100 мс тишины
        
        let mut silences = Vec::new();
        let mut silence_start: Option<usize> = None;
        
        for (i, &sample) in samples.iter().enumerate() {
            if sample.abs() < threshold {
                // Начало тишины
                if silence_start.is_none() {
                    silence_start = Some(i);
                }
            } else if let Some(start) = silence_start {
                // Конец тишины
                let silence_length = i - start;
                if silence_length >= min_silence_samples {
                    silences.push(SilenceSegment {
                        start_sample: start,
                        end_sample: i,
                        start_time: start as f64 / sample_rate as f64,
                        end_time: i as f64 / sample_rate as f64,
                    });
                }
                silence_start = None;
            }
        }
        
        // Проверяем, не закончился ли файл тишиной
        if let Some(start) = silence_start {
            let silence_length = samples.len() - start;
            if silence_length >= min_silence_samples {
                silences.push(SilenceSegment {
                    start_sample: start,
                    end_sample: samples.len(),
                    start_time: start as f64 / sample_rate as f64,
                    end_time: samples.len() as f64 / sample_rate as f64,
                });
            }
        }
        
        silences
    }
    
    /// Анализирует сегмент аудио и определяет оптимальный коэффициент растяжения/сжатия
    pub fn analyze_segment(segment: &AudioSegment, target_duration: f64) -> Result<SegmentAnalysis> {
        if segment.audio.is_empty() {
            return Err(Error::AudioProcessing("Cannot analyze empty audio segment".to_string()));
        }
        
        let audio_analysis = Self::analyze(&segment.audio)?;
        let current_duration = segment.audio.duration();
        
        // Базовый коэффициент растяжения/сжатия
        let base_stretch_factor = target_duration / current_duration;
        
        // Ограничиваем коэффициент, чтобы избежать слишком сильного искажения
        let min_stretch = 0.5;
        let max_stretch = 2.0;
        let stretch_factor = base_stretch_factor.max(min_stretch).min(max_stretch);
        
        // Определяем, нужно ли разбивать сегмент на части
        let should_split = audio_analysis.silences.len() > 1 && 
                          (stretch_factor < 0.7 || stretch_factor > 1.3);
        
        Ok(SegmentAnalysis {
            audio_analysis,
            current_duration,
            target_duration,
            stretch_factor,
            should_split,
        })
    }
    
    /// Разбивает сегмент аудио на части по паузам
    pub fn split_segment(segment: &AudioSegment) -> Result<Vec<AudioSegment>> {
        let audio_analysis = Self::analyze(&segment.audio)?;
        
        if audio_analysis.silences.is_empty() {
            return Ok(vec![segment.clone()]);
        }
        
        let mut segments = Vec::new();
        let mut start_sample = 0;
        let mut start_time = segment.start_time;
        
        for silence in &audio_analysis.silences {
            // Берем середину паузы как точку разделения
            let split_sample = (silence.start_sample + silence.end_sample) / 2;
            let _split_time = silence.start_time + (silence.end_time - silence.start_time) / 2.0;
            
            // Создаем сегмент до паузы
            let samples = segment.audio.samples[start_sample..split_sample].to_vec();
            let audio = AudioData::new(
                samples,
                segment.audio.sample_rate,
                segment.audio.channels
            );
            
            let end_time = start_time + audio.duration();
            
            segments.push(AudioSegment::new(
                audio,
                start_time,
                end_time,
                segment.text.clone()
            ));
            
            // Обновляем начальную позицию для следующего сегмента
            start_sample = split_sample;
            start_time = end_time;
        }
        
        // Добавляем последний сегмент
        if start_sample < segment.audio.samples.len() {
            let samples = segment.audio.samples[start_sample..].to_vec();
            let audio = AudioData::new(
                samples,
                segment.audio.sample_rate,
                segment.audio.channels
            );
            
            let end_time = start_time + audio.duration();
            
            segments.push(AudioSegment::new(
                audio,
                start_time,
                end_time,
                segment.text.clone()
            ));
        }
        
        Ok(segments)
    }
}

/// Сегмент тишины в аудио
#[derive(Debug, Clone)]
pub struct SilenceSegment {
    /// Индекс начального сэмпла
    pub start_sample: usize,
    /// Индекс конечного сэмпла
    pub end_sample: usize,
    /// Время начала в секундах
    pub start_time: f64,
    /// Время окончания в секундах
    pub end_time: f64,
}

impl SilenceSegment {
    /// Возвращает длительность тишины в секундах
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }
    
    /// Возвращает длительность тишины в сэмплах
    pub fn samples_count(&self) -> usize {
        self.end_sample - self.start_sample
    }
}

/// Результат анализа аудио
#[derive(Debug, Clone)]
pub struct AudioAnalysis {
    /// Длительность аудио в секундах
    pub duration: f64,
    /// RMS (Root Mean Square) - мера громкости
    pub rms: f32,
    /// Пиковая амплитуда
    pub peak: f32,
    /// Сегменты тишины
    pub silences: Vec<SilenceSegment>,
    /// Темп речи (слогов в секунду)
    pub speech_rate: f32,
}

/// Результат анализа сегмента аудио
#[derive(Debug, Clone)]
pub struct SegmentAnalysis {
    /// Анализ аудио
    pub audio_analysis: AudioAnalysis,
    /// Текущая длительность в секундах
    pub current_duration: f64,
    /// Целевая длительность в секундах
    pub target_duration: f64,
    /// Коэффициент растяжения/сжатия
    pub stretch_factor: f64,
    /// Нужно ли разбивать сегмент на части
    pub should_split: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calculate_rms() {
        let samples = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let rms = AudioAnalyzer::calculate_rms(&samples);
        assert!((rms - 0.7071).abs() < 0.001);
    }
    
    #[test]
    fn test_find_peak_amplitude() {
        let samples = vec![0.0, 0.5, -0.8, 0.3, -0.2];
        let peak = AudioAnalyzer::find_peak_amplitude(&samples);
        assert_eq!(peak, 0.8);
    }
    
    #[test]
    fn test_detect_silences() {
        let mut samples = vec![0.0; 44100]; // 1 секунда тишины
        
        // Добавляем шум в середине
        for i in 22050..22150 {
            samples[i] = 0.5;
        }
        
        let silences = AudioAnalyzer::detect_silences(&samples, 0.1, 44100);
        
        // Должно быть 2 сегмента тишины
        assert_eq!(silences.len(), 2);
        
        // Проверяем первый сегмент
        assert_eq!(silences[0].start_sample, 0);
        assert_eq!(silences[0].end_sample, 22050);
        
        // Проверяем второй сегмент
        assert_eq!(silences[1].start_sample, 22150);
        assert_eq!(silences[1].end_sample, 44100);
    }
}
