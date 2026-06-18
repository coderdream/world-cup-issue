use crate::models::Match;
use chrono::{Duration, FixedOffset, NaiveDate, NaiveTime, TimeZone, Utc};

const MATCH_PENDING_WINDOW_MINUTES: i64 = 180;

pub fn normalize_matches(matches: Vec<Match>) -> Vec<Match> {
    let mut normalized: Vec<Match> = matches.into_iter().map(normalize_match).collect();
    normalized.sort_by(|a, b| (a.date.as_str(), a.time.as_str(), a.id.as_str()).cmp(&(b.date.as_str(), b.time.as_str(), b.id.as_str())));
    normalized
}

pub fn normalize_match(mut item: Match) -> Match {
    item.status = derive_status(&item);
    item
}

fn derive_status(item: &Match) -> String {
    let Some(start) = beijing_start(item) else {
        if item.score.home.is_some() && item.score.away.is_some() {
            return "finished".to_string();
        }
        return item.status.clone();
    };
    let now = Utc::now();
    let pending_end = start + Duration::minutes(MATCH_PENDING_WINDOW_MINUTES);

    if item.status == "live" && now < pending_end {
        return "live".to_string();
    }
    if item.score.home.is_some() && item.score.away.is_some() {
        return "finished".to_string();
    }
    if now >= start && now < pending_end {
        "live".to_string()
    } else if now >= pending_end {
        "finished".to_string()
    } else {
        "scheduled".to_string()
    }
}

fn beijing_start(item: &Match) -> Option<chrono::DateTime<Utc>> {
    let date = NaiveDate::parse_from_str(&item.date, "%Y-%m-%d").ok()?;
    let time = NaiveTime::parse_from_str(&item.time, "%H:%M").ok()?;
    let beijing = FixedOffset::east_opt(8 * 3600)?;
    beijing
        .from_local_datetime(&date.and_time(time))
        .single()
        .map(|value| value.with_timezone(&Utc))
}
