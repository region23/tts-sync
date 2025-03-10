//! Модуль для обработки аудио (компрессия, эквализация)

use crate::error::{Error, Result, ErrorType};
use crate::audio::models::AudioData;
use crate::logging::{log_info, log_debug};

/// Процессор аудио для применения различных эффектов обработки
pub struct AudioProcessor;

impl AudioProcessor {
    /// Применяет компрессию динамического диапазона к аудио данным
    ///
    /// # Аргументы
    ///
    /// * `audio` - Аудио данные для обработки
    /// * `threshold` - Порог в дБ, выше которого начинается компрессия (отрицательное значение, например -20.0)
    /// * `ratio` - Коэффициент компрессии (например, 4.0 означает компрессию 4:1)
    /// * `attack` - Время атаки в миллисекундах
    /// * `release` - Время восстановления в миллисекундах
    /// * `makeup_gain` - Компенсационное усиление в дБ
    ///
    /// # Возвращает
    ///
    /// * `Result<AudioData>` - Обработанные аудио данные
    pub fn apply_compression(
        audio: &AudioData,
        threshold: f32,
        ratio: f32,
        attack: f32,
        release: f32,
        makeup_gain: f32,
    ) -> Result<AudioData> {
        log_info(&format!(
            "Применение компрессии: порог={} дБ, соотношение={}:1, атака={} мс, восстановление={} мс, усиление={} дБ",
            threshold, ratio, attack, release, makeup_gain
        ));

        if ratio <= 1.0 {
            return Err(Error::new(
                ErrorType::InvalidParameters,
                "Коэффициент компрессии должен быть больше 1.0",
            ));
        }

        // Конвертируем параметры в линейные значения
        let threshold_linear = 10.0_f32.powf(threshold / 20.0);
        let makeup_gain_linear = 10.0_f32.powf(makeup_gain / 20.0);
        
        // Рассчитываем константы времени для атаки и восстановления
        let attack_samples = (attack * 0.001 * audio.sample_rate as f32) as usize;
        let release_samples = (release * 0.001 * audio.sample_rate as f32) as usize;
        
        let attack_coef = if attack_samples > 0 {
            (-1.0 / attack_samples as f32).exp()
        } else {
            0.0
        };
        
        let release_coef = if release_samples > 0 {
            (-1.0 / release_samples as f32).exp()
        } else {
            0.0
        };
        
        // Создаем новый буфер для обработанных данных
        let mut processed_samples = Vec::with_capacity(audio.samples.len());
        
        // Переменные для отслеживания состояния компрессора
        let mut envelope = 0.0;
        
        // Обрабатываем каждый сэмпл
        for &sample in &audio.samples {
            // Вычисляем абсолютное значение сэмпла
            let abs_sample = sample.abs();
            
            // Обновляем огибающую (envelope)
            if abs_sample > envelope {
                // Атака - быстрое увеличение огибающей
                envelope = attack_coef * envelope + (1.0 - attack_coef) * abs_sample;
            } else {
                // Восстановление - медленное уменьшение огибающей
                envelope = release_coef * envelope + (1.0 - release_coef) * abs_sample;
            }
            
            // Вычисляем коэффициент усиления
            let gain;
            if envelope <= threshold_linear {
                // Ниже порога - без изменений
                gain = 1.0;
            } else {
                // Выше порога - применяем компрессию
                let slope = 1.0 / ratio;
                let db_above_threshold = 20.0 * (envelope / threshold_linear).log10();
                let db_gain_reduction = db_above_threshold * (1.0 - slope);
                gain = 10.0_f32.powf(-db_gain_reduction / 20.0);
            }
            
            // Применяем усиление и компенсацию
            let processed_sample = sample * gain * makeup_gain_linear;
            
            // Ограничиваем значение в диапазоне [-1.0, 1.0]
            let limited_sample = processed_sample.max(-1.0).min(1.0);
            
            processed_samples.push(limited_sample);
        }
        
        log_debug("Компрессия применена успешно");
        
        // Создаем новый объект AudioData с обработанными данными
        Ok(AudioData::new(
            processed_samples,
            audio.sample_rate,
            audio.channels,
        ))
    }

    /// Применяет эквализацию к аудио данным
    ///
    /// # Аргументы
    ///
    /// * `audio` - Аудио данные для обработки
    /// * `low_gain` - Усиление низких частот в дБ
    /// * `mid_gain` - Усиление средних частот в дБ
    /// * `high_gain` - Усиление высоких частот в дБ
    /// * `low_freq` - Частота разделения низких и средних частот в Гц
    /// * `high_freq` - Частота разделения средних и высоких частот в Гц
    ///
    /// # Возвращает
    ///
    /// * `Result<AudioData>` - Обработанные аудио данные
    pub fn apply_equalization(
        audio: &AudioData,
        low_gain: f32,
        mid_gain: f32,
        high_gain: f32,
        low_freq: f32,
        high_freq: f32,
    ) -> Result<AudioData> {
        log_info(&format!(
            "Применение эквализации: низкие={} дБ (до {} Гц), средние={} дБ, высокие={} дБ (от {} Гц)",
            low_gain, low_freq, mid_gain, high_freq, high_gain
        ));

        if audio.samples.is_empty() {
            return Err(Error::new(
                ErrorType::InvalidParameters,
                "Аудио данные пусты",
            ));
        }

        if low_freq >= high_freq {
            return Err(Error::new(
                ErrorType::InvalidParameters,
                "Частота разделения низких частот должна быть меньше частоты разделения высоких частот",
            ));
        }

        // Конвертируем усиление из дБ в линейные значения
        let low_gain_linear = 10.0_f32.powf(low_gain / 20.0);
        let mid_gain_linear = 10.0_f32.powf(mid_gain / 20.0);
        let high_gain_linear = 10.0_f32.powf(high_gain / 20.0);

        // Рассчитываем коэффициенты фильтров
        // Используем простые фильтры первого порядка для демонстрации
        let dt = 1.0 / audio.sample_rate as f32;
        let rc_low = 1.0 / (2.0 * std::f32::consts::PI * low_freq);
        let rc_high = 1.0 / (2.0 * std::f32::consts::PI * high_freq);
        
        let alpha_low = dt / (rc_low + dt);
        let alpha_high = dt / (rc_high + dt);

        // Создаем буферы для фильтрованных сигналов
        let mut low_pass = vec![0.0; audio.samples.len()];
        let mut high_pass = vec![0.0; audio.samples.len()];
        let mut band_pass = vec![0.0; audio.samples.len()];

        // Применяем фильтры
        // Низкочастотный фильтр
        low_pass[0] = audio.samples[0];
        for i in 1..audio.samples.len() {
            low_pass[i] = low_pass[i-1] + alpha_low * (audio.samples[i] - low_pass[i-1]);
        }

        // Высокочастотный фильтр (разница между входным сигналом и низкочастотным)
        high_pass[0] = audio.samples[0];
        for i in 1..audio.samples.len() {
            high_pass[i] = alpha_high * (high_pass[i-1] + audio.samples[i] - audio.samples[i-1]);
        }

        // Полосовой фильтр (средние частоты)
        // Это разница между входным сигналом и суммой низких и высоких частот
        for i in 0..audio.samples.len() {
            band_pass[i] = audio.samples[i] - (low_pass[i] + high_pass[i]);
        }

        // Применяем усиление и объединяем сигналы
        let mut processed_samples = Vec::with_capacity(audio.samples.len());
        for i in 0..audio.samples.len() {
            let eq_sample = low_pass[i] * low_gain_linear + 
                           band_pass[i] * mid_gain_linear + 
                           high_pass[i] * high_gain_linear;
            
            // Ограничиваем значение в диапазоне [-1.0, 1.0]
            let limited_sample = eq_sample.max(-1.0).min(1.0);
            
            processed_samples.push(limited_sample);
        }

        log_debug("Эквализация применена успешно");

        // Создаем новый объект AudioData с обработанными данными
        Ok(AudioData::new(
            processed_samples,
            audio.sample_rate,
            audio.channels,
        ))
    }

    /// Нормализует громкость аудио
    ///
    /// # Аргументы
    ///
    /// * `audio` - Аудио данные для обработки
    /// * `target_db` - Целевой уровень громкости в дБ (обычно -3.0 или -6.0)
    ///
    /// # Возвращает
    ///
    /// * `Result<AudioData>` - Обработанные аудио данные
    pub fn normalize_volume(audio: &AudioData, target_db: f32) -> Result<AudioData> {
        log_info(&format!("Нормализация громкости к {} дБ", target_db));

        if audio.samples.is_empty() {
            return Err(Error::new(
                ErrorType::InvalidParameters,
                "Аудио данные пусты",
            ));
        }

        // Находим максимальную амплитуду
        let max_amplitude = audio.samples.iter().fold(0.0f32, |max, &sample| {
            max.max(sample.abs())
        });

        if max_amplitude <= 0.0 {
            log_debug("Аудио содержит только тишину, нормализация не требуется");
            return Ok(audio.clone());
        }

        // Рассчитываем текущий уровень в дБ
        let current_db = 20.0 * max_amplitude.log10();
        
        // Рассчитываем необходимое усиление
        let gain_db = target_db - current_db;
        let gain_linear = 10.0_f32.powf(gain_db / 20.0);

        log_debug(&format!(
            "Текущий пик: {} ({:.2} дБ), целевой: {:.2} дБ, усиление: {:.2} дБ",
            max_amplitude, current_db, target_db, gain_db
        ));

        // Применяем усиление
        let processed_samples: Vec<f32> = audio.samples.iter()
            .map(|&sample| (sample * gain_linear).max(-1.0).min(1.0))
            .collect();

        // Создаем новый объект AudioData с обработанными данными
        Ok(AudioData::new(
            processed_samples,
            audio.sample_rate,
            audio.channels,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression() {
        // Создаем тестовые данные с разной амплитудой и резкими переходами
        let mut samples = Vec::new();
        for i in 0..44100 { // 1 second at 44.1kHz
            let t = i as f32 / 44100.0;
            if t < 0.25 {
                samples.push(0.1 * (t * 4.0).sin()); // Low amplitude
            } else if t < 0.5 {
                samples.push(0.8 * (t * 8.0).sin()); // High amplitude
            } else if t < 0.75 {
                samples.push(0.2 * (t * 12.0).sin()); // Medium amplitude
            } else {
                samples.push(0.9 * (t * 16.0).sin()); // Very high amplitude
            }
        }
        let audio = AudioData::new(samples, 44100, 1);

        // Применяем компрессию
        let compressed = AudioProcessor::apply_compression(
            &audio,
            -20.0, // порог в дБ
            4.0,   // соотношение
            10.0,  // атака в мс
            100.0, // восстановление в мс
            6.0    // компенсационное усиление в дБ
        ).unwrap();

        // Проверяем, что длина не изменилась
        assert_eq!(compressed.samples.len(), audio.samples.len());
        
        // Проверяем, что компрессия применена (значения изменились)
        let mut has_changes = false;
        for (&original, &compressed) in audio.samples.iter().zip(compressed.samples.iter()) {
            if (original - compressed).abs() > 0.001 {
                has_changes = true;
                break;
            }
        }
        assert!(has_changes, "Компрессия не изменила аудио данные");
        
        // Проверяем, что сжатие не исказило сигнал слишком сильно
        // (форма сигнала должна быть сохранена)
        for (&original, &compressed) in audio.samples.iter().zip(compressed.samples.iter()) {
            // Проверяем, что знак сигнала сохранен
            assert_eq!(original.signum(), compressed.signum());
            // Проверяем, что амплитуда не превышает 1.0
            assert!(compressed.abs() <= 1.0);
        }
    }

    #[test]
    fn test_equalization() {
        // Создаем тестовые данные
        let samples = vec![
            0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
            0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0
        ];
        let audio = AudioData::new(samples, 44100, 1);

        // Применяем эквализацию
        let equalized = AudioProcessor::apply_equalization(
            &audio,
            3.0,    // усиление низких частот в дБ
            0.0,    // усиление средних частот в дБ
            -3.0,   // усиление высоких частот в дБ
            300.0,  // частота разделения низких и средних частот в Гц
            3000.0  // частота разделения средних и высоких частот в Гц
        ).unwrap();

        // Проверяем, что длина не изменилась
        assert_eq!(equalized.samples.len(), audio.samples.len());
    }

    #[test]
    fn test_normalize_volume() {
        // Создаем тестовые данные с максимальной амплитудой 0.5
        let samples = vec![
            0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0,
            0.0, -0.1, -0.2, -0.3, -0.4, -0.5, -0.4, -0.3, -0.2, -0.1
        ];
        let audio = AudioData::new(samples, 44100, 1);

        // Нормализуем к -6 дБ
        let normalized = AudioProcessor::normalize_volume(&audio, -6.0).unwrap();

        // Проверяем, что длина не изменилась
        assert_eq!(normalized.samples.len(), audio.samples.len());

        // Находим максимальную амплитуду нормализованного аудио
        let max_amplitude = normalized.samples.iter().fold(0.0f32, |max, &sample| {
            max.max(sample.abs())
        });

        // Проверяем, что максимальная амплитуда близка к целевому значению
        // -6 дБ соответствует амплитуде примерно 0.5012 (10^(-6/20))
        let target_amplitude = 10.0f32.powf(-6.0 / 20.0);
        assert!((max_amplitude - target_amplitude).abs() < 0.01, 
                "Ожидаемая амплитуда: {}, фактическая: {}", target_amplitude, max_amplitude);
    }
}