use thiserror::Error;

mod logging;

pub use logging::{
    setup_logging, log_error, log_warning, log_info, log_debug, log_trace
};

/// Ошибки, которые могут возникнуть при синхронизации
#[derive(Debug, Error)]
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
    
    #[error("{0}")]
    LoggedError(String),
    
    #[error("Ошибка HTTP запроса: {0}")]
    HttpRequest(#[from] reqwest::Error),
    
    #[error("Ошибка сериализации JSON: {0}")]
    JsonSerialization(#[from] serde_json::Error),
    
    #[error("Ошибка обработки аудио: {0}")]
    AudioError(String),
    
    #[error("Ошибка в библиотеке rubato: {0}")]
    RubatoError(String),
    
    #[error("Неизвестная ошибка: {0}")]
    Unknown(String),
}

impl Error {
    /// Логирует ошибку
    pub fn log(&self) {
        logging::log_error::<(), _>(self, "Произошла ошибка").ok();
    }
    
    /// Создает ошибку из строки с указанным типом
    pub fn from_str(error_type: ErrorType, message: &str) -> Self {
        match error_type {
            ErrorType::VttParsing => Self::VttParsing(message.to_string()),
            ErrorType::OpenAi => Self::OpenAi(message.to_string()),
            ErrorType::AudioProcessing => Self::AudioProcessing(message.to_string()),
            ErrorType::Synchronization => Self::Synchronization(message.to_string()),
            ErrorType::InvalidParameters => Self::InvalidParameters(message.to_string()),
            ErrorType::AudioError => Self::AudioError(message.to_string()),
            ErrorType::RubatoError => Self::RubatoError(message.to_string()),
            ErrorType::Unknown => Self::Unknown(message.to_string()),
        }
    }
}

/// Типы ошибок
#[derive(Debug, Clone, Copy)]
pub enum ErrorType {
    VttParsing,
    OpenAi,
    AudioProcessing,
    Synchronization,
    InvalidParameters,
    AudioError,
    RubatoError,
    Unknown,
}

/// Результат с обработкой ошибок
pub type Result<T> = std::result::Result<T, Error>;
