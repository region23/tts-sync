use std::time::Duration;

/// Структура данных для представления субтитра
#[derive(Debug, Clone, PartialEq)]
pub struct Subtitle {
    /// Время начала субтитра в секундах
    pub start_time: f64,
    /// Время окончания субтитра в секундах
    pub end_time: f64,
    /// Текст субтитра
    pub text: String,
}

impl Subtitle {
    /// Создает новый субтитр
    pub fn new(start_time: f64, end_time: f64, text: String) -> Self {
        Self {
            start_time,
            end_time,
            text,
        }
    }

    /// Возвращает длительность субтитра в секундах
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }

    /// Возвращает длительность субтитра как Duration
    pub fn duration_as_duration(&self) -> Duration {
        Duration::from_secs_f64(self.duration())
    }
}

/// Коллекция субтитров
#[derive(Debug, Clone, Default)]
pub struct SubtitleTrack {
    /// Субтитры
    pub subtitles: Vec<Subtitle>,
}

impl SubtitleTrack {
    /// Создает новую пустую коллекцию субтитров
    pub fn new() -> Self {
        Self {
            subtitles: Vec::new(),
        }
    }

    /// Добавляет субтитр в коллекцию
    pub fn add(&mut self, subtitle: Subtitle) {
        self.subtitles.push(subtitle);
    }

    /// Возвращает количество субтитров в коллекции
    pub fn len(&self) -> usize {
        self.subtitles.len()
    }

    /// Проверяет, пуста ли коллекция субтитров
    pub fn is_empty(&self) -> bool {
        self.subtitles.is_empty()
    }

    /// Возвращает итератор по субтитрам
    pub fn iter(&self) -> impl Iterator<Item = &Subtitle> {
        self.subtitles.iter()
    }

    /// Сортирует субтитры по времени начала
    pub fn sort(&mut self) {
        self.subtitles.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
    }

    /// Возвращает общую длительность всех субтитров
    pub fn total_duration(&self) -> f64 {
        if self.subtitles.is_empty() {
            return 0.0;
        }
        
        let min_start = self.subtitles.iter()
            .map(|s| s.start_time)
            .fold(f64::INFINITY, f64::min);
            
        let max_end = self.subtitles.iter()
            .map(|s| s.end_time)
            .fold(0.0, f64::max);
            
        max_end - min_start
    }
}

impl std::ops::Index<usize> for SubtitleTrack {
    type Output = Subtitle;

    fn index(&self, index: usize) -> &Self::Output {
        &self.subtitles[index]
    }
}
