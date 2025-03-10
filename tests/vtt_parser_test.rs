use tts_sync::{VttParser, Result, vtt::{Subtitle, SubtitleTrack}, error::Error};
use tempfile::NamedTempFile;

#[test]
fn test_parse_empty_file() -> Result<()> {
    // Создаем пустой временный файл
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();
    
    // Парсим пустой файл
    let subtitles = VttParser::parse_file(&temp_path)?;
    
    // Проверяем, что результат пуст
    assert_eq!(subtitles.len(), 0);
    
    Ok(())
}

#[test]
fn test_parse_valid_vtt() -> Result<()> {
    // Создаем временный файл с валидным VTT содержимым
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();
    
    let vtt_content = r#"WEBVTT

00:00:01.000 --> 00:00:05.000
Hello, world!

00:00:06.000 --> 00:00:10.000
This is a test.
"#;
    
    std::fs::write(&temp_path, vtt_content).unwrap();
    
    // Парсим файл
    let subtitles = VttParser::parse_file(&temp_path)?;
    
    // Проверяем результат
    assert_eq!(subtitles.len(), 2);
    
    // Проверяем первый субтитр
    let first = &subtitles[0];
    assert_eq!(first.start_time, 1.0);
    assert_eq!(first.end_time, 5.0);
    assert_eq!(first.text, "Hello, world!");
    
    // Проверяем второй субтитр
    let second = &subtitles[1];
    assert_eq!(second.start_time, 6.0);
    assert_eq!(second.end_time, 10.0);
    assert_eq!(second.text, "This is a test.");
    
    Ok(())
}

#[test]
fn test_parse_vtt_with_metadata() -> Result<()> {
    // Создаем временный файл с VTT содержимым, включающим метаданные
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();
    
    let vtt_content = r#"WEBVTT - Some title

NOTE
This is a comment and should be ignored.

00:00:01.000 --> 00:00:05.000
Hello, world!

NOTE Another comment
This should also be ignored.

00:00:06.000 --> 00:00:10.000
This is a test.
"#;
    
    std::fs::write(&temp_path, vtt_content).unwrap();
    
    // Парсим файл
    let subtitles = VttParser::parse_file(&temp_path)?;
    
    // Проверяем результат
    assert_eq!(subtitles.len(), 2);
    
    // Проверяем первый субтитр
    let first = &subtitles[0];
    assert_eq!(first.start_time, 1.0);
    assert_eq!(first.end_time, 5.0);
    assert_eq!(first.text, "Hello, world!");
    
    // Проверяем второй субтитр
    let second = &subtitles[1];
    assert_eq!(second.start_time, 6.0);
    assert_eq!(second.end_time, 10.0);
    assert_eq!(second.text, "This is a test.");
    
    Ok(())
}

#[test]
fn test_parse_vtt_with_multiline_text() -> Result<()> {
    // Создаем временный файл с VTT содержимым, включающим многострочный текст
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();
    
    let vtt_content = r#"WEBVTT

00:00:01.000 --> 00:00:05.000
Hello, world!
This is a multiline
subtitle text.

00:00:06.000 --> 00:00:10.000
This is a test.
"#;
    
    std::fs::write(&temp_path, vtt_content).unwrap();
    
    // Парсим файл
    let subtitles = VttParser::parse_file(&temp_path)?;
    
    // Проверяем результат
    assert_eq!(subtitles.len(), 2);
    
    // Проверяем первый субтитр
    let first = &subtitles[0];
    assert_eq!(first.start_time, 1.0);
    assert_eq!(first.end_time, 5.0);
    assert_eq!(first.text, "Hello, world!\nThis is a multiline\nsubtitle text.");
    
    // Проверяем второй субтитр
    let second = &subtitles[1];
    assert_eq!(second.start_time, 6.0);
    assert_eq!(second.end_time, 10.0);
    assert_eq!(second.text, "This is a test.");
    
    Ok(())
}

#[test]
fn test_parse_vtt_with_milliseconds() -> Result<()> {
    // Создаем временный файл с VTT содержимым, включающим миллисекунды
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();
    
    let vtt_content = r#"WEBVTT

00:00:01.500 --> 00:00:05.750
Hello, world!

00:00:06.250 --> 00:00:10.800
This is a test.
"#;
    
    std::fs::write(&temp_path, vtt_content).unwrap();
    
    // Парсим файл
    let subtitles = VttParser::parse_file(&temp_path)?;
    
    // Проверяем результат
    assert_eq!(subtitles.len(), 2);
    
    // Проверяем первый субтитр
    let first = &subtitles[0];
    assert_eq!(first.start_time, 1.5);
    assert_eq!(first.end_time, 5.75);
    assert_eq!(first.text, "Hello, world!");
    
    // Проверяем второй субтитр
    let second = &subtitles[1];
    assert_eq!(second.start_time, 6.25);
    assert_eq!(second.end_time, 10.8);
    assert_eq!(second.text, "This is a test.");
    
    Ok(())
}

#[test]
fn test_parse_vtt_with_hours() -> Result<()> {
    // Создаем временный файл с VTT содержимым, включающим часы
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();
    
    let vtt_content = r#"WEBVTT

01:30:01.000 --> 01:30:05.000
Hello, world!

02:15:06.000 --> 02:15:10.000
This is a test.
"#;
    
    std::fs::write(&temp_path, vtt_content).unwrap();
    
    // Парсим файл
    let subtitles = VttParser::parse_file(&temp_path)?;
    
    // Проверяем результат
    assert_eq!(subtitles.len(), 2);
    
    // Проверяем первый субтитр
    let first = &subtitles[0];
    assert_eq!(first.start_time, 5401.0); // 1*3600 + 30*60 + 1
    assert_eq!(first.end_time, 5405.0);   // 1*3600 + 30*60 + 5
    assert_eq!(first.text, "Hello, world!");
    
    // Проверяем второй субтитр
    let second = &subtitles[1];
    assert_eq!(second.start_time, 8106.0); // 2*3600 + 15*60 + 6
    assert_eq!(second.end_time, 8110.0);   // 2*3600 + 15*60 + 10
    assert_eq!(second.text, "This is a test.");
    
    Ok(())
}

#[test]
fn test_subtitle_track_operations() -> Result<()> {
    // Создаем пустой трек субтитров
    let mut track = SubtitleTrack::new();
    
    // Проверяем, что трек пуст
    assert_eq!(track.len(), 0);
    
    // Добавляем субтитры
    track.add(Subtitle {
        start_time: 1.0,
        end_time: 5.0,
        text: "Hello, world!".to_string(),
    });
    
    track.add(Subtitle {
        start_time: 6.0,
        end_time: 10.0,
        text: "This is a test.".to_string(),
    });
    
    // Проверяем, что субтитры добавлены
    assert_eq!(track.len(), 2);
    
    // Проверяем итерацию по субтитрам
    let mut iter = track.iter();
    let first = iter.next().unwrap();
    assert_eq!(first.start_time, 1.0);
    assert_eq!(first.end_time, 5.0);
    assert_eq!(first.text, "Hello, world!");
    
    let second = iter.next().unwrap();
    assert_eq!(second.start_time, 6.0);
    assert_eq!(second.end_time, 10.0);
    assert_eq!(second.text, "This is a test.");
    
    assert!(iter.next().is_none());
    
    // Проверяем доступ по индексу
    assert_eq!(track[0].text, "Hello, world!");
    assert_eq!(track[1].text, "This is a test.");
    
    // Проверяем сортировку субтитров
    let mut unsorted_track = SubtitleTrack::new();
    
    unsorted_track.add(Subtitle {
        start_time: 6.0,
        end_time: 10.0,
        text: "This is a test.".to_string(),
    });
    
    unsorted_track.add(Subtitle {
        start_time: 1.0,
        end_time: 5.0,
        text: "Hello, world!".to_string(),
    });
    
    // Сортируем трек
    unsorted_track.sort();
    
    // Проверяем, что субтитры отсортированы по времени начала
    assert_eq!(unsorted_track[0].start_time, 1.0);
    assert_eq!(unsorted_track[1].start_time, 6.0);
    
    Ok(())
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