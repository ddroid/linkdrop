use std::time::Duration;

use crate::error::LinkdropError;

pub fn parse_ttl(input: &str) -> Result<Duration, LinkdropError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(LinkdropError::InvalidSlug("ttl cannot be empty".into()));
    }

    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Ok(Duration::from_secs(seconds));
    }

    let (num_part, unit) = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .map(|idx| trimmed.split_at(idx))
        .unwrap_or((trimmed, ""));

    let amount = num_part
        .parse::<u64>()
        .map_err(|_| LinkdropError::InvalidSlug(format!("invalid ttl: {input}")))?;

    let seconds = match unit {
        "s" | "sec" | "secs" => amount,
        "m" | "min" | "mins" => amount * 60,
        "h" | "hr" | "hrs" => amount * 3600,
        "d" | "day" | "days" => amount * 86400,
        "" => amount,
        _ => {
            return Err(LinkdropError::InvalidSlug(format!(
                "invalid ttl unit in: {input}"
            )));
        }
    };

    Ok(Duration::from_secs(seconds))
}
