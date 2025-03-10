# Архитектура и API библиотеки tts-sync

## Обзор архитектуры

Библиотека tts-sync представляет собой модульную систему для синхронизации TTS с видео и субтитрами. Архитектура библиотеки разработана с учетом следующих принципов:

1. **Модульность** - разделение функциональности на независимые компоненты
2. **Расширяемость** - возможность добавления новых функций и поддержки новых форматов
3. **Асинхронность** - поддержка асинхронных операций для длительных процессов
4. **Отслеживание прогресса** - предоставление информации о ходе выполнения операций
5. **Обработка ошибок** - надежная система обработки и сообщения об ошибках

## Структура библиотеки

```
tts-sync/
├── src/
│   ├── lib.rs              # Основной файл библиотеки, экспортирует публичный API
│   ├── vtt/                # Модуль для работы с VTT файлами
│   │   ├── mod.rs
│   │   ├── parser.rs       # Парсер VTT файлов
│   │   └── models.rs       # Модели данных для субтитров
│   ├── tts/                # Модуль для работы с TTS
│   │   ├── mod.rs
│   │   ├── openai.rs       # Интеграция с OpenAI TTS API
│   │   └── models.rs       # Модели данных для TTS
│   ├── audio/              # Модуль для обработки аудио
│   │   ├── mod.rs
│   │   ├── analysis.rs     # Анализ аудио
│   │   ├── adjustment.rs   # Корректировка длительности и темпа
│   │   ├── processing.rs   # Обработка аудио (нормализация, эффекты)
│   │   └── io.rs           # Ввод/вывод аудио файлов
│   ├── sync/               # Модуль синхронизации
│   │   ├── mod.rs
│   │   ├── algorithm.rs    # Основной алгоритм синхронизации
│   │   ├── segment.rs      # Работа с сегментами аудио и субтитров
│   │   └── timing.rs       # Расчет и корректировка таймингов
│   ├── progress/           # Модуль отслеживания прогресса
│   │   ├── mod.rs
│   │   └── tracker.rs      # Трекер прогресса
│   └── error.rs            # Определения ошибок
├── examples/               # Примеры использования библиотеки
├── tests/                  # Интеграционные тесты
└── benches/                # Бенчмарки
```

## Основные модули и их ответственность

### 1. Модуль VTT (`vtt`)

Отвечает за парсинг и работу с VTT файлами субтитров.

**Основные компоненты:**
- `VttParser` - парсер VTT файлов
- `Subtitle` - структура данных для представления субтитра
- `SubtitleTrack` - коллекция субтитров с методами для работы с ними

### 2. Модуль TTS (`tts`)

Отвечает за интеграцию с OpenAI TTS API и генерацию аудио.

**Основные компоненты:**
- `OpenAiTts` - клиент для работы с OpenAI TTS API
- `TtsOptions` - настройки для генерации TTS
- `TtsSegment` - структура данных для представления сегмента TTS

### 3. Модуль обработки аудио (`audio`)

Отвечает за анализ, обработку и корректировку аудио.

**Основные компоненты:**
- `AudioAnalyzer` - анализатор аудио
- `TempoAdjuster` - корректировщик темпа аудио
- `AudioProcessor` - обработчик аудио (нормализация, эффекты)
- `AudioIO` - ввод/вывод аудио файлов

### 4. Модуль синхронизации (`sync`)

Отвечает за синхронизацию аудио с субтитрами и видео.

**Основные компоненты:**
- `Synchronizer` - основной класс для синхронизации
- `SyncOptions` - настройки синхронизации
- `SegmentManager` - управление сегментами аудио и субтитров
- `TimingCalculator` - расчет и корректировка таймингов

### 5. Модуль отслеживания прогресса (`progress`)

Отвечает за отслеживание и сообщение о прогрессе выполнения операций.

**Основные компоненты:**
- `ProgressTracker` - трекер прогресса
- `ProgressCallback` - тип для функций обратного вызова прогресса

### 6. Модуль ошибок (`error`)

Определяет типы ошибок и функции для их обработки.

**Основные компоненты:**
- `Error` - перечисление всех возможных ошибок
- `Result<T>` - тип результата с обработкой ошибок

## Публичный API

### Основной интерфейс

```rust
/// Основной интерфейс для синхронизации TTS с видео и субтитрами
pub struct TtsSync {
    options: SyncOptions,
}

impl TtsSync {
    /// Создает новый экземпляр TtsSync с заданными настройками
    pub fn new(options: SyncOptions) -> Self {
        Self { options }
    }
    
    /// Создает новый экземпляр TtsSync с настройками по умолчанию
    pub fn default() -> Self {
        Self::new(SyncOptions::default())
    }
    
    /// Синхронизирует TTS с видео и субтитрами
    pub async fn synchronize(
        &self,
        vtt_path: &str,
        video_duration: f64,
        api_key: &str,
        progress_callback: impl Fn(f32, &str) + Send + 'static,
    ) -> Result<String> {
        // Реализация алгоритма синхронизации
        // ...
    }
    
    /// Синхронизирует TTS с видео и субтитрами, возвращая аудио данные
    pub async fn synchronize_to_memory(
        &self,
        vtt_path: &str,
        video_duration: f64,
        api_key: &str,
        progress_callback: impl Fn(f32, &str) + Send + 'static,
    ) -> Result<Vec<f32>> {
        // Реализация алгоритма синхронизации с возвратом аудио данных
        // ...
    }
}
```

### Настройки синхронизации

```rust
/// Настройки для синхронизации TTS с видео и субтитрами
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
        }
    }
}
```

### Перечисления и типы

```rust
/// Формат выходного аудио файла
pub enum AudioFormat {
    Mp3,
    Wav,
    Ogg,
}

/// Алгоритм изменения темпа
pub enum TempoAlgorithm {
    /// Алгоритм на основе sinc интерполяции (высокое качество, медленнее)
    Sinc,
    /// Алгоритм на основе FIR фильтра (среднее качество, быстрее)
    Fir,
    /// Алгоритм на основе линейной интерполяции (низкое качество, очень быстрый)
    Linear,
}

/// Тип для функций обратного вызова прогресса
pub type ProgressCallback = Box<dyn Fn(f32, &str) + Send + 'static>;

/// Результат с обработкой ошибок
pub type Result<T> = std::result::Result<T, Error>;
```

### Ошибки

```rust
/// Ошибки, которые могут возникнуть при синхронизации
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Ошибка ввода/вывода: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Ошибка парсинга VTT: {0}")]
    VttParsing(String),
    
    #[error("Ошибка OpenAI API: {0}")]
    OpenAi(String),
    
    #[error("Ошибка обработки аудио: {0}")]
    AudioProcessing(String),
    
    #[error("Ошибка синхронизации: {0}")]
    Synchronization(String),
    
    #[error("Неверные параметры: {0}")]
    InvalidParameters(String),
}
```

## Примеры использования

### Базовое использование

```rust
use tts_sync::{TtsSync, SyncOptions, Result};

async fn synchronize_subtitles() -> Result<()> {
    // Создание экземпляра TtsSync с настройками по умолчанию
    let tts_sync = TtsSync::default();
    
    // Синхронизация TTS с видео и субтитрами
    let output_path = tts_sync.synchronize(
        "subtitles.vtt",
        120.0, // длительность видео в секундах
        "your-openai-api-key",
        |progress, status| {
            println!("Прогресс: {}%, Статус: {}", progress, status);
        },
    ).await?;
    
    println!("Аудио сохранено в: {}", output_path);
    
    Ok(())
}
```

### Использование с пользовательскими настройками

```rust
use tts_sync::{TtsSync, SyncOptions, AudioFormat, TempoAlgorithm, Result};

async fn synchronize_with_custom_options() -> Result<()> {
    // Создание пользовательских настроек
    let options = SyncOptions {
        voice: "echo".to_string(),
        output_format: AudioFormat::Wav,
        sample_rate: 48000,
        max_segment_duration: 5.0,
        normalize_volume: true,
        apply_compression: true,
        apply_equalization: true,
        tempo_algorithm: TempoAlgorithm::Sinc,
    };
    
    // Создание экземпляра TtsSync с пользовательскими настройками
    let tts_sync = TtsSync::new(options);
    
    // Синхронизация TTS с видео и субтитрами
    let output_path = tts_sync.synchronize(
        "subtitles.vtt",
        120.0, // длительность видео в секундах
        "your-openai-api-key",
        |progress, status| {
            println!("Прогресс: {}%, Статус: {}", progress, status);
        },
    ).await?;
    
    println!("Аудио сохранено в: {}", output_path);
    
    Ok(())
}
```

### Получение аудио данных в памяти

```rust
use tts_sync::{TtsSync, Result};
use std::fs::File;
use std::io::Write;

async fn synchronize_to_memory() -> Result<()> {
    // Создание экземпляра TtsSync с настройками по умолчанию
    let tts_sync = TtsSync::default();
    
    // Синхронизация TTS с видео и субтитрами, получение аудио данных в памяти
    let audio_data = tts_sync.synchronize_to_memory(
        "subtitles.vtt",
        120.0, // длительность видео в секундах
        "your-openai-api-key",
        |progress, status| {
            println!("Прогресс: {}%, Статус: {}", progress, status);
        },
    ).await?;
    
    // Использование аудио данных (например, сохранение в файл)
    let mut file = File::create("output.raw")?;
    for sample in audio_data {
        file.write_all(&sample.to_le_bytes())?;
    }
    
    println!("Аудио данные сохранены в: output.raw");
    
    Ok(())
}
```

## Интеграция с Tauri и Vue 3

Для интеграции библиотеки tts-sync с приложением на Tauri и Vue 3, можно использовать следующий подход:

### Команда Tauri

```rust
#[tauri::command]
async fn synchronize_tts(
    vtt_path: String,
    video_duration: f64,
    api_key: String,
    window: tauri::Window,
) -> Result<String, String> {
    // Создание экземпляра TtsSync с настройками по умолчанию
    let tts_sync = TtsSync::default();
    
    // Функция обратного вызова для отслеживания прогресса
    let progress_callback = move |progress: f32, status: &str| {
        // Отправка события прогресса в окно Tauri
        let _ = window.emit("tts-sync-progress", ProgressPayload {
            progress,
            status: status.to_string(),
        });
    };
    
    // Синхронизация TTS с видео и субтитрами
    match tts_sync.synchronize(&vtt_path, video_duration, &api_key, progress_callback).await {
        Ok(output_path) => Ok(output_path),
        Err(err) => Err(err.to_string()),
    }
}

// Структура для передачи информации о прогрессе
#[derive(serde::Serialize)]
struct ProgressPayload {
    progress: f32,
    status: String,
}
```

### Компонент Vue 3

```vue
<template>
  <div>
    <h1>TTS Synchronization</h1>
    
    <div class="form">
      <input type="file" @change="onVttFileSelected" accept=".vtt" />
      <input type="text" v-model="apiKey" placeholder="OpenAI API Key" />
      <input type="number" v-model="videoDuration" placeholder="Video Duration (seconds)" />
      <button @click="synchronize" :disabled="isProcessing">Synchronize</button>
    </div>
    
    <div v-if="isProcessing" class="progress">
      <progress :value="progress" max="100"></progress>
      <p>{{ status }} ({{ progress.toFixed(1) }}%)</p>
    </div>
    
    <div v-if="outputPath" class="result">
      <p>Audio saved to: {{ outputPath }}</p>
      <audio controls :src="audioSrc"></audio>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/tauri';

const vttPath = ref('');
const apiKey = ref('');
const videoDuration = ref(0);
const isProcessing = ref(false);
const progress = ref(0);
const status = ref('');
const outputPath = ref('');

const audioSrc = computed(() => {
  if (outputPath.value) {
    return convertFileSrc(outputPath.value);
  }
  return '';
});

// Обработчик выбора VTT файла
const onVttFileSelected = (event) => {
  const file = event.target.files[0];
  if (file) {
    // В Tauri мы получаем путь к файлу через API
    // Здесь упрощенно, в реальном приложении нужно использовать Tauri API
    vttPath.value = file.path;
  }
};

// Функция для синхронизации TTS
const synchronize = async () => {
  if (!vttPath.value || !apiKey.value || videoDuration.value <= 0) {
    alert('Please fill all fields');
    return;
  }
  
  try {
    isProcessing.value = true;
    progress.value = 0;
    status.value = 'Starting...';
    
    // Подписка на события прогресса
    const unlistenProgress = await listen('tts-sync-progress', (event) => {
      progress.value = event.payload.progress;
      status.value = event.payload.status;
    });
    
    // Вызов команды Tauri
    outputPath.value = await invoke('synchronize_tts', {
      vttPath: vttPath.value,
      videoDuration: videoDuration.value,
      apiKey: apiKey.value,
    });
    
    // Отписка от событий прогресса
    unlistenProgress();
    
  } catch (error) {
    alert(`Error: ${error}`);
  } finally {
    isProcessing.value = false;
  }
};
</script>
```

## Заключение

Предложенная архитектура библиотеки tts-sync обеспечивает модульность, расширяемость и удобство использования. Библиотека предоставляет простой и понятный API для синхронизации TTS с видео и субтитрами, а также возможность отслеживания прогресса выполнения операций. Интеграция с Tauri и Vue 3 позволяет легко использовать библиотеку в приложениях с графическим интерфейсом.
