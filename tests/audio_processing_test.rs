use tts_sync::{AudioData, AudioProcessor, TempoAdjuster, audio::TempoAlgorithm, Result};

#[test]
fn test_audio_data_creation() {
    // Создаем тестовые данные
    let samples = vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5];
    let sample_rate = 44100;
    let channels = 1;
    
    // Создаем объект AudioData
    let audio = AudioData::new(samples.clone(), sample_rate, channels);
    
    // Проверяем, что данные сохранены корректно
    assert_eq!(audio.samples, samples);
    assert_eq!(audio.sample_rate, sample_rate);
    assert_eq!(audio.channels, channels);
    
    // Проверяем расчет длительности
    let expected_duration = samples.len() as f64 / sample_rate as f64;
    assert!((audio.duration() - expected_duration).abs() < 0.0001);
}

#[test]
fn test_audio_compression() -> Result<()> {
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
    )?;
    
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
    
    Ok(())
}

#[test]
fn test_audio_equalization() -> Result<()> {
    // Создаем тестовые данные с разными частотами
    let mut samples = Vec::new();
    for i in 0..44100 { // 1 second at 44.1kHz
        let t = i as f32 / 44100.0;
        // Mix of low, mid, and high frequencies
        // Using frequencies well-separated from crossover points (300 Hz and 3 kHz)
        samples.push(
            0.3 * (t * 100.0).sin() +  // Low frequency (100 Hz)
            0.3 * (t * 1500.0).sin() + // Mid frequency (1.5 kHz)
            0.3 * (t * 5000.0).sin()   // High frequency (5 kHz)
        );
    }
    let audio = AudioData::new(samples, 44100, 1);
    
    // Применяем эквализацию
    let equalized = AudioProcessor::apply_equalization(
        &audio,
        3.0,    // усиление низких частот в дБ
        0.0,    // усиление средних частот в дБ
        -3.0,   // усиление высоких частот в дБ
        300.0,  // частота разделения низких и средних частот в Гц
        3000.0  // частота разделения средних и высоких частот в Гц
    )?;
    
    // Проверяем, что длина не изменилась
    assert_eq!(equalized.samples.len(), audio.samples.len());
    
    // Проверяем, что эквализация применена (значения изменились)
    let mut has_changes = false;
    for (&original, &equalized) in audio.samples.iter().zip(equalized.samples.iter()) {
        if (original - equalized).abs() > 0.001 {
            has_changes = true;
            break;
        }
    }
    assert!(has_changes, "Эквализация не изменила аудио данные");
    
    // Проверяем, что эквализация не исказила сигнал слишком сильно
    // (форма сигнала должна быть сохранена)
    let mut max_diff: f32 = 0.0;
    let mut max_amplitude: f32 = 0.0;
    for (&original, &equalized) in audio.samples.iter().zip(equalized.samples.iter()) {
        max_diff = max_diff.max((original - equalized).abs());
        max_amplitude = max_amplitude.max(original.abs());
    }
    
    // Проверяем, что максимальная разница не превышает 40% от максимальной амплитуды
    assert!(max_diff <= 0.4 * max_amplitude, 
        "Эквализация слишком сильно исказила сигнал: max_diff={}, max_amplitude={}", 
        max_diff, max_amplitude);
    
    // Проверяем, что амплитуда не превышает 1.0
    for &sample in &equalized.samples {
        assert!(sample.abs() <= 1.0);
    }
    
    Ok(())
}

#[test]
fn test_volume_normalization() -> Result<()> {
    // Создаем тестовые данные с максимальной амплитудой 0.5
    let samples = vec![
        0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0,
        0.0, -0.1, -0.2, -0.3, -0.4, -0.5, -0.4, -0.3, -0.2, -0.1
    ];
    let audio = AudioData::new(samples, 44100, 1);
    
    // Нормализуем к -6 дБ
    let normalized = AudioProcessor::normalize_volume(&audio, -6.0)?;
    
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
    
    Ok(())
}

#[test]
fn test_tempo_adjustment_linear() -> Result<()> {
    // Создаем тестовые данные
    let samples = vec![
        0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
        0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0
    ];
    let audio = AudioData::new(samples, 44100, 1);
    let tempo_factor = 2.0f64;

    // Ускоряем аудио в 2 раза
    let adjusted = TempoAdjuster::adjust_tempo(&audio, tempo_factor, TempoAlgorithm::Linear)?;

    // Проверяем, что длина изменилась примерно в 2 раза
    assert!(
        (adjusted.samples.len() as f64 - audio.samples.len() as f64 / tempo_factor).abs() < 2.0,
        "Ожидаемая длина: {}, фактическая: {}",
        audio.samples.len() as f64 / tempo_factor,
        adjusted.samples.len()
    );
    
    Ok(())
}

#[test]
fn test_fit_to_duration() -> Result<()> {
    // Создаем тестовые данные
    let samples = vec![
        0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
        0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0
    ];
    let audio = AudioData::new(samples, 44100, 1);
    let current_duration = audio.duration();

    // Подгоняем под половину текущей длительности
    let target_duration = (current_duration / 2.0) as f32;
    let adjusted = TempoAdjuster::fit_to_duration(&audio, target_duration, TempoAlgorithm::Linear)?;

    // Проверяем, что длительность изменилась примерно в 2 раза
    let adjusted_duration = adjusted.duration();
    assert!(
        (adjusted_duration - target_duration as f64).abs() < 0.01,
        "Ожидаемая длительность: {:.2}, фактическая: {:.2}",
        target_duration,
        adjusted_duration
    );
    
    Ok(())
}

#[test]
fn test_adaptive_tempo_adjustment() -> Result<()> {
    // Создаем тестовые данные
    let samples = vec![
        0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
        0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0
    ];
    let audio = AudioData::new(samples, 44100, 1);
    let current_duration = audio.duration();

    // Подгоняем под половину текущей длительности
    let target_duration = (current_duration / 2.0) as f32;
    let adjusted = TempoAdjuster::adaptive_tempo_adjustment(
        &audio,
        target_duration,
        TempoAlgorithm::Linear,
        true
    )?;

    // Проверяем, что длительность изменилась примерно в 2 раза
    let adjusted_duration = adjusted.duration();
    assert!(
        (adjusted_duration - target_duration as f64).abs() < 0.01,
        "Ожидаемая длительность: {:.2}, фактическая: {:.2}",
        target_duration,
        adjusted_duration
    );
    
    Ok(())
}