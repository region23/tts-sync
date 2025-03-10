use crate::error::Result;
use crate::audio::models::{AudioSegment, AudioTrack};
use crate::audio::adjustment::tempo::{TempoAdjuster, TempoAlgorithm};
use crate::logging::{log_info, log_debug};

/// Синхронизатор аудио
pub struct AudioSynchronizer {
    sample_rate: u32,
    channels: u16,
    normalize_volume: bool,
}

impl AudioSynchronizer {
    /// Создает новый экземпляр AudioSynchronizer
    pub fn new(sample_rate: u32, channels: u16, normalize_volume: bool) -> Self {
        Self {
            sample_rate,
            channels,
            normalize_volume,
        }
    }

    /// Синхронизирует аудио с субтитрами
    pub fn synchronize(&self, audio_track: &AudioTrack, target_duration: f64) -> Result<AudioTrack> {
        log_info("Начало синхронизации аудио");
        
        let mut synchronized_track = AudioTrack::new(self.sample_rate, self.channels);
        
        for segment in &audio_track.segments {
            log_debug(&format!("Обработка сегмента: {:?}", segment));
            
            // Анализируем сегмент и определяем необходимые изменения
            let adjusted_segment = TempoAdjuster::fit_to_duration(
                &segment.audio,
                target_duration as f32,
                TempoAlgorithm::Sinc
            )?;
            
            synchronized_track.add_segment(AudioSegment::new(
                adjusted_segment,
                segment.start_time,
                segment.end_time,
                segment.text.clone()
            ));
        }
        
        // Нормализуем громкость, если требуется
        if self.normalize_volume {
            log_debug("Применение нормализации громкости");
            synchronized_track.normalize_volume()?;
        }
        
        log_info("Синхронизация аудио завершена");
        Ok(synchronized_track)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_synchronizer_creation() {
        let synchronizer = AudioSynchronizer::new(44100, 1, true);
        assert_eq!(synchronizer.sample_rate, 44100);
        assert_eq!(synchronizer.channels, 1);
        assert!(synchronizer.normalize_volume);
    }
} 