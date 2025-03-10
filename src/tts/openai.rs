use crate::error::{Error, Result, ErrorType};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use crate::logging::{log_debug, log_info, log_error, log_warning, log_trace};

/// Модели голосов OpenAI TTS
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenAiVoice {
    /// Alloy: универсальный голос с нейтральным тоном
    #[serde(rename = "alloy")]
    Alloy,
    /// Echo: голос с низким тоном и ясной артикуляцией
    #[serde(rename = "echo")]
    Echo,
    /// Fable: выразительный голос, подходящий для повествования
    #[serde(rename = "fable")]
    Fable,
    /// Onyx: глубокий мужской голос с авторитетным тоном
    #[serde(rename = "onyx")]
    Onyx,
    /// Nova: женский голос с мягким тоном
    #[serde(rename = "nova")]
    Nova,
    /// Shimmer: молодой женский голос с ясной артикуляцией
    #[serde(rename = "shimmer")]
    Shimmer,
}

impl Default for OpenAiVoice {
    fn default() -> Self {
        Self::Alloy
    }
}

impl OpenAiVoice {
    /// Возвращает строковое представление голоса
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Alloy => "alloy",
            Self::Echo => "echo",
            Self::Fable => "fable",
            Self::Onyx => "onyx",
            Self::Nova => "nova",
            Self::Shimmer => "shimmer",
        }
    }
    
    /// Создает OpenAiVoice из строки
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "alloy" => Ok(Self::Alloy),
            "echo" => Ok(Self::Echo),
            "fable" => Ok(Self::Fable),
            "onyx" => Ok(Self::Onyx),
            "nova" => Ok(Self::Nova),
            "shimmer" => Ok(Self::Shimmer),
            _ => Err(Error::InvalidParameters(format!("Unknown voice: {}", s))),
        }
    }
}

/// Модели для TTS OpenAI
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenAiTtsModel {
    /// tts-1: стандартная модель TTS
    #[serde(rename = "tts-1")]
    Tts1,
    /// tts-1-hd: модель TTS высокого качества
    #[serde(rename = "tts-1-hd")]
    Tts1Hd,
}

impl Default for OpenAiTtsModel {
    fn default() -> Self {
        Self::Tts1
    }
}

impl OpenAiTtsModel {
    /// Возвращает строковое представление модели
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tts1 => "tts-1",
            Self::Tts1Hd => "tts-1-hd",
        }
    }
    
    /// Создает OpenAiTtsModel из строки
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "tts-1" => Ok(Self::Tts1),
            "tts-1-hd" => Ok(Self::Tts1Hd),
            _ => Err(Error::InvalidParameters(format!("Unknown model: {}", s))),
        }
    }
}

/// Формат аудио для OpenAI TTS
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenAiAudioFormat {
    /// MP3 формат
    #[serde(rename = "mp3")]
    Mp3,
    /// AAC формат
    #[serde(rename = "aac")]
    Aac,
    /// FLAC формат
    #[serde(rename = "flac")]
    Flac,
    /// Opus формат
    #[serde(rename = "opus")]
    Opus,
    /// PCM формат
    #[serde(rename = "pcm")]
    Pcm,
}

impl Default for OpenAiAudioFormat {
    fn default() -> Self {
        Self::Mp3
    }
}

impl OpenAiAudioFormat {
    /// Возвращает строковое представление формата
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Aac => "aac",
            Self::Flac => "flac",
            Self::Opus => "opus",
            Self::Pcm => "pcm",
        }
    }
    
    /// Создает OpenAiAudioFormat из строки
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "mp3" => Ok(Self::Mp3),
            "aac" => Ok(Self::Aac),
            "flac" => Ok(Self::Flac),
            "opus" => Ok(Self::Opus),
            "pcm" => Ok(Self::Pcm),
            _ => Err(Error::InvalidParameters(format!("Unknown audio format: {}", s))),
        }
    }
    
    /// Возвращает расширение файла для формата
    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Aac => "aac",
            Self::Flac => "flac",
            Self::Opus => "opus",
            Self::Pcm => "wav", // PCM обычно сохраняется как WAV
        }
    }
}

/// Настройки для генерации TTS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsOptions {
    /// Модель TTS
    pub model: OpenAiTtsModel,
    /// Голос
    pub voice: OpenAiVoice,
    /// Скорость речи (0.25 - 4.0)
    pub speed: f32,
    /// Формат аудио
    pub response_format: OpenAiAudioFormat,
}

impl Default for TtsOptions {
    fn default() -> Self {
        Self {
            model: OpenAiTtsModel::default(),
            voice: OpenAiVoice::default(),
            speed: 1.0,
            response_format: OpenAiAudioFormat::default(),
        }
    }
}

/// Запрос к OpenAI TTS API
#[derive(Debug, Clone, Serialize)]
struct TtsRequest {
    model: String,
    input: String,
    voice: String,
    response_format: String,
    speed: f32,
}

/// Сегмент TTS
#[derive(Debug, Clone)]
pub struct TtsSegment {
    /// Текст сегмента
    pub text: String,
    /// Аудио данные
    pub audio_data: Vec<u8>,
    /// Длительность аудио в секундах
    pub duration: Option<f64>,
    /// Целевая длительность (из субтитров)
    pub target_duration: f64,
    /// Коэффициент растяжения/сжатия
    pub stretch_factor: Option<f64>,
}

/// Клиент для работы с OpenAI TTS API
pub struct OpenAiTts {
    api_key: String,
    options: TtsOptions,
    client: reqwest::Client,
}

impl OpenAiTts {
    /// Создает новый клиент для работы с OpenAI TTS API
    pub fn new(api_key: String, options: TtsOptions) -> Self {
        Self {
            api_key,
            options,
            client: reqwest::Client::new(),
        }
    }
    
    /// Создает новый клиент для работы с OpenAI TTS API с настройками по умолчанию
    pub fn with_api_key(api_key: String) -> Self {
        Self::new(api_key, TtsOptions::default())
    }
    
    /// Генерирует TTS для указанного текста
    pub async fn generate_speech(&self, text: &str) -> Result<Vec<u8>> {
        log_debug(&format!("OpenAI TTS запрос: '{}' с использованием голоса {} и модели {}", 
            text, self.options.voice.as_str(), self.options.model.as_str()));
        
        let client = reqwest::Client::new();
        
        let form = reqwest::multipart::Form::new()
            .text("model", self.options.model.as_str().to_string())
            .text("voice", self.options.voice.as_str().to_string())
            .text("response_format", self.options.response_format.as_str().to_string())
            .text("speed", self.options.speed.to_string())
            .text("input", text.to_string());

        let response = client.post("https://api.openai.com/v1/audio/speech")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| Error::new(
                ErrorType::OpenAi,
                &format!("Ошибка при отправке запроса к OpenAI TTS API: {}", e)
            ))?;
            
        let status = response.status();
        log_debug(&format!("Получен ответ от OpenAI API, статус: {}", status));
        
        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Не удалось получить текст ошибки".to_string());
            
            log_error::<(), _>(
                &Error::new(ErrorType::OpenAi, &error_text),
                &format!("OpenAI API вернул ошибку: {}", status)
            )?;
            
            return Err(Error::new(
                ErrorType::OpenAi,
                &format!("Ошибка OpenAI API: {}. {}", status, error_text)
            ));
        }
        
        let audio_data = response.bytes().await
            .map_err(|e| Error::new(
                ErrorType::OpenAi,
                &format!("Ошибка при получении данных от OpenAI TTS API: {}", e)
            ))?;
            
        let bytes = audio_data.to_vec();
        let size = bytes.len();
        
        if size < 100 {
            log_warning(&format!("Получены подозрительно малые данные от OpenAI TTS: {} байт", size));
            
            // Для отладки выведем первые несколько байт
            if !bytes.is_empty() {
                let debug_bytes = bytes.iter().take(16).map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
                log_debug(&format!("Первые 16 байт: {}", debug_bytes));
            }
        } else {
            log_debug(&format!("Получено {} байт аудио данных от OpenAI TTS API", size));
        }
        
        Ok(bytes)
    }
    
    /// Генерирует TTS для текста и сохраняет в файл
    pub async fn generate_speech_to_file<P: AsRef<Path>>(&self, text: &str, path: P) -> Result<()> {
        let audio_data = self.generate_speech(text).await?;
        
        let mut file = File::create(path).await
            .map_err(|e| Error::Io(e))?;
        
        file.write_all(&audio_data).await
            .map_err(|e| Error::Io(e))?;
        
        Ok(())
    }
    
    /// Генерирует TTS для сегмента субтитров
    pub async fn generate_segment(&self, text: &str, target_duration: f64) -> Result<TtsSegment> {
        let audio_data = self.generate_speech(text).await?;
        
        // Длительность аудио будет определена позже при анализе аудио
        let segment = TtsSegment {
            text: text.to_string(),
            audio_data,
            duration: None,
            target_duration,
            stretch_factor: None,
        };
        
        Ok(segment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_voice_as_str() {
        assert_eq!(OpenAiVoice::Alloy.as_str(), "alloy");
        assert_eq!(OpenAiVoice::Echo.as_str(), "echo");
        assert_eq!(OpenAiVoice::Fable.as_str(), "fable");
        assert_eq!(OpenAiVoice::Onyx.as_str(), "onyx");
        assert_eq!(OpenAiVoice::Nova.as_str(), "nova");
        assert_eq!(OpenAiVoice::Shimmer.as_str(), "shimmer");
    }
    
    #[test]
    fn test_voice_from_str() {
        assert_eq!(OpenAiVoice::from_str("alloy").unwrap(), OpenAiVoice::Alloy);
        assert_eq!(OpenAiVoice::from_str("echo").unwrap(), OpenAiVoice::Echo);
        assert_eq!(OpenAiVoice::from_str("fable").unwrap(), OpenAiVoice::Fable);
        assert_eq!(OpenAiVoice::from_str("onyx").unwrap(), OpenAiVoice::Onyx);
        assert_eq!(OpenAiVoice::from_str("nova").unwrap(), OpenAiVoice::Nova);
        assert_eq!(OpenAiVoice::from_str("shimmer").unwrap(), OpenAiVoice::Shimmer);
        
        // Проверка регистронезависимости
        assert_eq!(OpenAiVoice::from_str("ALLOY").unwrap(), OpenAiVoice::Alloy);
        
        // Проверка ошибки
        assert!(OpenAiVoice::from_str("unknown").is_err());
    }
    
    #[test]
    fn test_model_as_str() {
        assert_eq!(OpenAiTtsModel::Tts1.as_str(), "tts-1");
        assert_eq!(OpenAiTtsModel::Tts1Hd.as_str(), "tts-1-hd");
    }
    
    #[test]
    fn test_model_from_str() {
        assert_eq!(OpenAiTtsModel::from_str("tts-1").unwrap(), OpenAiTtsModel::Tts1);
        assert_eq!(OpenAiTtsModel::from_str("tts-1-hd").unwrap(), OpenAiTtsModel::Tts1Hd);
        
        // Проверка регистронезависимости
        assert_eq!(OpenAiTtsModel::from_str("TTS-1").unwrap(), OpenAiTtsModel::Tts1);
        
        // Проверка ошибки
        assert!(OpenAiTtsModel::from_str("unknown").is_err());
    }
    
    #[test]
    fn test_audio_format_as_str() {
        assert_eq!(OpenAiAudioFormat::Mp3.as_str(), "mp3");
        assert_eq!(OpenAiAudioFormat::Aac.as_str(), "aac");
        assert_eq!(OpenAiAudioFormat::Flac.as_str(), "flac");
        assert_eq!(OpenAiAudioFormat::Opus.as_str(), "opus");
        assert_eq!(OpenAiAudioFormat::Pcm.as_str(), "pcm");
    }
    
    #[test]
    fn test_audio_format_from_str() {
        assert_eq!(OpenAiAudioFormat::from_str("mp3").unwrap(), OpenAiAudioFormat::Mp3);
        assert_eq!(OpenAiAudioFormat::from_str("aac").unwrap(), OpenAiAudioFormat::Aac);
        assert_eq!(OpenAiAudioFormat::from_str("flac").unwrap(), OpenAiAudioFormat::Flac);
        assert_eq!(OpenAiAudioFormat::from_str("opus").unwrap(), OpenAiAudioFormat::Opus);
        assert_eq!(OpenAiAudioFormat::from_str("pcm").unwrap(), OpenAiAudioFormat::Pcm);
        
        // Проверка регистронезависимости
        assert_eq!(OpenAiAudioFormat::from_str("MP3").unwrap(), OpenAiAudioFormat::Mp3);
        
        // Проверка ошибки
        assert!(OpenAiAudioFormat::from_str("unknown").is_err());
    }
    
    #[test]
    fn test_audio_format_file_extension() {
        assert_eq!(OpenAiAudioFormat::Mp3.file_extension(), "mp3");
        assert_eq!(OpenAiAudioFormat::Aac.file_extension(), "aac");
        assert_eq!(OpenAiAudioFormat::Flac.file_extension(), "flac");
        assert_eq!(OpenAiAudioFormat::Opus.file_extension(), "opus");
        assert_eq!(OpenAiAudioFormat::Pcm.file_extension(), "wav");
    }
}