use std::{fs, io::Write, path::Path};

pub fn timestamp(ms: i64, separator: char) -> String {
    let ms = ms.max(0);
    format!(
        "{:02}:{:02}:{:02}{}{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        separator,
        ms % 1000
    )
}

fn parse_time(time: &str) -> Result<i64, String> {
    let t = time.trim().replace(',', ".");
    let parts: Vec<_> = t.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return Err("Invalid caption timestamp".into());
    }
    let seconds: f64 = parts[parts.len() - 1]
        .parse()
        .map_err(|_| "Invalid caption seconds")?;
    let minutes: i64 = parts[parts.len() - 2]
        .parse()
        .map_err(|_| "Invalid caption minutes")?;
    let hours: i64 = if parts.len() == 3 {
        parts[0].parse().map_err(|_| "Invalid caption hours")?
    } else {
        0
    };
    if !seconds.is_finite()
        || !(0.0..60.0).contains(&seconds)
        || !(0..60).contains(&minutes)
        || !(0..=9999).contains(&hours)
    {
        return Err("Caption timestamp is out of range".into());
    }
    Ok(hours * 3_600_000 + minutes * 60_000 + (seconds * 1000.0).round() as i64)
}

pub fn to_vtt(input: &str) -> Result<String, String> {
    let normalized = input
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    if normalized.starts_with("WEBVTT") {
        return Ok(normalized);
    }
    let mut output = String::from("WEBVTT\n\n");
    let mut count = 0;
    for block in normalized.split("\n\n") {
        let lines: Vec<_> = block.lines().collect();
        let Some(index) = lines.iter().position(|l| l.contains("-->")) else {
            continue;
        };
        let (start, end) = lines[index].split_once("-->").unwrap();
        let start = parse_time(start)?;
        let end = parse_time(end)?;
        if end <= start {
            return Err("Caption end precedes its start".into());
        }
        output.push_str(&format!(
            "{} --> {}\n",
            timestamp(start, '.'),
            timestamp(end, '.')
        ));
        for line in &lines[index + 1..] {
            output.push_str(line);
            output.push('\n');
        }
        output.push('\n');
        count += 1;
    }
    if count == 0 {
        return Err("No valid subtitle cues found".into());
    }
    Ok(output)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub position: f64,
    pub end: f64,
    pub text: String,
}

pub fn cues(input: &str) -> Result<Vec<Cue>, String> {
    let normalized = to_vtt(input)?;
    let mut result = Vec::new();
    for block in normalized.split("\n\n") {
        if block.starts_with("NOTE") || block.starts_with("STYLE") || block.starts_with("REGION") {
            continue;
        }
        let lines: Vec<_> = block.lines().collect();
        let Some(index) = lines.iter().position(|l| l.contains("-->")) else {
            continue;
        };
        let (start, end) = lines[index].split_once("-->").unwrap();
        let start = parse_time(start)?;
        let end = parse_time(end.split_whitespace().next().ok_or("Missing caption end")?)?;
        if end <= start {
            return Err("Caption end precedes its start".into());
        }
        let text = plain_text(&lines[index + 1..].join("\n"));
        if !text.trim().is_empty() {
            result.push(Cue {
                position: start as f64 / 1000.0,
                end: end as f64 / 1000.0,
                text,
            });
        }
    }
    if result.is_empty() {
        return Err("No valid subtitle cues found".into());
    }
    Ok(result)
}
fn plain_text(input: &str) -> String {
    let mut output = String::new();
    let mut rest = input;
    while let Some(open) = rest.find('<') {
        output.push_str(&rest[..open]);
        rest = &rest[open..];
        if let Some(close) = rest.find('>') {
            let tag = &rest[1..close];
            let name = tag
                .trim_start_matches('/')
                .split([' ', '.'])
                .next()
                .unwrap_or_default();
            if [
                "b", "i", "u", "c", "v", "lang", "ruby", "rt", "font", "span", "br",
            ]
            .contains(&name)
                || (tag.contains(':') && parse_time(tag).is_ok())
            {
                rest = &rest[close + 1..];
                continue;
            }
        }
        output.push('<');
        rest = &rest[1..];
    }
    output.push_str(rest);
    output
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

pub fn atomic_write_new(path: &Path, text: &str) -> Result<(), String> {
    let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        // A hard link publishes atomically and refuses to overwrite an existing caption.
        fs::hard_link(&temp, path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    let _ = fs::remove_file(temp);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn converts_bom_crlf_and_multiline() {
        let vtt =
            to_vtt("\u{feff}1\r\n00:00:01,250 --> 00:00:02,500\r\nHello\r\nworld\r\n").unwrap();
        assert!(vtt.contains("00:00:01.250 --> 00:00:02.500\nHello\nworld"));
        assert!(to_vtt("1\n00:00:09,000 --> 00:00:01,000\nx").is_err());
        assert!(to_vtt("garbage").is_err());
    }
    #[test]
    fn parses_vtt_settings_and_preserves_plain_code_text() {
        let cues = cues("WEBVTT\n\nNOTE ignored --> note\n\nfirst\n00:01.250 --> 00:03.000 align:start\n<v Speaker><b>Café</b></v> &amp; x < 10\n\n00:04.000 --> 00:06.000\nVec<T> and <00:04.500>code").unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].position, 1.25);
        assert_eq!(cues[0].text, "Café & x < 10");
        assert_eq!(cues[1].text, "Vec<T> and code");
        assert!(super::cues("WEBVTT\n\n00:03.000 --> 00:01.000\nInvalid").is_err());
    }
    #[test]
    fn publication_never_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.srt");
        atomic_write_new(&path, "first").unwrap();
        assert!(atomic_write_new(&path, "second").is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "first");
    }
}
