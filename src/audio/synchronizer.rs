use crate::error::{Error, Result};
use crate::audio::models::{AudioData, AudioSegment, AudioTrack};
use crate::audio::analysis::analyzer::{AudioAnalyzer, SegmentAnalysis};
use crate::audio::adjustment::tempo::TempoAdjuster;
use crate::vtt::{Subtitle, SubtitleTrack};

/// Синхронизатор аудио с субтитрами
pub struct AudioSynchronizer;

impl AudioSynchronizer {
    /// Синхронизирует аудио сегменты с субтитрами
    pub fn synchronize_with_subtitles(
        audio_segments: &[AudioSegment],
        subtitles: &SubtitleTrack,
    ) -> Result<AudioTrack> {
        if audio_segments.len() != subtitles.len() {
            return Err(Error::Synchronization(format!(
                "Number of audio segments ({}) doesn't match number of subtitles ({})",
                audio_segments.len(), subtitles.len()
            )));
        }

        if audio_segments.is_empty() {
            return Err(Error::Synchronization("No audio segments to synchronize".to_string()));
        }

        // Определяем частоту дискретизации и количество каналов из первого сегмента
        let sample_rate = audio_segments[0].audio.sample_rate;
        let channels = audio_segments[0].audio.channels;

        // Создаем аудио трек
        let mut audio_track = AudioTrack::new(sample_rate, channels);

        // Получаем целевые длительности из субтитров
        let target_durations: Vec<f64> = subtitles.iter()
            .map(|subtitle| subtitle.duration())
            .collect();

        // Анализируем каждый сегмент и определяем оптимальный подход к синхронизации
        let mut segment_analyses = Vec::with_capacity(audio_segments.len());
        for (i, segment) in audio_segments.iter().enumerate() {
            let analysis = AudioAnalyzer::analyze_segment(segment, target_durations[i])?;
            segment_analyses.push(analysis);
        }

        // Синхронизируем каждый сегмент
        for (i, segment) in audio_segments.iter().enumerate() {
            let analysis = &segment_analyses[i];
            let subtitle = &subtitles.subtitles[i];

            let synchronized_segment = if analysis.should_split {
                // Если сегмент нужно разбить на части, делаем это
                Self::synchronize_complex_segment(segment, subtitle, analysis)?
            } else {
                // Иначе просто подстраиваем темп
                TempoAdjuster::fit_segment_to_duration(segment, subtitle.duration())?
            };

            // Добавляем синхронизированный сегмент в трек
            audio_track.add_segment(synchronized_segment);
        }

        // Сортируем сегменты по времени начала
        audio_track.sort_by_start_time();

        Ok(audio_track)
    }

    /// Синхронизирует сложный сегмент, разбивая его на части
    fn synchronize_complex_segment(
        segment: &AudioSegment,
        subtitle: &Subtitle,
        analysis: &SegmentAnalysis,
    ) -> Result<AudioSegment> {
        // Разбиваем сегмент на части по паузам
        let sub_segments = AudioAnalyzer::split_segment(segment)?;

        if sub_segments.len() <= 1 {
            // Если не удалось разбить на части, просто подстраиваем темп
            return TempoAdjuster::fit_segment_to_duration(segment, subtitle.duration());
        }

        // Определяем целевые длительности для каждой части
        // Распределяем общую целевую длительность пропорционально исходным длительностям
        let total_duration: f64 = sub_segments.iter()
            .map(|s| s.audio.duration())
            .sum();

        let target_durations: Vec<f64> = sub_segments.iter()
            .map(|s| (s.audio.duration() / total_duration) * subtitle.duration())
            .collect();

        // Подстраиваем темп каждой части
        let adjusted_segments = TempoAdjuster::fit_segments_to_durations(
            &sub_segments,
            &target_durations,
        )?;

        // Объединяем части в один сегмент
        let mut merged_samples = Vec::new();
        for segment in &adjusted_segments {
            merged_samples.extend_from_slice(&segment.audio.samples);
        }

        let audio = AudioData::new(
            merged_samples,
            segment.audio.sample_rate,
            segment.audio.channels,
        );

        Ok(AudioSegment::new(
            audio,
            subtitle.start_time,
            subtitle.end_time,
            subtitle.text.clone(),
        ))
    }

    /// Добавляет паузы между сегментами аудио
    pub fn add_pauses_between_segments(audio_track: &AudioTrack) -> Result<AudioTrack> {
        if audio_track.is_empty() {
            return Ok(audio_track.clone());
        }

        let mut result_track = AudioTrack::new(
            audio_track.sample_rate,
            audio_track.channels,
        );

        // Сортируем сегменты по времени начала
        let mut segments = audio_track.segments.clone();
        segments.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

        // Добавляем первый сегмент
        result_track.add_segment(segments[0].clone());

        // Обрабатываем остальные сегменты
        for i in 1..segments.len() {
            let prev_segment = &segments[i - 1];
            let curr_segment = &segments[i];

            // Проверяем, есть ли пауза между сегментами
            let pause_duration = curr_segment.start_time - prev_segment.end_time;

            if pause_duration > 0.01 {
                // Если есть пауза, создаем сегмент тишины
                let pause_samples = (pause_duration * audio_track.sample_rate as f64) as usize;
                let silence = vec![0.0; pause_samples];

                let pause_audio = AudioData::new(
                    silence,
                    audio_track.sample_rate,
                    audio_track.channels,
                );

                let pause_segment = AudioSegment::new(
                    pause_audio,
                    prev_segment.end_time,
                    curr_segment.start_time,
                    String::new(), // Пустой текст для паузы
                );

                result_track.add_segment(pause_segment);
            }

            // Добавляем текущий сегмент
            result_track.add_segment(curr_segment.clone());
        }

        Ok(result_track)
    }

    /// Нормализует громкость аудио трека
    pub fn normalize_track(audio_track: &AudioTrack, target_peak: f32) -> Result<AudioTrack> {
        if audio_track.is_empty() {
            return Ok(audio_track.clone());
        }

        // Объединяем все сегменты для анализа общей громкости
        let merged_audio = audio_track.merge()?;

        // Находим максимальную амплитуду
        let max_amplitude = AudioAnalyzer::find_peak_amplitude(&merged_audio.samples);

        if max_amplitude <= 0.0 {
            return Ok(audio_track.clone());
        }

        // Рассчитываем коэффициент усиления
        let gain = target_peak / max_amplitude;

        // Применяем усиление к каждому сегменту
        let mut normalized_track = AudioTrack::new(
            audio_track.sample_rate,
            audio_track.channels,
        );

        for segment in &audio_track.segments {
            let mut normalized_samples = segment.audio.samples.clone();
            
            for sample in &mut normalized_samples {
                *sample *= gain;
            }

            let normalized_audio = AudioData::new(
                normalized_samples,
                segment.audio.sample_rate,
                segment.audio.channels,
            );

            let normalized_segment = AudioSegment::new(
                normalized_audio,
                segment.start_time,
                segment.end_time,
                segment.text.clone(),
            );

            normalized_track.add_segment(normalized_segment);
        }

        Ok(normalized_track)
    }
}

#[cfg(test)]
mod tests {
    // Тесты будут добавлены после полной реализации
    // Для тестирования AudioSynchronizer требуются реальные аудио данные
}
