# Алгоритм TTS-Sync

## Обзор

Алгоритм TTS-Sync предназначен для создания синхронизированной аудиодорожки на основе переведенных субтитров, где русская озвучка максимально точно соответствует таймингу оригинального видео и субтитров. Алгоритм использует OpenAI API для генерации TTS и анализирует тайминги субтитров для создания синхронизированной аудиодорожки.

## Основные компоненты алгоритма

### 1. Парсинг VTT файла

Первым шагом алгоритма является парсинг VTT файла для извлечения текста субтитров и их таймингов. Каждый субтитр представляет собой сегмент с:
- Временем начала
- Временем окончания
- Текстом

### 2. Генерация TTS для каждого сегмента

Для каждого сегмента субтитров генерируется аудио с помощью OpenAI TTS API. Важно сохранить информацию о:
- Длительности сгенерированного аудио
- Соответствии между текстом и аудио

### 3. Анализ и корректировка длительности

Для каждого сегмента выполняется анализ и корректировка длительности:
1. Сравнение длительности сгенерированного аудио с длительностью соответствующего сегмента субтитров
2. Расчет коэффициента растяжения/сжатия для каждого сегмента
3. Применение алгоритма изменения темпа без изменения высоты тона (с использованием библиотеки rubato)

### 4. Обработка пауз и переходов

Для естественного звучания необходимо обработать паузы и переходы между сегментами:
1. Анализ пауз между оригинальными сегментами
2. Вставка пауз соответствующей длительности между сегментами озвучки
3. Сглаживание переходов для более естественного звучания

### 5. Сборка финальной аудиодорожки

Финальным шагом является сборка всех обработанных сегментов в единую аудиодорожку:
1. Последовательное объединение всех сегментов с учетом пауз
2. Нормализация громкости
3. Финальная обработка (компрессия, эквализация при необходимости)

## Детальное описание алгоритма

### Шаг 1: Парсинг VTT файла

```rust
struct SubtitleSegment {
    start_time: f64,  // время начала в секундах
    end_time: f64,    // время окончания в секундах
    text: String,     // текст субтитра
}

fn parse_vtt(vtt_path: &str) -> Result<Vec<SubtitleSegment>, Error> {
    // Чтение VTT файла
    // Парсинг временных меток и текста
    // Возврат вектора сегментов
}
```

### Шаг 2: Генерация TTS для каждого сегмента

```rust
struct TtsSegment {
    subtitle: SubtitleSegment,
    audio_data: Vec<f32>,       // аудио данные
    duration: f64,              // длительность аудио в секундах
    target_duration: f64,       // целевая длительность (из субтитров)
    stretch_factor: f64,        // коэффициент растяжения/сжатия
}

async fn generate_tts_segments(
    segments: &[SubtitleSegment], 
    api_key: &str,
    voice: &str,
) -> Result<Vec<TtsSegment>, Error> {
    let mut tts_segments = Vec::new();
    
    for segment in segments {
        // Генерация TTS с помощью OpenAI API
        let audio_data = generate_tts(segment.text.clone(), api_key, voice).await?;
        
        // Определение длительности сгенерированного аудио
        let duration = audio_data.len() as f64 / SAMPLE_RATE;
        
        // Расчет целевой длительности из субтитров
        let target_duration = segment.end_time - segment.start_time;
        
        // Расчет коэффициента растяжения/сжатия
        let stretch_factor = target_duration / duration;
        
        tts_segments.push(TtsSegment {
            subtitle: segment.clone(),
            audio_data,
            duration,
            target_duration,
            stretch_factor,
        });
    }
    
    Ok(tts_segments)
}
```

### Шаг 3: Анализ и корректировка длительности

```rust
fn adjust_segment_duration(segment: &TtsSegment) -> Result<Vec<f32>, Error> {
    // Создание ресемплера с использованием библиотеки rubato
    let params = rubato::SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: rubato::SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: rubato::WindowFunction::BlackmanHarris2,
    };
    
    let stretch_ratio = segment.target_duration / segment.duration;
    
    // Создание ресемплера для изменения темпа без изменения высоты тона
    let mut resampler = rubato::SincFixedOut::new(
        stretch_ratio,
        1.0,
        params,
        segment.audio_data.len(),
        1, // mono
    )?;
    
    // Применение ресемплинга
    let mut output = vec![0.0; (segment.audio_data.len() as f64 * stretch_ratio) as usize];
    resampler.process(&[&segment.audio_data], &mut [&mut output])?;
    
    Ok(output)
}
```

### Шаг 4: Обработка пауз и переходов

```rust
fn process_pauses_and_transitions(
    segments: &[TtsSegment],
    adjusted_audio: &[Vec<f32>],
) -> Result<Vec<f32>, Error> {
    let mut final_audio = Vec::new();
    let mut current_time = 0.0;
    
    for (i, segment) in segments.iter().enumerate() {
        // Расчет паузы перед текущим сегментом
        let pause_duration = if i == 0 {
            segment.subtitle.start_time
        } else {
            segment.subtitle.start_time - segments[i-1].subtitle.end_time
        };
        
        // Добавление паузы (тишины)
        if pause_duration > 0.0 {
            let pause_samples = (pause_duration * SAMPLE_RATE as f64) as usize;
            final_audio.extend(vec![0.0; pause_samples]);
        }
        
        // Добавление аудио сегмента
        final_audio.extend_from_slice(&adjusted_audio[i]);
        
        // Обновление текущего времени
        current_time = segment.subtitle.end_time;
    }
    
    Ok(final_audio)
}
```

### Шаг 5: Сборка финальной аудиодорожки

```rust
fn build_final_audio(
    processed_audio: Vec<f32>,
    video_duration: f64,
) -> Result<Vec<f32>, Error> {
    // Убедиться, что длительность аудио соответствует длительности видео
    let expected_samples = (video_duration * SAMPLE_RATE as f64) as usize;
    let mut final_audio = processed_audio;
    
    if final_audio.len() < expected_samples {
        // Добавить тишину в конец, если аудио короче видео
        final_audio.extend(vec![0.0; expected_samples - final_audio.len()]);
    } else if final_audio.len() > expected_samples {
        // Обрезать аудио, если оно длиннее видео
        final_audio.truncate(expected_samples);
    }
    
    // Нормализация громкости
    normalize_volume(&mut final_audio);
    
    Ok(final_audio)
}

fn normalize_volume(audio: &mut [f32]) {
    // Найти максимальную амплитуду
    let max_amplitude = audio.iter().fold(0.0, |max, &sample| {
        max.max(sample.abs())
    });
    
    // Нормализовать, если максимальная амплитуда не равна 0
    if max_amplitude > 0.0 {
        let gain = 0.9 / max_amplitude; // Оставляем небольшой запас
        for sample in audio.iter_mut() {
            *sample *= gain;
        }
    }
}
```

## Оптимизации и улучшения

### 1. Интеллектуальное разделение длинных сегментов

Для длинных сегментов субтитров можно применить алгоритм разделения на более мелкие части на основе естественных пауз в речи:

```rust
fn split_long_segment(segment: &TtsSegment) -> Vec<TtsSegment> {
    // Если сегмент короче порогового значения, возвращаем его без изменений
    if segment.duration < MAX_SEGMENT_DURATION {
        return vec![segment.clone()];
    }
    
    // Анализ текста для определения естественных точек разделения
    // (знаки препинания, конец предложений и т.д.)
    let split_points = analyze_text_for_split_points(&segment.subtitle.text);
    
    // Разделение сегмента на более мелкие части
    // ...
}
```

### 2. Адаптивное изменение темпа

Для более естественного звучания можно применить адаптивное изменение темпа, учитывающее характеристики речи:

```rust
fn adaptive_tempo_adjustment(segment: &TtsSegment) -> Result<Vec<f32>, Error> {
    // Анализ характеристик речи
    let speech_characteristics = analyze_speech(&segment.audio_data);
    
    // Определение оптимального алгоритма изменения темпа
    let algorithm = select_tempo_algorithm(
        speech_characteristics,
        segment.stretch_factor
    );
    
    // Применение выбранного алгоритма
    // ...
}
```

### 3. Сохранение просодии

Для сохранения естественной интонации и акцентов можно применить алгоритмы сохранения просодии:

```rust
fn preserve_prosody(
    original_audio: &[f32],
    stretched_audio: &mut [f32],
) -> Result<(), Error> {
    // Анализ просодии оригинального аудио
    let prosody = analyze_prosody(original_audio);
    
    // Применение просодии к растянутому аудио
    apply_prosody(stretched_audio, prosody)?;
    
    Ok(())
}
```

## Интеграция с основным приложением

Алгоритм TTS-Sync будет интегрирован в основное приложение через асинхронный API, который будет предоставлять информацию о прогрессе:

```rust
pub async fn synchronize_tts(
    vtt_path: &str,
    video_duration: f64,
    api_key: &str,
    voice: &str,
    progress_callback: impl Fn(f32, &str) + Send + 'static,
) -> Result<String, Error> {
    // Шаг 1: Парсинг VTT файла
    progress_callback(0.0, "Парсинг субтитров");
    let segments = parse_vtt(vtt_path)?;
    
    // Шаг 2: Генерация TTS
    progress_callback(10.0, "Генерация TTS");
    let tts_segments = generate_tts_segments(&segments, api_key, voice).await?;
    
    // Шаг 3: Корректировка длительности
    progress_callback(50.0, "Корректировка длительности");
    let mut adjusted_segments = Vec::new();
    for (i, segment) in tts_segments.iter().enumerate() {
        adjusted_segments.push(adjust_segment_duration(segment)?);
        progress_callback(50.0 + (i as f32 / tts_segments.len() as f32) * 20.0, 
                         "Корректировка длительности");
    }
    
    // Шаг 4: Обработка пауз и переходов
    progress_callback(70.0, "Обработка пауз и переходов");
    let processed_audio = process_pauses_and_transitions(&tts_segments, &adjusted_segments)?;
    
    // Шаг 5: Сборка финальной аудиодорожки
    progress_callback(90.0, "Сборка финальной аудиодорожки");
    let final_audio = build_final_audio(processed_audio, video_duration)?;
    
    // Сохранение аудио в файл
    progress_callback(95.0, "Сохранение аудио");
    let output_path = save_audio_to_file(&final_audio)?;
    
    progress_callback(100.0, "Завершено");
    Ok(output_path)
}
```

## Заключение

Предложенный алгоритм TTS-Sync обеспечивает максимально точную и качественную синхронизацию TTS с видео и субтитрами. Он учитывает особенности речи, сохраняет естественное звучание и обеспечивает плавные переходы между сегментами. Алгоритм использует современные библиотеки для обработки аудио в Rust, такие как rubato для изменения темпа без изменения высоты тона, и предоставляет асинхронный API с информацией о прогрессе.
