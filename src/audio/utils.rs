use crate::error::{Error, Result, ErrorType};
use crate::audio::models::AudioData;
use crate::logging::{log_debug, log_info, log_warning, log_error};
use std::io::Cursor;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::audio::{SampleBuffer};
use symphonia::core::probe::Hint;
use symphonia::default::{get_probe, get_codecs};

/// Декодирует MP3 данные в аудио сэмплы
pub fn decode_mp3_to_samples(mp3_data: &[u8], sample_rate: u32, _channels: u16) -> Result<AudioData> {
    log_debug(&format!("Декодирование MP3 данных размером {} байт", mp3_data.len()));
    
    if mp3_data.is_empty() {
        log_warning("Получены пустые MP3 данные");
        return Err(Error::new(ErrorType::AudioProcessingError, "Пустые MP3 данные"));
    }

    // Проверяем MP3 заголовок
    if mp3_data.len() < 4 {
        log_warning("MP3 данные слишком короткие для анализа заголовка");
        return Err(Error::new(ErrorType::AudioProcessingError, "Некорректные MP3 данные"));
    }

    // Создаем источник данных из бинарного буфера
    // Делаем копию данных, чтобы избежать проблемы с временем жизни
    let mp3_data_owned = mp3_data.to_vec();
    let cursor = Cursor::new(mp3_data_owned);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    // Настраиваем пробер формата
    let format_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let metadata_opts = MetadataOptions::default();
    let probe = get_probe();
    let hint = Hint::new();

    // Определяем формат
    let probe_result = match probe.format(&hint, mss, &format_opts, &metadata_opts) {
        Ok(probe_result) => {
            log_debug(&format!("Формат декодирован успешно"));
            probe_result
        },
        Err(err) => {
            log_error::<(), _>(&Error::new(ErrorType::AudioProcessingError, &format!("Ошибка определения формата: {}", err)), "Ошибка декодирования");
            return Err(Error::new(ErrorType::AudioProcessingError, &format!("Ошибка определения формата: {}", err)));
        }
    };

    // Получаем первый поток
    let track = match probe_result.format.default_track() {
        Some(track) => track,
        None => {
            log_error::<(), _>(&Error::new(ErrorType::AudioProcessingError, "Не найден аудио поток"), "Ошибка декодирования");
            return Err(Error::new(ErrorType::AudioProcessingError, "Не найден аудио поток"));
        }
    };

    // Проверяем, что это аудио поток
    if track.codec_params.codec == CODEC_TYPE_NULL {
        log_error::<(), _>(&Error::new(ErrorType::AudioProcessingError, "Поток не является аудио"), "Ошибка декодирования");
        return Err(Error::new(ErrorType::AudioProcessingError, "Поток не является аудио"));
    }

    // Создаем декодер для потока
    let decoder_opts = DecoderOptions::default();
    let codec_params = track.codec_params.clone();
    let decoder = match get_codecs().make(&codec_params, &decoder_opts) {
        Ok(decoder) => decoder,
        Err(err) => {
            log_error::<(), _>(&Error::new(ErrorType::AudioProcessingError, &format!("Ошибка создания декодера: {}", err)), "Ошибка декодирования");
            return Err(Error::new(ErrorType::AudioProcessingError, &format!("Ошибка создания декодера: {}", err)));
        }
    };

    let track_id = track.id;
    let mut format = probe_result.format;
    let mut decoder = decoder;
    let mut _sample_count = 0;
    let mut all_samples = Vec::new();

    // Декодируем пакеты
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(_)) |
            Err(symphonia::core::errors::Error::ResetRequired) => {
                // Достигнут конец потока или требуется сброс
                break;
            },
            Err(err) => {
                log_warning(&format!("Ошибка чтения пакета: {}, пропускаем", err));
                continue;
            }
        };

        // Пропускаем пакеты, не относящиеся к нашему треку
        if packet.track_id() != track_id {
            continue;
        }

        // Декодируем пакет
        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Получаем буфер сэмплов
                let spec = *decoded.spec();
                let duration = decoded.capacity() as u64;
                
                log_debug(&format!(
                    "Декодировано {} сэмплов, канал: {}, частота: {}", 
                    duration, spec.channels.count(), spec.rate
                ));

                // Создаем буфер для сэмплов
                let mut sample_buffer = SampleBuffer::<f32>::new(duration, spec);
                
                // Заполняем буфер и конвертируем в f32
                sample_buffer.copy_planar_ref(decoded);
                let samples = sample_buffer.samples();
                all_samples.extend_from_slice(samples);
                
                _sample_count += samples.len();
            },
            Err(err) => {
                log_warning(&format!("Ошибка декодирования пакета: {}, пропускаем", err));
                continue;
            }
        }
    }
    
    if all_samples.is_empty() {
        log_warning("Не удалось декодировать аудио данные - получены пустые сэмплы");
        return Err(Error::new(ErrorType::AudioProcessingError, "Не получены аудио сэмплы"));
    }
    
    // Преобразуем сэмплы в моно если требуется
    // Расчёт происходит на уровне уже имеющихся сэмплов
    log_debug(&format!("Декодировано всего {} сэмплов. Создаём AudioData...", all_samples.len()));
    
    // Возвращаем AudioData с заданной частотой дискретизации
    Ok(AudioData {
        samples: all_samples,
        sample_rate,
        channels: 1 // По умолчанию используем монофонический формат
    })
}

/// Преобразует многоканальное аудио в моно
fn convert_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }

    let mut mono = Vec::with_capacity(samples.len() / channels);
    
    for chunk in samples.chunks(channels) {
        let sum: f32 = chunk.iter().sum();
        mono.push(sum / channels as f32);
    }
    
    mono
} 