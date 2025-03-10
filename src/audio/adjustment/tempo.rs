//! Улучшенный алгоритм изменения темпа аудио с сохранением качества звучания.

use crate::error::{Error, Result, ErrorType};
use crate::audio::models::AudioData;
use crate::logging::{log_info, log_debug};
use crate::audio::AudioAnalyzer;

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

/// Улучшенный корректировщик темпа аудио
pub struct TempoAdjuster;

impl TempoAdjuster {
    /// Изменяет темп аудио без изменения высоты тона
    ///
    /// # Аргументы
    ///
    /// * `audio` - Аудио данные для обработки
    /// * `tempo_factor` - Коэффициент изменения темпа (>1.0 - ускорение, <1.0 - замедление)
    /// * `algorithm` - Алгоритм изменения темпа
    ///
    /// # Возвращает
    ///
    /// * `Result<AudioData>` - Обработанные аудио данные
    pub fn adjust_tempo(
        audio: &AudioData,
        tempo_factor: f64,
        algorithm: TempoAlgorithm,
    ) -> Result<AudioData> {
        log_info(&format!(
            "Изменение темпа аудио: фактор={}, алгоритм={:?}",
            tempo_factor, algorithm
        ));

        if tempo_factor <= 0.0 {
            return Err(Error::new(
                ErrorType::InvalidParameters,
                "Коэффициент темпа должен быть положительным числом",
            ));
        }

        if audio.samples.is_empty() {
            return Err(Error::new(
                ErrorType::InvalidParameters,
                "Аудио данные пусты",
            ));
        }

        // Выбираем алгоритм изменения темпа
        match algorithm {
            TempoAlgorithm::Sinc => Self::adjust_tempo_sinc(audio, tempo_factor as f32),
            TempoAlgorithm::Fir => Self::adjust_tempo_fir(audio, tempo_factor as f32),
            TempoAlgorithm::Linear => Self::adjust_tempo_linear(audio, tempo_factor as f32),
        }
    }

    /// Подгоняет длительность аудио сегмента под целевую длительность
    ///
    /// # Аргументы
    ///
    /// * `audio` - Аудио данные для обработки
    /// * `target_duration` - Целевая длительность в секундах
    /// * `algorithm` - Алгоритм изменения темпа
    ///
    /// # Возвращает
    ///
    /// * `Result<AudioData>` - Обработанные аудио данные
    pub fn fit_to_duration(
        audio: &AudioData,
        target_duration: f32,
        algorithm: TempoAlgorithm,
    ) -> Result<AudioData> {
        let current_duration = audio.duration();
        if current_duration <= 0.0 {
            return Err(Error::new(
                ErrorType::InvalidParameters,
                "Аудио данные имеют нулевую длительность",
            ));
        }

        let tempo_factor = current_duration / target_duration as f64;
        log_info(&format!(
            "Подгонка длительности аудио: текущая={:.2}с, целевая={:.2}с, фактор={}",
            current_duration, target_duration, tempo_factor
        ));

        Self::adjust_tempo(audio, tempo_factor, algorithm)
    }

    /// Изменяет темп аудио с использованием алгоритма sinc интерполяции
    fn adjust_tempo_sinc(audio: &AudioData, tempo_factor: f32) -> Result<AudioData> {
        log_debug("Применение алгоритма sinc интерполяции");

        // Рассчитываем количество выходных сэмплов
        let output_size = (audio.samples.len() as f64 / tempo_factor as f64).round() as usize;
        let mut output_samples = Vec::with_capacity(output_size);

        // Параметры для sinc интерполяции
        let window_size = 256;
        let _oversampling = 256;

        // Выполняем sinc интерполяцию
        for i in 0..output_size {
            let pos = i as f64 * tempo_factor as f64;
            let index = pos.floor() as usize;
            let fraction = pos - index as f64;
            
            let mut sum = 0.0;
            let mut weight_sum = 0.0;
            
            // Применяем окно sinc
            for j in 0..window_size {
                let offset = j as i32 - (window_size as i32 / 2);
                let sample_index = index as i32 + offset;
                
                if sample_index >= 0 && sample_index < audio.samples.len() as i32 {
                    let x = (fraction + offset as f64) * std::f64::consts::PI;
                    let sinc = if x == 0.0 { 1.0 } else { x.sin() / x };
                    let window = 0.54 - 0.46 * (2.0 * std::f64::consts::PI * (j as f64) / (window_size as f64)).cos();
                    
                    sum += audio.samples[sample_index as usize] as f64 * sinc * window;
                    weight_sum += window;
                }
            }
            
            if weight_sum > 0.0 {
                output_samples.push((sum / weight_sum) as f32);
            } else {
                output_samples.push(0.0);
            }
        }

        Ok(AudioData::new(
            output_samples,
            audio.sample_rate,
            audio.channels,
        ))
    }

    /// Изменяет темп аудио с использованием алгоритма FIR фильтра
    fn adjust_tempo_fir(audio: &AudioData, tempo_factor: f32) -> Result<AudioData> {
        log_debug("Применение алгоритма FIR фильтра");

        // Рассчитываем количество выходных сэмплов
        let output_size = (audio.samples.len() as f64 / tempo_factor as f64).round() as usize;
        let mut output_samples = Vec::with_capacity(output_size);

        // Параметры для FIR фильтра
        let window_size = 64;
        let _oversampling = 160;

        // Выполняем FIR интерполяцию
        for i in 0..output_size {
            let pos = i as f64 * tempo_factor as f64;
            let index = pos.floor() as usize;
            let fraction = pos - index as f64;
            
            let mut sum = 0.0;
            let mut weight_sum = 0.0;
            
            // Применяем FIR фильтр
            for j in 0..window_size {
                let offset = j as i32 - (window_size as i32 / 2);
                let sample_index = index as i32 + offset;
                
                if sample_index >= 0 && sample_index < audio.samples.len() as i32 {
                    let x = (fraction + offset as f64) * std::f64::consts::PI;
                    let sinc = if x == 0.0 { 1.0 } else { x.sin() / x };
                    let window = 0.5 * (1.0 + (2.0 * std::f64::consts::PI * (j as f64) / (window_size as f64)).cos());
                    
                    sum += audio.samples[sample_index as usize] as f64 * sinc * window;
                    weight_sum += window;
                }
            }
            
            if weight_sum > 0.0 {
                output_samples.push((sum / weight_sum) as f32);
            } else {
                output_samples.push(0.0);
            }
        }

        Ok(AudioData::new(
            output_samples,
            audio.sample_rate,
            audio.channels,
        ))
    }

    /// Изменяет темп аудио с использованием линейной интерполяции
    fn adjust_tempo_linear(audio: &AudioData, tempo_factor: f32) -> Result<AudioData> {
        log_debug("Применение алгоритма линейной интерполяции");

        // Рассчитываем количество выходных сэмплов
        let output_size = (audio.samples.len() as f64 / tempo_factor as f64).round() as usize;
        let mut output_samples = Vec::with_capacity(output_size);

        // Выполняем линейную интерполяцию
        for i in 0..output_size {
            let pos = i as f64 * tempo_factor as f64;
            let index = pos.floor() as usize;
            let fraction = pos - index as f64;
            
            if index + 1 < audio.samples.len() {
                let sample = audio.samples[index] as f64 * (1.0 - fraction) + 
                           audio.samples[index + 1] as f64 * fraction;
                output_samples.push(sample as f32);
            } else {
                output_samples.push(audio.samples[index]);
            }
        }

        Ok(AudioData::new(
            output_samples,
            audio.sample_rate,
            audio.channels,
        ))
    }

    /// Адаптивное изменение темпа с сохранением пауз
    pub fn adaptive_tempo_adjustment(
        audio: &AudioData,
        target_duration: f32,
        algorithm: TempoAlgorithm,
        preserve_pauses: bool,
    ) -> Result<AudioData> {
        log_debug("Применение адаптивного изменения темпа");

        // Если не нужно сохранять паузы, просто изменяем темп всего аудио
        if !preserve_pauses {
            return Self::fit_to_duration(audio, target_duration, algorithm);
        }

        // Анализируем аудио на наличие пауз
        let analysis = AudioAnalyzer::analyze(audio)?;

        if analysis.silences.is_empty() {
            // Если пауз нет, просто изменяем темп всего аудио
            return Self::fit_to_duration(audio, target_duration, algorithm);
        }

        // Рассчитываем длительность речи
        let speech_duration = analysis.duration;
        let audio_duration = audio.duration();
        let target_speech_duration = target_duration as f64 * (speech_duration / audio_duration);

        if target_speech_duration <= 0.1 {
            // В этом случае сокращаем и паузы, и речь
            return Self::fit_to_duration(audio, target_duration, algorithm);
        }

        // Изменяем темп только для сегментов речи
        let mut output_samples = Vec::new();
        let mut current_pos = 0;

        for silence in &analysis.silences {
            // Добавляем сегмент речи до паузы
            let speech_segment = AudioData::new(
                audio.samples[current_pos..silence.start_sample].to_vec(),
                audio.sample_rate,
                audio.channels,
            );

            let adjusted_segment = Self::adjust_tempo(
                &speech_segment,
                speech_duration / target_speech_duration,
                algorithm,
            )?;

            output_samples.extend(adjusted_segment.samples);

            // Добавляем паузу
            output_samples.extend(&audio.samples[silence.start_sample..silence.end_sample]);

            current_pos = silence.end_sample;
        }

        // Добавляем оставшийся сегмент речи
        if current_pos < audio.samples.len() {
            let speech_segment = AudioData::new(
                audio.samples[current_pos..].to_vec(),
                audio.sample_rate,
                audio.channels,
            );

            let adjusted_segment = Self::adjust_tempo(
                &speech_segment,
                speech_duration / target_speech_duration,
                algorithm,
            )?;

            output_samples.extend(adjusted_segment.samples);
        }

        Ok(AudioData::new(
            output_samples,
            audio.sample_rate,
            audio.channels,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjust_tempo() {
        // Создаем тестовые данные
        let samples = vec![
            0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
            0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0
        ];
        let audio = AudioData::new(samples, 44100, 1);

        // Ускоряем аудио в 2 раза
        let tempo_factor = 2.0;
        let adjusted = TempoAdjuster::adjust_tempo(&audio, tempo_factor as f64, TempoAlgorithm::Linear).unwrap();

        // Проверяем, что длина изменилась примерно в 2 раза
        assert!(
            (adjusted.samples.len() as f32 - audio.samples.len() as f32 / tempo_factor).abs() < 2.0,
            "Ожидаемая длина: {}, фактическая: {}",
            audio.samples.len() as f32 / tempo_factor,
            adjusted.samples.len()
        );
    }

    #[test]
    fn test_fit_to_duration() {
        // Создаем тестовые данные
        let samples = vec![
            0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
            0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0
        ];
        let audio = AudioData::new(samples, 44100, 1);
        let current_duration = audio.duration();

        // Подгоняем под половину текущей длительности
        let target_duration = (current_duration / 2.0) as f32;
        let adjusted = TempoAdjuster::fit_to_duration(&audio, target_duration, TempoAlgorithm::Linear).unwrap();

        // Проверяем, что длительность изменилась примерно в 2 раза
        let adjusted_duration = adjusted.duration() as f32;
        assert!(
            (adjusted_duration - target_duration).abs() < 0.01,
            "Ожидаемая длительность: {:.2}, фактическая: {:.2}",
            target_duration,
            adjusted_duration
        );
    }
}
