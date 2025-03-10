use crate::error::Result;
use std::sync::{Arc, Mutex};

/// Тип для функций обратного вызова прогресса
pub type ProgressCallback = Box<dyn Fn(f32, &str) + Send + 'static>;

/// Трекер прогресса
#[derive(Clone)]
pub struct ProgressTracker {
    /// Текущий прогресс (от 0.0 до 100.0)
    progress: Arc<Mutex<f32>>,
    /// Текущий статус
    status: Arc<Mutex<String>>,
    /// Функция обратного вызова для отслеживания прогресса
    callback: Option<Arc<ProgressCallback>>,
}

impl ProgressTracker {
    /// Создает новый трекер прогресса
    pub fn new() -> Self {
        Self {
            progress: Arc::new(Mutex::new(0.0)),
            status: Arc::new(Mutex::new(String::new())),
            callback: None,
        }
    }
    
    /// Создает новый трекер прогресса с функцией обратного вызова
    pub fn with_callback(callback: ProgressCallback) -> Self {
        Self {
            progress: Arc::new(Mutex::new(0.0)),
            status: Arc::new(Mutex::new(String::new())),
            callback: Some(Arc::new(callback)),
        }
    }
    
    /// Устанавливает функцию обратного вызова
    pub fn set_callback(&mut self, callback: ProgressCallback) {
        self.callback = Some(Arc::new(callback));
    }
    
    /// Обновляет прогресс
    pub fn update(&self, progress: f32, status: &str) -> Result<()> {
        // Ограничиваем прогресс от 0 до 100
        let clamped_progress = progress.max(0.0).min(100.0);
        
        // Обновляем прогресс и статус
        {
            let mut p = self.progress.lock().unwrap();
            *p = clamped_progress;
        }
        
        {
            let mut s = self.status.lock().unwrap();
            *s = status.to_string();
        }
        
        // Вызываем функцию обратного вызова, если она установлена
        if let Some(callback) = &self.callback {
            callback(clamped_progress, status);
        }
        
        Ok(())
    }
    
    /// Возвращает текущий прогресс
    pub fn get_progress(&self) -> f32 {
        *self.progress.lock().unwrap()
    }
    
    /// Возвращает текущий статус
    pub fn get_status(&self) -> String {
        self.status.lock().unwrap().clone()
    }
    
    /// Создает дочерний трекер прогресса с заданным диапазоном
    pub fn create_child(&self, start: f32, end: f32) -> ChildProgressTracker {
        ChildProgressTracker {
            parent: self.clone(),
            start,
            end,
        }
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Дочерний трекер прогресса
#[derive(Clone)]
pub struct ChildProgressTracker {
    /// Родительский трекер прогресса
    parent: ProgressTracker,
    /// Начальное значение прогресса в родительском трекере
    start: f32,
    /// Конечное значение прогресса в родительском трекере
    end: f32,
}

impl ChildProgressTracker {
    /// Обновляет прогресс
    pub fn update(&self, progress: f32, status: &str) -> Result<()> {
        // Ограничиваем прогресс от 0 до 100
        let clamped_progress = progress.max(0.0).min(100.0);
        
        // Масштабируем прогресс в диапазон родительского трекера
        let parent_progress = self.start + (self.end - self.start) * clamped_progress / 100.0;
        
        // Обновляем прогресс в родительском трекере
        self.parent.update(parent_progress, status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    
    #[test]
    fn test_progress_tracker() {
        let tracker = ProgressTracker::new();
        
        // Проверяем начальные значения
        assert_eq!(tracker.get_progress(), 0.0);
        assert_eq!(tracker.get_status(), "");
        
        // Обновляем прогресс
        tracker.update(50.0, "Halfway there").unwrap();
        
        // Проверяем обновленные значения
        assert_eq!(tracker.get_progress(), 50.0);
        assert_eq!(tracker.get_status(), "Halfway there");
        
        // Проверяем ограничение прогресса
        tracker.update(150.0, "Over the limit").unwrap();
        assert_eq!(tracker.get_progress(), 100.0);
        
        tracker.update(-10.0, "Under the limit").unwrap();
        assert_eq!(tracker.get_progress(), 0.0);
    }
    
    #[test]
    fn test_progress_callback() {
        let (tx, rx) = mpsc::channel();
        
        let callback = Box::new(move |progress: f32, status: &str| {
            tx.send((progress, status.to_string())).unwrap();
        });
        
        let tracker = ProgressTracker::with_callback(callback);
        
        // Обновляем прогресс
        tracker.update(25.0, "Quarter done").unwrap();
        
        // Проверяем, что callback был вызван
        let (progress, status) = rx.recv().unwrap();
        assert_eq!(progress, 25.0);
        assert_eq!(status, "Quarter done");
    }
    
    #[test]
    fn test_child_progress_tracker() {
        let parent = ProgressTracker::new();
        let child = parent.create_child(50.0, 75.0);
        
        // Обновляем прогресс в дочернем трекере
        child.update(50.0, "Child halfway").unwrap();
        
        // Проверяем, что прогресс в родительском трекере обновился правильно
        assert_eq!(parent.get_progress(), 62.5); // 50 + (75-50) * 0.5 = 62.5
        assert_eq!(parent.get_status(), "Child halfway");
    }
}
