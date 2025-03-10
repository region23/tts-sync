# Руководство по использованию библиотеки TTS-Sync

## Содержание

1. [Введение](#введение)
2. [Установка](#установка)
3. [Основные концепции](#основные-концепции)
4. [Пошаговое руководство](#пошаговое-руководство)
   - [Парсинг VTT файлов](#парсинг-vtt-файлов)
   - [Интеграция с OpenAI TTS](#интеграция-с-openai-tts)
   - [Анализ аудио и корректировка таймингов](#анализ-аудио-и-корректировка-таймингов)
   - [Обработка аудио](#обработка-аудио)
   - [Отслеживание прогресса](#отслеживание-прогресса)
   - [Полный процесс синхронизации](#полный-процесс-синхронизации)
5. [Настройка и оптимизация](#настройка-и-оптимизация)
6. [Обработка ошибок и логирование](#обработка-ошибок-и-логирование)
7. [Примеры использования](#примеры-использования)
8. [Интеграция с Tauri и Vue 3](#интеграция-с-tauri-и-vue-3)
9. [Часто задаваемые вопросы](#часто-задаваемые-вопросы)

## Введение

TTS-Sync - это библиотека на Rust для синхронизации TTS (Text-to-Speech) с видео и субтитрами. Она позволяет создавать качественную озвучку на основе переведенных субтитров, где аудио максимально точно соответствует таймингу оригинального видео.

Основная цель библиотеки - решить проблему несоответствия длительности сгенерированной речи и оригинальных субтитров, что часто приводит к рассинхронизации аудио и видео. TTS-Sync анализирует аудио, определяет паузы и характеристики речи, а затем корректирует темп для достижения идеальной синхронизации.

## Установка

### Требования

- Rust 1.56 или выше
- OpenAI API ключ для генерации TTS
- Для работы с аудио файлами может потребоваться установка дополнительных системных библиотек

### Добавление в проект

Добавьте библиотеку в ваш проект Rust, добавив следующую строку в `Cargo.toml`:

```toml
[dependencies]
tts-sync = "0.1.0"
```

Или используйте команду cargo:

```bash
cargo add tts-sync
```

### Настройка окружения

Рекомендуется хранить API ключ OpenAI в переменных окружения:

```bash
export OPENAI_API_KEY="ваш-api-ключ"
```

## Основные концепции

TTS-Sync основан на нескольких ключевых концепциях:

1. **Субтитры (Subtitles)** - текстовые сегменты с временными метками начала и окончания.
2. **TTS Сегменты (TTS Segments)** - аудио сегменты, сгенерированные из текста субтитров.
3. **Аудио Анализ (Audio Analysis)** - процесс анализа аудио для определения пауз и характеристик речи.
4. **Корректировка Темпа (Tempo Adjustment)** - изменение скорости воспроизведения аудио без изменения высоты тона.
5. **Обработка Аудио (Audio Processing)** - применение эффектов обработки аудио для улучшения качества звучания.
6. **Синхронизация (Synchronization)** - процесс подгонки аудио сегментов под временные метки субтитров.
7. **Отслеживание Прогресса (Progress Tracking)** - механизм для информирования о ходе выполнения операций.

## Пошаговое руководство

### Парсинг VTT файлов

WebVTT (Web Video Text Tracks) - это формат субтитров, используемый в веб-видео. TTS-Sync предоставляет инструменты для парсинга VTT файлов:

```rust
use tts_sync::{VttParser, Result};

fn parse_vtt_example() -> Result<()> {
    // Парсинг VTT файла
    let subtitles = VttParser::parse_file("path/to/subtitles.vtt")?;
    
    println!("Количество субтитров: {}", subtitles.len());
    
    // Перебор субтитров
    for subtitle in subtitles.iter() {
        println!("Время: {:.2} - {:.2}, Текст: {}", 
                 subtitle.start_time, 
                 subtitle.end_time, 
                 subtitle.text);
    }
    
    Ok(())
}
```

### Интеграция с OpenAI TTS

TTS-Sync использует OpenAI API для генерации речи из текста:

```rust
use tts_sync::{OpenAiTts, TtsOptions, OpenAiVoice, OpenAiTtsModel, OpenAiAudioFormat, Result};
use std::env;

async fn generate_tts_example() -> Result<()> {
    // Получаем API ключ из переменных окружения
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY не установлен");
    
    // Настройки для TTS
    let tts_options = TtsOptions {
        model: OpenAiTtsModel::Tts1Hd,
        voice: OpenAiVoice::Alloy,
        speed: 1.0,
        response_format: OpenAiAudioFormat::Mp3,
    };
    
    // Создаем клиент OpenAI TTS
    let tts_client = OpenAiTts::new(api_key, tts_options);
    
    // Генерируем речь и сохраняем в файл
    tts_client.generate_speech_to_file(
        "Привет, это пример синтезированной речи.",
        "output.mp3"
    ).await?;
    
    println!("Аудио сохранено в output.mp3");
    
    Ok(())
}
```

### Анализ аудио и корректировка таймингов

Это ключевая функциональность TTS-Sync, которая позволяет анализировать аудио и корректировать его темп для синхронизации с субтитрами:

```rust
use tts_sync::{AudioData, AudioAnalyzer, TempoAdjuster, TempoAlgorithm, Result};

fn analyze_and_adjust_audio_example() -> Result<()> {
    // Предположим, у нас есть аудио данные
    let audio_data = AudioData::new(
        vec![/* аудио сэмплы */], 
        44100, // частота дискретизации
        1      // количество каналов (моно)
    );
    
    // Анализируем аудио
    let analysis = AudioAnalyzer::analyze(&audio_data)?;
    
    println!("Длительность аудио: {:.2} секунд", analysis.duration);
    println!("RMS (громкость): {:.4}", analysis.rms);
    println!("Пиковая амплитуда: {:.4}", analysis.peak);
    println!("Количество пауз: {}", analysis.silences.len());
    
    // Изменяем темп аудио с использованием разных алгоритмов
    
    // 1. Линейный алгоритм (быстрый, но низкое качество)
    let tempo_factor = 1.2; // ускоряем на 20%
    let adjusted_linear = TempoAdjuster::adjust_tempo(
        &audio_data, 
        tempo_factor, 
        TempoAlgorithm::Linear
    )?;
    
    // 2. FIR алгоритм (средний по скорости и качеству)
    let adjusted_fir = TempoAdjuster::adjust_tempo(
        &audio_data, 
        tempo_factor, 
        TempoAlgorithm::Fir
    )?;
    
    // 3. Sinc алгоритм (медленный, но высокое качество)
    let adjusted_sinc = TempoAdjuster::adjust_tempo(
        &audio_data, 
        tempo_factor, 
        TempoAlgorithm::Sinc
    )?;
    
    // 4. Адаптивное изменение темпа с сохранением пауз
    let target_duration = audio_data.duration() / tempo_factor;
    let adjusted_adaptive = TempoAdjuster::adaptive_tempo_adjustment(
        &audio_data,
        target_duration,
        TempoAlgorithm::Sinc,
        true // сохраняем паузы
    )?;
    
    println!("Новая длительность (линейный): {:.2} секунд", adjusted_linear.duration());
    println!("Новая длительность (FIR): {:.2} секунд", adjusted_fir.duration());
    println!("Новая длительность (Sinc): {:.2} секунд", adjusted_sinc.duration());
    println!("Новая длительность (адаптивный): {:.2} секунд", adjusted_adaptive.duration());
    
    Ok(())
}
```

#### Подробнее об анализе аудио

Анализатор аудио (`AudioAnalyzer`) выполняет следующие операции:

1. **Расчет RMS (Root Mean Square)** - мера громкости аудио
2. **Определение пиковой амплитуды** - максимальное значение амплитуды
3. **Обнаружение пауз** - поиск участков с низкой амплитудой
4. **Оценка темпа речи** - приблизительное количество слогов в секунду

Эта информация используется для принятия решений о том, как лучше корректировать темп аудио.

#### Подробнее о корректировке темпа

Корректировщик темпа (`TempoAdjuster`) использует библиотеку `rubato` для изменения темпа аудио без изменения высоты тона. Это позволяет ускорять или замедлять речь, сохраняя естественное звучание голоса.

Основные методы:

- `adjust_tempo` - изменяет темп всего аудио с выбором алгоритма
- `fit_to_duration` - подгоняет длительность аудио под целевую длительность
- `adaptive_tempo_adjustment` - адаптивно изменяет темп с сохранением пауз

### Обработка аудио

TTS-Sync предоставляет функции для обработки аудио, которые позволяют улучшить качество звучания:

```rust
use tts_sync::{AudioData, AudioProcessor, Result};

fn audio_processing_example() -> Result<()> {
    // Предположим, у нас есть аудио данные
    let audio_data = AudioData::new(
        vec![/* аудио сэмплы */], 
        44100, // частота дискретизации
        1      // количество каналов (моно)
    );
    
    // 1. Применяем компрессию динамического диапазона
    let compressed = AudioProcessor::apply_compression(
        &audio_data,
        -20.0, // порог в дБ
        4.0,   // соотношение компрессии (4:1)
        10.0,  // время атаки в мс
        100.0, // время восстановления в мс
        6.0    // компенсационное усиление в дБ
    )?;
    
    // 2. Применяем эквализацию (трехполосный эквалайзер)
    let equalized = AudioProcessor::apply_equalization(
        &compressed,
        3.0,    // усиление низких частот в дБ
        0.0,    // усиление средних частот в дБ
        2.0,    // усиление высоких частот в дБ
        300.0,  // частота разделения низких и средних частот в Гц
        3000.0  // частота разделения средних и высоких частот в Гц
    )?;
    
    // 3. Нормализуем громкость
    let normalized = AudioProcessor::normalize_volume(
        &equalized,
        -3.0    // целевой уровень в дБ
    )?;
    
    println!("Аудио успешно обработано");
    
    Ok(())
}
```

#### Компрессия динамического диапазона

Компрессия уменьшает разницу между тихими и громкими частями аудио, делая речь более разборчивой и равномерной по громкости. Параметры компрессии:

- **Порог (threshold)** - уровень в дБ, выше которого начинается компрессия
- **Соотношение (ratio)** - степень компрессии (например, 4:1 означает, что превышение порога на 4 дБ будет уменьшено до 1 дБ)
- **Время атаки (attack)** - время в мс, за которое компрессор начинает действовать
- **Время восстановления (release)** - время в мс, за которое компрессор перестает действовать
- **Компенсационное усиление (makeup gain)** - усиление в дБ, применяемое после компрессии

#### Эквализация

Эквализация позволяет регулировать уровень разных частотных диапазонов, улучшая звучание речи. TTS-Sync предоставляет трехполосный эквалайзер с настройками:

- **Усиление низких частот** - регулировка уровня низких частот (до частоты разделения низких и средних)
- **Усиление средних частот** - регулировка уровня средних частот (между частотами разделения)
- **Усиление высоких частот** - регулировка уровня высоких частот (выше частоты разделения средних и высоких)
- **Частота разделения низких и средних частот** - граница между низкими и средними частотами
- **Частота разделения средних и высоких частот** - граница между средними и высокими частотами

#### Нормализация громкости

Нормализация громкости обеспечивает одинаковый уровень громкости для всего аудио, что важно для согласованного звучания. Параметр нормализации:

- **Целевой уровень в дБ** - желаемый пиковый уровень громкости (обычно от -6 до -3 дБ)

### Отслеживание прогресса

TTS-Sync предоставляет систему отслеживания прогресса, которая позволяет информировать пользователя о ходе выполнения операций:

```rust
use tts_sync::{ProgressTracker, Result};
use std::{thread, time::Duration};

fn progress_tracking_example() -> Result<()> {
    // Создаем трекер прогресса с функцией обратного вызова
    let tracker = ProgressTracker::with_callback(Box::new(|progress, status| {
        println!("Прогресс: {:.1}%, Статус: {}", progress, status);
    }));
    
    // Симулируем длительную операцию
    for i in 0..10 {
        let progress = i as f32 * 10.0;
        tracker.update(progress, &format!("Шаг {} из 10", i + 1))?;
        thread::sleep(Duration::from_millis(500));
    }
    
    // Завершаем операцию
    tracker.update(100.0, "Операция завершена")?;
    
    Ok(())
}
```

#### Вложенные трекеры прогресса

Для сложных операций, состоящих из нескольких этапов, можно использовать вложенные трекеры прогресса:

```rust
use tts_sync::{ProgressTracker, Result};

fn nested_progress_tracking_example() -> Result<()> {
    // Создаем основной трекер прогресса
    let main_tracker = ProgressTracker::with_callback(Box::new(|progress, status| {
        println!("Общий прогресс: {:.1}%, Статус: {}", progress, status);
    }));
    
    // Создаем дочерний трекер для первого этапа (0-50%)
    let stage1_tracker = main_tracker.create_child(0.0, 50.0);
    
    // Обновляем прогресс первого этапа
    for i in 0..5 {
        let progress = i as f32 * 20.0;
        stage1_tracker.update(progress, &format!("Этап 1: шаг {} из 5", i + 1))?;
        // Симуляция работы...
    }
    
    // Создаем дочерний трекер для второго этапа (50-100%)
    let stage2_tracker = main_tracker.create_child(50.0, 100.0);
    
    // Обновляем прогресс второго этапа
    for i in 0..5 {
        let progress = i as f32 * 20.0;
        stage2_tracker.update(progress, &format!("Этап 2: шаг {} из 5", i + 1))?;
        // Симуляция работы...
    }
    
    Ok(())
}
```

### Полный процесс синхронизации

Для выполнения полного процесса синхронизации TTS с видео и субтитрами используется основной класс `TtsSync`:

```rust
use tts_sync::{TtsSync, SyncOptions, AudioFormat, TempoAlgorithm, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Получаем API ключ из переменных окружения
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY не установлен");
    
    // Создаем экземпляр TtsSync с настройками по умолчанию и fluent-интерфейсом
    let tts_sync = TtsSync::default()
        .with_tempo_algorithm(TempoAlgorithm::Sinc)
        .with_compression(true)
        .with_equalization(true)
        .with_volume_normalization(true)
        .with_preserve_pauses(true)
        .with_progress_callback(Box::new(|progress, status| {
            println!("Прогресс: {:.1}%, Статус: {}", progress, status);
        }));
    
    // Синхронизируем TTS с видео и субтитрами
    let output_path = tts_sync.synchronize(
        "subtitles.vtt",
        120.0, // длительность видео в секундах
        &api_key
    ).await?;
    
    println!("Аудио сохранено в: {}", output_path);
    
    Ok(())
}
```

## Настройка и оптимизация

### Настройка параметров синхронизации

TTS-Sync предоставляет множество параметров для настройки процесса синхронизации:

```rust
use tts_sync::{SyncOptions, AudioFormat, TempoAlgorithm};
use log::LevelFilter;

// Создаем настройки с оптимальными параметрами для качества
let quality_options = SyncOptions {
    voice: "nova".to_string(),
    output_format: AudioFormat::Mp3,
    sample_rate: 48000,
    max_segment_duration: 5.0,
    normalize_volume: true,
    apply_compression: true,
    apply_equalization: true,
    tempo_algorithm: TempoAlgorithm::Sinc,
    preserve_pauses: true,
    
    // Параметры компрессии
    compression_threshold: -20.0,
    compression_ratio: 4.0,
    compression_attack: 10.0,
    compression_release: 100.0,
    compression_makeup_gain: 6.0,
    
    // Параметры эквализации
    eq_low_gain: 3.0,
    eq_mid_gain: 0.0,
    eq_high_gain: 2.0,
    eq_low_freq: 300.0,
    eq_high_freq: 3000.0,
    
    // Целевой уровень нормализации громкости
    normalization_target_db: -3.0,
    
    log_level: LevelFilter::Info,
};

// Создаем настройки с оптимальными параметрами для скорости
let speed_options = SyncOptions {
    voice: "alloy".to_string(),
    output_format: AudioFormat::Mp3,
    sample_rate: 44100,
    max_segment_duration: 10.0,
    normalize_volume: true,
    apply_compression: false,
    apply_equalization: false,
    tempo_algorithm: TempoAlgorithm::Linear,
    preserve_pauses: false,
    
    // Параметры компрессии (не используются)
    compression_threshold: -20.0,
    compression_ratio: 4.0,
    compression_attack: 10.0,
    compression_release: 100.0,
    compression_makeup_gain: 6.0,
    
    // Параметры эквализации (не используются)
    eq_low_gain: 0.0,
    eq_mid_gain: 0.0,
    eq_high_gain: 0.0,
    eq_low_freq: 300.0,
    eq_high_freq: 3000.0,
    
    // Целевой уровень нормализации громкости
    normalization_target_db: -3.0,
    
    log_level: LevelFilter::Warn,
};
```

### Оптимизация производительности

Для оптимизации производительности рекомендуется:

1. Выбирать подходящий алгоритм изменения темпа:
   - `TempoAlgorithm::Sinc` - высокое качество, но медленнее
   - `TempoAlgorithm::Fir` - среднее качество, быстрее
   - `TempoAlgorithm::Linear` - низкое качество, очень быстрый

2. Устанавливать разумное значение `max_segment_duration` - это влияет на разбиение длинных субтитров на части.

3. Отключать ненужные обработки (компрессия, эквализация), если они не требуются.

4. Использовать параметр `preserve_pauses`:
   - `true` - сохраняет естественные паузы в речи, но может быть медленнее
   - `false` - изменяет темп всего аудио, включая паузы, работает быстрее

## Обработка ошибок и логирование

### Обработка ошибок

TTS-Sync использует тип `Result<T, Error>` для обработки ошибок:

```rust
use tts_sync::{TtsSync, Error, Result};

async fn handle_errors_example() -> Result<()> {
    let tts_sync = TtsSync::default();
    
    match tts_sync.synchronize("subtitles.vtt", 120.0, "invalid-api-key").await {
        Ok(output_path) => {
            println!("Аудио сохранено в: {}", output_path);
        },
        Err(e) => {
            match e {
                Error::OpenAi(msg) => {
                    println!("Ошибка OpenAI API: {}", msg);
                },
                Error::VttParsing(msg) => {
                    println!("Ошибка парсинга VTT: {}", msg);
                },
                Error::Io(io_err) => {
                    println!("Ошибка ввода/вывода: {}", io_err);
                },
                Error::AudioProcessing(msg) => {
                    println!("Ошибка обработки аудио: {}", msg);
                },
                Error::Synchronization(msg) => {
                    println!("Ошибка синхронизации: {}", msg);
                },
                _ => {
                    println!("Другая ошибка: {}", e);
                }
            }
        }
    }
    
    Ok(())
}
```

### Логирование

TTS-Sync предоставляет функции для настройки логирования и логирования сообщений разных уровней:

```rust
use tts_sync::{setup_logging, log_info, log_error, log_warning, log_debug, log_trace};
use log::LevelFilter;

fn logging_example() {
    // Настраиваем логирование с уровнем Debug
    setup_logging(LevelFilter::Debug);
    
    // Логируем сообщения разных уровней
    log_info("Информационное сообщение");
    log_warning("Предупреждение");
    log_error::<(), _>(&std::io::Error::new(std::io::ErrorKind::Other, "Тестовая ошибка"), "Произошла ошибка")?;
    log_debug("Отладочное сообщение");
    log_trace("Трассировочное сообщение");
}
```

## Примеры использования

### Базовый пример

```rust
use tts_sync::{TtsSync, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Получаем API ключ из переменных окружения
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY не установлен");
    
    // Создаем экземпляр TtsSync с настройками по умолчанию
    let tts_sync = TtsSync::default();
    
    // Синхронизируем TTS с видео и субтитрами
    let output_path = tts_sync.synchronize(
        "subtitles.vtt",
        120.0, // длительность видео в секундах
        &api_key
    ).await?;
    
    println!("Аудио сохранено в: {}", output_path);
    
    Ok(())
}
```

### Пример с отслеживанием прогресса

```rust
use tts_sync::{TtsSync, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Получаем API ключ из переменных окружения
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY не установлен");
    
    // Создаем экземпляр TtsSync с функцией обратного вызова для отслеживания прогресса
    let tts_sync = TtsSync::default()
        .with_progress_callback(Box::new(|progress, status| {
            println!("Прогресс: {:.1}%, Статус: {}", progress, status);
        }));
    
    // Синхронизируем TTS с видео и субтитрами
    let output_path = tts_sync.synchronize(
        "subtitles.vtt",
        120.0, // длительность видео в секундах
        &api_key
    ).await?;
    
    println!("Аудио сохранено в: {}", output_path);
    
    Ok(())
}
```

### Пример с пользовательскими настройками

```rust
use tts_sync::{TtsSync, SyncOptions, AudioFormat, TempoAlgorithm, Result};
use log::LevelFilter;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Получаем API ключ из переменных окружения
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY не установлен");
    
    // Настройки синхронизации
    let options = SyncOptions {
        voice: "nova".to_string(),
        output_format: AudioFormat::Wav,
        sample_rate: 48000,
        max_segment_duration: 5.0,
        normalize_volume: true,
        apply_compression: true,
        apply_equalization: true,
        tempo_algorithm: TempoAlgorithm::Sinc,
        preserve_pauses: true,
        
        // Параметры компрессии
        compression_threshold: -20.0,
        compression_ratio: 4.0,
        compression_attack: 10.0,
        compression_release: 100.0,
        compression_makeup_gain: 6.0,
        
        // Параметры эквализации
        eq_low_gain: 3.0,
        eq_mid_gain: 0.0,
        eq_high_gain: 2.0,
        eq_low_freq: 300.0,
        eq_high_freq: 3000.0,
        
        // Целевой уровень нормализации громкости
        normalization_target_db: -3.0,
        
        log_level: LevelFilter::Debug,
    };
    
    // Создаем экземпляр TtsSync с настройками
    let tts_sync = TtsSync::new(options)
        .with_progress_callback(Box::new(|progress, status| {
            println!("Прогресс: {:.1}%, Статус: {}", progress, status);
        }));
    
    // Синхронизируем TTS с видео и субтитрами
    let output_path = tts_sync.synchronize(
        "subtitles.vtt",
        120.0, // длительность видео в секундах
        &api_key
    ).await?;
    
    println!("Аудио сохранено в: {}", output_path);
    
    Ok(())
}
```

### Пример с использованием fluent-интерфейса

```rust
use tts_sync::{TtsSync, TempoAlgorithm, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Получаем API ключ из переменных окружения
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY не установлен");
    
    // Создаем экземпляр TtsSync с использованием fluent-интерфейса
    let tts_sync = TtsSync::default()
        .with_tempo_algorithm(TempoAlgorithm::Sinc)
        .with_compression(true)
        .with_equalization(true)
        .with_volume_normalization(true)
        .with_preserve_pauses(true)
        .with_progress_callback(Box::new(|progress, status| {
            println!("Прогресс: {:.1}%, Статус: {}", progress, status);
        }));
    
    // Синхронизируем TTS с видео и субтитрами
    let output_path = tts_sync.synchronize(
        "subtitles.vtt",
        120.0, // длительность видео в секундах
        &api_key
    ).await?;
    
    println!("Аудио сохранено в: {}", output_path);
    
    Ok(())
}
```

## Интеграция с Tauri и Vue 3

TTS-Sync можно легко интегрировать в приложения на Tauri и Vue 3. Ниже приведен пример такой интеграции.

### Настройка Tauri проекта

1. Создайте новый Tauri проект с Vue 3:

```bash
npm create tauri-app@latest
```

2. Добавьте tts-sync в зависимости в `src-tauri/Cargo.toml`:

```toml
[dependencies]
tts-sync = "0.1.0"
tokio = { version = "1.36", features = ["full"] }
```

### Создание Rust функций для Tauri

В файле `src-tauri/src/main.rs` добавьте функции для работы с TTS-Sync:

```rust
#[tauri::command]
async fn synchronize_tts(
    vtt_path: String,
    video_duration: f64,
    api_key: String,
    voice: String,
    apply_compression: bool,
    apply_equalization: bool,
    window: tauri::Window,
) -> Result<String, String> {
    // Создаем экземпляр TtsSync с настройками
    let tts_sync = tts_sync::TtsSync::default()
        .with_tempo_algorithm(tts_sync::TempoAlgorithm::Sinc)
        .with_compression(apply_compression)
        .with_equalization(apply_equalization)
        .with_volume_normalization(true)
        .with_preserve_pauses(true)
        .with_progress_callback(Box::new(move |progress, status| {
            // Отправляем прогресс в Vue приложение
            let _ = window.emit("tts-progress", (progress, status.to_string()));
            Ok(())
        }));
    
    // Синхронизируем TTS с видео и субтитрами
    match tts_sync.synchronize(&vtt_path, video_duration, &api_key).await {
        Ok(output_path) => Ok(output_path),
        Err(e) => Err(format!("Ошибка синхронизации: {}", e)),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![synchronize_tts])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Создание Vue компонента

В Vue приложении создайте компонент для работы с TTS-Sync:

```vue
<template>
  <div class="tts-sync">
    <h1>TTS Synchronization</h1>
    
    <div class="form">
      <div class="form-group">
        <label for="vtt-file">VTT файл:</label>
        <input type="file" id="vtt-file" @change="handleVttFile" accept=".vtt" />
      </div>
      
      <div class="form-group">
        <label for="video-duration">Длительность видео (секунды):</label>
        <input type="number" id="video-duration" v-model="videoDuration" />
      </div>
      
      <div class="form-group">
        <label for="api-key">OpenAI API ключ:</label>
        <input type="password" id="api-key" v-model="apiKey" />
      </div>
      
      <div class="form-group">
        <label for="voice">Голос:</label>
        <select id="voice" v-model="voice">
          <option value="alloy">Alloy</option>
          <option value="echo">Echo</option>
          <option value="fable">Fable</option>
          <option value="nova">Nova</option>
          <option value="onyx">Onyx</option>
          <option value="shimmer">Shimmer</option>
        </select>
      </div>
      
      <div class="form-group">
        <label>
          <input type="checkbox" v-model="applyCompression" />
          Применить компрессию
        </label>
      </div>
      
      <div class="form-group">
        <label>
          <input type="checkbox" v-model="applyEqualization" />
          Применить эквализацию
        </label>
      </div>
      
      <button @click="synchronize" :disabled="isProcessing">Синхронизировать</button>
    </div>
    
    <div v-if="isProcessing" class="progress">
      <div class="progress-bar" :style="{ width: `${progress}%` }"></div>
      <div class="progress-text">{{ progress.toFixed(1) }}% - {{ status }}</div>
    </div>
    
    <div v-if="outputPath" class="result">
      <h2>Результат:</h2>
      <p>Аудио сохранено в: {{ outputPath }}</p>
    </div>
    
    <div v-if="error" class="error">
      <h2>Ошибка:</h2>
      <p>{{ error }}</p>
    </div>
  </div>
</template>

<script>
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

export default {
  setup() {
    const vttPath = ref('');
    const videoDuration = ref(120);
    const apiKey = ref('');
    const voice = ref('alloy');
    const applyCompression = ref(false);
    const applyEqualization = ref(false);
    const isProcessing = ref(false);
    const progress = ref(0);
    const status = ref('');
    const outputPath = ref('');
    const error = ref('');
    
    onMounted(async () => {
      // Слушаем события прогресса от Rust
      await listen('tts-progress', (event) => {
        progress.value = event.payload[0];
        status.value = event.payload[1];
      });
    });
    
    const handleVttFile = (event) => {
      const file = event.target.files[0];
      if (file) {
        // Сохраняем файл во временную директорию
        // В реальном приложении здесь будет код для сохранения файла
        vttPath.value = `/path/to/temp/${file.name}`;
      }
    };
    
    const synchronize = async () => {
      if (!vttPath.value || !apiKey.value) {
        error.value = 'Пожалуйста, выберите VTT файл и введите API ключ';
        return;
      }
      
      isProcessing.value = true;
      progress.value = 0;
      status.value = 'Начало синхронизации';
      error.value = '';
      outputPath.value = '';
      
      try {
        const result = await invoke('synchronize_tts', {
          vttPath: vttPath.value,
          videoDuration: videoDuration.value,
          apiKey: apiKey.value,
          voice: voice.value,
          applyCompression: applyCompression.value,
          applyEqualization: applyEqualization.value,
        });
        
        outputPath.value = result;
      } catch (e) {
        error.value = e;
      } finally {
        isProcessing.value = false;
      }
    };
    
    return {
      vttPath,
      videoDuration,
      apiKey,
      voice,
      applyCompression,
      applyEqualization,
      isProcessing,
      progress,
      status,
      outputPath,
      error,
      handleVttFile,
      synchronize,
    };
  }
}
</script>

<style scoped>
.tts-sync {
  max-width: 600px;
  margin: 0 auto;
  padding: 20px;
}

.form-group {
  margin-bottom: 15px;
}

.progress {
  margin: 20px 0;
  background-color: #f0f0f0;
  border-radius: 4px;
  overflow: hidden;
}

.progress-bar {
  height: 20px;
  background-color: #4caf50;
  transition: width 0.3s ease;
}

.progress-text {
  text-align: center;
  margin-top: 5px;
}

.result, .error {
  margin-top: 20px;
  padding: 15px;
  border-radius: 4px;
}

.result {
  background-color: #e8f5e9;
}

.error {
  background-color: #ffebee;
}
</style>
```

## Часто задаваемые вопросы

### Какие языки поддерживает TTS-Sync?

TTS-Sync поддерживает любые языки, которые поддерживаются OpenAI TTS API. Библиотека не зависит от конкретного языка и может работать с любыми языковыми парами.

### Как выбрать оптимальный алгоритм изменения темпа?

Выбор алгоритма зависит от ваших приоритетов:
- `TempoAlgorithm::Sinc` - лучшее качество звука, но самый медленный
- `TempoAlgorithm::Fir` - хороший баланс между качеством и скоростью
- `TempoAlgorithm::Linear` - самый быстрый, но может снизить качество звука

### Как обрабатывать длинные субтитры?

Для длинных субтитров рекомендуется:
1. Установить разумное значение `max_segment_duration` (например, 5-10 секунд)
2. Использовать адаптивное изменение темпа с сохранением пауз (`preserve_pauses = true`)
3. Применять компрессию для улучшения разборчивости речи

### Какой формат аудио лучше использовать?

- `AudioFormat::Mp3` - хорошее сжатие, подходит для большинства случаев
- `AudioFormat::Wav` - без потерь качества, но большие файлы
- `AudioFormat::Ogg` - хорошее сжатие с высоким качеством

### Как интегрировать TTS-Sync с ffmpeg для финальной сборки видео?

Пример интеграции с ffmpeg для объединения видео и синхронизированного аудио:

```rust
use std::process::Command;

fn merge_audio_with_video(video_path: &str, audio_path: &str, output_path: &str) -> Result<(), std::io::Error> {
    let status = Command::new("ffmpeg")
        .arg("-i")
        .arg(video_path)
        .arg("-i")
        .arg(audio_path)
        .arg("-c:v")
        .arg("copy")
        .arg("-c:a")
        .arg("aac")
        .arg("-map")
        .arg("0:v:0")
        .arg("-map")
        .arg("1:a:0")
        .arg("-shortest")
        .arg(output_path)
        .status()?;
    
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "ffmpeg command failed"
        ))
    }
}
```

### Как оптимизировать использование памяти при работе с длинными видео?

Для оптимизации использования памяти:
1. Обрабатывайте субтитры небольшими группами
2. Используйте метод `synchronize_to_memory` вместо `synchronize` для контроля над процессом сохранения
3. Освобождайте память после обработки каждого сегмента
