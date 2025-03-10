use thiserror::Error;

/// Типы ошибок, которые могут возникнуть при синхронизации
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    /// Ошибка ввода/вывода
    Io,
    /// Ошибка парсинга VTT
    VttParsing,
    /// Ошибка OpenAI API
    OpenAi,
    /// Ошибка обработки аудио
    AudioProcessingError,
    /// Ошибка синхронизации
    Synchronization,
    /// Неверные параметры
    InvalidParameters,
}

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

    #[error("Ошибка логирования: {0}")]
    LoggedError(String),
}

impl Error {
    /// Создает новую ошибку указанного типа с сообщением
    pub fn new(error_type: ErrorType, message: &str) -> Self {
        match error_type {
            ErrorType::Io => Self::Io(std::io::Error::new(std::io::ErrorKind::Other, message)),
            ErrorType::VttParsing => Self::VttParsing(message.to_string()),
            ErrorType::OpenAi => Self::OpenAi(message.to_string()),
            ErrorType::AudioProcessingError => Self::AudioProcessing(message.to_string()),
            ErrorType::Synchronization => Self::Synchronization(message.to_string()),
            ErrorType::InvalidParameters => Self::InvalidParameters(message.to_string()),
        }
    }
}

/// Результат с обработкой ошибок
pub type Result<T> = std::result::Result<T, Error>;
