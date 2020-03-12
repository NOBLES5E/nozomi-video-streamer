use chrono::Timelike;
use anyhow::Result;

/// Parse start time like 00:11:21 to number of seconds.
pub fn start_time_to_seconds(start_time: &str) -> Result<u32> {
    let time = chrono::NaiveTime::parse_from_str(start_time, "%H:%M:%S")?;
    Ok(time.num_seconds_from_midnight())
}

/// Escape ffmpeg filtergraph parameter
/// See https://ffmpeg.org/ffmpeg-filters.html#toc-Notes-on-filtergraph-escaping
pub fn ffmpeg_filtergraph_escaping(raw_string: &str) -> String {
    // first level
    let result = raw_string.replace(r#"'"#, r#"\'"#);
    let result = result.replace(r#":"#, r#"\:"#);
    // second levresult
    let result = result.replace(r#"\"#, r#"\\"#);
    let result = result.replace(r#"'"#, r#"\'"#);
    let result = result.replace(r#"["#, r#"\["#);
    let result = result.replace(r#"]"#, r#"\]"#);
    let result = result.replace(r#","#, r#"\,"#);
    let result = result.replace(r#";"#, r#"\;"#);
    log::info!("ffmpeg filter graph {:?}", result);
    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_time_to_seconds() -> Result<()> {
        assert_eq!(start_time_to_seconds("00:01:20")?, 80);
        Ok(())
    }

    #[test]
    fn test_ffmpeg_filtergraph_escaping() -> Result<()> {
        assert_eq!(
            ffmpeg_filtergraph_escaping(
                "this is a 'string': may contain one, or more, special characters"
            ),
            r#"this is a \\\'string\\\'\\: may contain one\, or more\, special characters"#.to_string()
        );
        Ok(())
    }
}
