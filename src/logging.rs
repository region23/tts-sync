use log::{debug, error, info, trace, warn, LevelFilter, Log};
use env_logger::Builder;
use std::io::Write;
use std::sync::Once;

static INIT: Once = Once::new();

#[derive(Clone)]
pub struct TestLogger {
    level: LevelFilter,
}

impl TestLogger {
    pub fn new(level: LevelFilter) -> Self {
        Self { level }
    }
}

impl Log for TestLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            println!(
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

/// Настраивает логирование для библиотеки
pub fn setup_logging(level: LevelFilter) {
    let mut builder = Builder::new();
    
    builder.filter_level(level);
    builder.format(|buf, record| {
        writeln!(
            buf,
            "{} [{}] - {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.args()
        )
    });
    
    let _ = builder.try_init();
    
    info!("Логирование настроено с уровнем: {}", level);
}

/// Настраивает логирование для тестов
pub fn setup_test_logging(level: LevelFilter) {
    INIT.call_once(|| {
        let logger = TestLogger::new(level);
        let _ = log::set_boxed_logger(Box::new(logger));
    });
}

/// Логирует ошибку и возвращает её
pub fn log_error<T, E: std::fmt::Display>(err: E, message: &str) -> Result<T, crate::error::Error> {
    let error_message = format!("{}: {}", message, err);
    error!("{}", error_message);
    Err(crate::error::Error::LoggedError(error_message))
}

/// Логирует предупреждение
pub fn log_warning(message: &str) {
    warn!("{}", message);
}

/// Логирует информационное сообщение
pub fn log_info(message: &str) {
    info!("{}", message);
}

/// Логирует отладочное сообщение
pub fn log_debug(message: &str) {
    debug!("{}", message);
}

/// Логирует трассировочное сообщение
pub fn log_trace(message: &str) {
    trace!("{}", message);
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::Level;
    
    #[test]
    fn test_setup_logging() {
        // Проверяем, что логирование настраивается без ошибок
        setup_logging(LevelFilter::Debug);
        
        // Проверяем, что уровень логирования установлен правильно
        assert!(log::log_enabled!(Level::Debug));
        assert!(log::log_enabled!(Level::Info));
        assert!(log::log_enabled!(Level::Warn));
        assert!(log::log_enabled!(Level::Error));
        assert!(!log::log_enabled!(Level::Trace));
    }
}
