use crate::error::{Error, Result};
use crate::vtt::models::{Subtitle, SubtitleTrack};
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Парсер VTT файлов
pub struct VttParser;

impl VttParser {
    /// Парсит VTT файл и возвращает коллекцию субтитров
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<SubtitleTrack> {
        let file = File::open(path).map_err(|e| Error::Io(e))?;
        let reader = BufReader::new(file);
        Self::parse_reader(reader)
    }

    /// Парсит VTT из строки и возвращает коллекцию субтитров
    pub fn parse_str(content: &str) -> Result<SubtitleTrack> {
        let reader = BufReader::new(content.as_bytes());
        Self::parse_reader(reader)
    }

    /// Парсит VTT из любого источника, реализующего BufRead
    pub fn parse_reader<R: BufRead>(reader: R) -> Result<SubtitleTrack> {
        let mut subtitle_track = SubtitleTrack::new();
        let mut lines = reader.lines();
        
        // Проверка заголовка WebVTT
        if let Some(Ok(first_line)) = lines.next() {
            if !first_line.trim().starts_with("WEBVTT") {
                return Err(Error::VttParsing("Invalid WebVTT file: missing WEBVTT header".to_string()));
            }
        } else {
            // Пустой файл - возвращаем пустой трек
            return Ok(subtitle_track);
        }
        
        // Регулярное выражение для парсинга временных меток
        let timestamp_regex = Regex::new(r"(\d{2}):(\d{2}):(\d{2})\.(\d{3})\s*-->\s*(\d{2}):(\d{2}):(\d{2})\.(\d{3})")
            .map_err(|e| Error::VttParsing(format!("Failed to compile regex: {}", e)))?;
        
        let mut current_start_time: Option<f64> = None;
        let mut current_end_time: Option<f64> = None;
        let mut current_text = String::new();
        
        // Парсинг субтитров
        for line_result in lines {
            let line = line_result.map_err(|e| Error::Io(e))?;
            let trimmed_line = line.trim();
            
            if let Some(captures) = timestamp_regex.captures(trimmed_line) {
                // Если у нас уже есть временные метки и текст, добавляем предыдущий субтитр
                if current_start_time.is_some() && !current_text.trim().is_empty() {
                    subtitle_track.add(Subtitle::new(
                        current_start_time.unwrap(),
                        current_end_time.unwrap(),
                        current_text.trim().to_string(),
                    ));
                    current_text.clear();
                }
                
                // Парсинг новых временных меток
                let start_time = Self::parse_timestamp(
                    &captures[1], &captures[2], &captures[3], &captures[4]
                )?;
                
                let end_time = Self::parse_timestamp(
                    &captures[5], &captures[6], &captures[7], &captures[8]
                )?;
                
                current_start_time = Some(start_time);
                current_end_time = Some(end_time);
            } else if trimmed_line.is_empty() {
                // Пустая строка может означать конец субтитра
                if current_start_time.is_some() && !current_text.trim().is_empty() {
                    subtitle_track.add(Subtitle::new(
                        current_start_time.unwrap(),
                        current_end_time.unwrap(),
                        current_text.trim().to_string(),
                    ));
                    current_text.clear();
                    current_start_time = None;
                    current_end_time = None;
                }
            } else if current_start_time.is_some() {
                // Добавляем текст к текущему субтитру
                if !current_text.is_empty() {
                    current_text.push('\n');
                }
                current_text.push_str(trimmed_line);
            }
        }
        
        // Добавляем последний субтитр, если он есть
        if current_start_time.is_some() && !current_text.trim().is_empty() {
            subtitle_track.add(Subtitle::new(
                current_start_time.unwrap(),
                current_end_time.unwrap(),
                current_text.trim().to_string(),
            ));
        }
        
        // Сортируем субтитры по времени начала
        subtitle_track.sort();
        
        Ok(subtitle_track)
    }
    
    /// Парсит временную метку и возвращает время в секундах
    fn parse_timestamp(hours: &str, minutes: &str, seconds: &str, milliseconds: &str) -> Result<f64> {
        let hours: u32 = hours.parse()
            .map_err(|_| Error::VttParsing(format!("Invalid hours: {}", hours)))?;
        
        let minutes: u32 = minutes.parse()
            .map_err(|_| Error::VttParsing(format!("Invalid minutes: {}", minutes)))?;
        
        let seconds: u32 = seconds.parse()
            .map_err(|_| Error::VttParsing(format!("Invalid seconds: {}", seconds)))?;
        
        let milliseconds: u32 = milliseconds.parse()
            .map_err(|_| Error::VttParsing(format!("Invalid milliseconds: {}", milliseconds)))?;
        
        let total_seconds = (hours as f64) * 3600.0 + 
                           (minutes as f64) * 60.0 + 
                           (seconds as f64) + 
                           (milliseconds as f64) / 1000.0;
        
        Ok(total_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_timestamp() {
        assert_eq!(VttParser::parse_timestamp("00", "00", "00", "000").unwrap(), 0.0);
        assert_eq!(VttParser::parse_timestamp("00", "00", "01", "000").unwrap(), 1.0);
        assert_eq!(VttParser::parse_timestamp("00", "01", "00", "000").unwrap(), 60.0);
        assert_eq!(VttParser::parse_timestamp("01", "00", "00", "000").unwrap(), 3600.0);
        assert_eq!(VttParser::parse_timestamp("00", "00", "00", "500").unwrap(), 0.5);
        assert_eq!(VttParser::parse_timestamp("01", "30", "45", "500").unwrap(), 5445.5);
    }
    
    #[test]
    fn test_parse_str_simple() {
        let vtt = "WEBVTT\n\n00:00:01.000 --> 00:00:04.000\nHello, world!\n\n00:00:05.000 --> 00:00:08.000\nThis is a test.";
        let track = VttParser::parse_str(vtt).unwrap();
        
        assert_eq!(track.len(), 2);
        assert_eq!(track.subtitles[0].start_time, 1.0);
        assert_eq!(track.subtitles[0].end_time, 4.0);
        assert_eq!(track.subtitles[0].text, "Hello, world!");
        assert_eq!(track.subtitles[1].start_time, 5.0);
        assert_eq!(track.subtitles[1].end_time, 8.0);
        assert_eq!(track.subtitles[1].text, "This is a test.");
    }
    
    #[test]
    fn test_parse_str_with_multiline_text() {
        let vtt = "WEBVTT\n\n00:00:01.000 --> 00:00:04.000\nHello,\nworld!\n\n00:00:05.000 --> 00:00:08.000\nThis is\na test.";
        let track = VttParser::parse_str(vtt).unwrap();
        
        assert_eq!(track.len(), 2);
        assert_eq!(track.subtitles[0].text, "Hello,\nworld!");
        assert_eq!(track.subtitles[1].text, "This is\na test.");
    }
    
    #[test]
    fn test_parse_str_with_cue_identifiers() {
        let vtt = "WEBVTT\n\n1\n00:00:01.000 --> 00:00:04.000\nHello, world!\n\n2\n00:00:05.000 --> 00:00:08.000\nThis is a test.";
        let track = VttParser::parse_str(vtt).unwrap();
        
        assert_eq!(track.len(), 2);
        assert_eq!(track.subtitles[0].text, "Hello, world!");
        assert_eq!(track.subtitles[1].text, "This is a test.");
    }
    
    #[test]
    fn test_parse_str_with_header_metadata() {
        let vtt = "WEBVTT\nKind: captions\nLanguage: en\n\n00:00:01.000 --> 00:00:04.000\nHello, world!";
        let track = VttParser::parse_str(vtt).unwrap();
        
        assert_eq!(track.len(), 1);
        assert_eq!(track.subtitles[0].text, "Hello, world!");
    }
    
    #[test]
    fn test_parse_str_invalid_header() {
        let vtt = "NOT WEBVTT\n\n00:00:01.000 --> 00:00:04.000\nHello, world!";
        let result = VttParser::parse_str(vtt);
        
        assert!(result.is_err());
        if let Err(Error::VttParsing(msg)) = result {
            assert!(msg.contains("missing WEBVTT header"));
        } else {
            panic!("Expected VttParsing error");
        }
    }
}
