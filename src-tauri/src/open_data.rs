use crate::{
    http_client::build_clients,
    match_status::normalize_matches,
    models::{ConnectivityTestResult, Match, Score},
};
use chrono::{FixedOffset, NaiveDate, NaiveTime, TimeZone};
use reqwest::Client;
use serde::Deserialize;
use std::{collections::HashMap, fmt, time::Duration};

const DATA_SOURCES: &[&str] = &[
    "https://pub-9d9e6c0cb6934fb0a0c505e3c64f39b2.r2.dev/cupwatch/data/worldcup-2026.json",
    "https://cdn.jsdelivr.net/gh/openfootball/worldcup.json@master/2026/worldcup.json",
    "https://raw.githubusercontent.com/openfootball/worldcup.json/master/2026/worldcup.json",
];

#[derive(Debug)]
pub enum OpenDataError {
    Http(reqwest::Error),
    Json(serde_json::Error),
    Empty,
    AllSourcesFailed(Vec<String>),
}

impl fmt::Display for OpenDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(err) => write!(f, "network error: {err}"),
            Self::Json(err) => write!(f, "data parse error: {err}"),
            Self::Empty => write!(f, "open data source returned no matches"),
            Self::AllSourcesFailed(errors) => write!(f, "all open data sources failed: {}", errors.join("; ")),
        }
    }
}

impl std::error::Error for OpenDataError {}

impl From<reqwest::Error> for OpenDataError {
    fn from(value: reqwest::Error) -> Self {
        Self::Http(value)
    }
}

impl From<serde_json::Error> for OpenDataError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub struct OpenDataRefresh {
    pub source: String,
    pub payload: String,
    pub matches: Vec<Match>,
}

pub struct FootballDataRefresh {
    pub source: String,
    pub payload: String,
    pub matches: Vec<Match>,
}

#[derive(Debug, Deserialize)]
struct OpenFootballCup {
    matches: Vec<OpenFootballMatch>,
}

#[derive(Debug, Deserialize)]
struct OpenFootballMatch {
    round: String,
    num: Option<i64>,
    date: String,
    time: String,
    team1: String,
    team2: String,
    score: Option<OpenFootballScore>,
    group: Option<String>,
    ground: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenFootballScore {
    ft: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
struct FootballDataMatchesResponse {
    matches: Vec<FootballDataMatch>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FootballDataMatch {
    id: i64,
    status: String,
    utc_date: String,
    home_team: FootballDataTeam,
    away_team: FootballDataTeam,
    score: FootballDataScore,
    venue: Option<String>,
    stage: Option<String>,
    group: Option<String>,
    goals: Option<Vec<FootballDataGoal>>,
}

#[derive(Debug, Deserialize)]
struct FootballDataTeam {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FootballDataScore {
    full_time: Option<FootballDataScorePair>,
    half_time: Option<FootballDataScorePair>,
    extra_time: Option<FootballDataScorePair>,
    penalties: Option<FootballDataScorePair>,
}

#[derive(Debug, Deserialize)]
struct FootballDataScorePair {
    home: Option<i64>,
    away: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct FootballDataGoal {
    score: Option<FootballDataScorePair>,
}

pub async fn refresh_open_football() -> Result<OpenDataRefresh, OpenDataError> {
    let clients = build_clients(Duration::from_secs(8));
    let mut failures = Vec::new();
    let mut best: Option<OpenDataRefresh> = None;

    for source in DATA_SOURCES {
        let mut source_refresh = None;
        for client in &clients {
            match fetch_source(client, source).await {
                Ok(refresh) => {
                    source_refresh = Some(refresh);
                    break;
                }
                Err(error) => failures.push(format!("{source}: {error}")),
            }
        }
        if let Some(refresh) = source_refresh {
            if best
                .as_ref()
                .map(|current| score_count(&refresh.matches) > score_count(&current.matches))
                .unwrap_or(true)
            {
                best = Some(refresh);
            }
        }
    }

    best.ok_or(OpenDataError::AllSourcesFailed(failures))
}

pub async fn refresh_football_data(token: &str) -> Result<Option<FootballDataRefresh>, OpenDataError> {
    if token.trim().is_empty() {
        return Ok(None);
    }

    let clients = build_clients(Duration::from_secs(12));
    let mut failures = Vec::new();

    for client in &clients {
        match fetch_football_data(client, token).await {
            Ok(refresh) => return Ok(Some(refresh)),
            Err(error) => failures.push(error.to_string()),
        }
    }

    Err(OpenDataError::AllSourcesFailed(failures))
}

pub async fn test_football_data_token(token: &str) -> ConnectivityTestResult {
    let token = token.trim();
    if token.is_empty() {
        return ConnectivityTestResult {
            ok: false,
            message: "请先输入 API Token".to_string(),
            details: None,
        };
    }

    let clients = build_clients(Duration::from_secs(12));
    let mut errors = Vec::new();
    for client in &clients {
        match client
            .get("https://api.football-data.org/v4/competitions/WC/matches")
            .header("X-Auth-Token", token)
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    return ConnectivityTestResult {
                        ok: true,
                        message: "Token 可用，football-data.org 已返回 WC 赛事数据".to_string(),
                        details: None,
                    };
                }
                let body = response.text().await.unwrap_or_default();
                log::warn!("football-data token test failed with status {status}: {body}");
                return ConnectivityTestResult {
                    ok: false,
                    message: format!("Token 测试失败：HTTP {status}"),
                    details: if body.is_empty() { None } else { Some(body) },
                };
            }
            Err(error) => {
                log::warn!("football-data token test request failed: {error}");
                errors.push(error.to_string());
            }
        }
    }

    ConnectivityTestResult {
        ok: false,
        message: "Token 测试失败：无法连接 football-data.org".to_string(),
        details: if errors.is_empty() { None } else { Some(errors.join("; ")) },
    }
}

async fn fetch_source(client: &Client, source: &str) -> Result<OpenDataRefresh, OpenDataError> {
    let response = client.get(source).send().await?.error_for_status()?;
    let payload = response.text().await?;
    let cup: OpenFootballCup = serde_json::from_str(&payload)?;
    let mut matches = normalize_matches(parse_matches(cup.matches));
    if matches.is_empty() {
        return Err(OpenDataError::Empty);
    }
    matches.sort_by(|a, b| (a.date.as_str(), a.time.as_str(), a.id.as_str()).cmp(&(b.date.as_str(), b.time.as_str(), b.id.as_str())));
    Ok(OpenDataRefresh {
        source: source.to_string(),
        payload,
        matches,
    })
}

async fn fetch_football_data(client: &Client, token: &str) -> Result<FootballDataRefresh, OpenDataError> {
    let url = "https://api.football-data.org/v4/competitions/WC/matches";
    let response = client
        .get(url)
        .header("X-Auth-Token", token)
        .header("X-Unfold-Goals", "true")
        .send()
        .await?
        .error_for_status()?;
    let payload = response.text().await?;
    let api: FootballDataMatchesResponse = serde_json::from_str(&payload)?;
    let mut matches = api.matches.into_iter().map(convert_football_data_match).collect::<Vec<_>>();
    matches = normalize_matches(matches);
    Ok(FootballDataRefresh {
        source: "football-data.org".to_string(),
        payload,
        matches,
    })
}

fn parse_matches(raw_matches: Vec<OpenFootballMatch>) -> Vec<Match> {
    let team_ids = team_name_map();
    raw_matches
        .into_iter()
        .enumerate()
        .map(|(index, raw)| {
            let group = raw.group.as_deref().and_then(|value| value.strip_prefix("Group ")).unwrap_or("").to_string();
            let (date, time) = beijing_datetime(&raw.date, &raw.time).unwrap_or_else(|| (raw.date.clone(), raw.time.clone()));
            let score = raw.score.as_ref().and_then(|score| score.ft.as_ref()).filter(|ft| ft.len() >= 2);
            let stage = stage_label(&raw.round);
            Match {
                id: format!("m{:03}", raw.num.unwrap_or(index as i64 + 1)),
                group,
                stage,
                date,
                time,
                utc_offset: "UTC+8".to_string(),
                home_team_id: lookup_team_id(&team_ids, &raw.team1),
                away_team_id: lookup_team_id(&team_ids, &raw.team2),
                score: Score {
                    home: score.map(|ft| ft[0]),
                    away: score.map(|ft| ft[1]),
                },
                status: if score.is_some() { "finished" } else { "scheduled" }.to_string(),
                venue: raw.ground.clone().unwrap_or_default(),
                city: raw.ground.unwrap_or_default(),
                updated_at: Some(chrono::Local::now().format("%H:%M:%S").to_string()),
            }
        })
        .collect()
}

fn convert_football_data_match(item: FootballDataMatch) -> Match {
    let (date, time) = utc_to_beijing(&item.utc_date).unwrap_or_else(|| ("".to_string(), "".to_string()));
    let score = select_football_score(&item);
    Match {
        id: format!("fd-{}", item.id),
        group: football_data_group_label(item.group),
        stage: football_data_stage_label(item.stage).unwrap_or_else(|| stage_label_from_status(&item.status)),
        date,
        time,
        utc_offset: "UTC+8".to_string(),
        home_team_id: team_id_from_name(&item.home_team.name),
        away_team_id: team_id_from_name(&item.away_team.name),
        score,
        status: match_status_from_football_data(&item.status),
        venue: item.venue.clone().unwrap_or_default(),
        city: item.venue.unwrap_or_default(),
        updated_at: Some(chrono::Local::now().format("%H:%M:%S").to_string()),
    }
}

fn select_football_score(item: &FootballDataMatch) -> Score {
    let full_time = item
        .score
        .full_time
        .as_ref()
        .and_then(|pair| pair.home.zip(pair.away));
    let latest_goal = latest_goal_score(item);
    let live_interval = item
        .score
        .half_time
        .as_ref()
        .and_then(|pair| pair.home.zip(pair.away))
        .or_else(|| {
            item.score
                .extra_time
                .as_ref()
                .and_then(|pair| pair.home.zip(pair.away))
        })
        .or_else(|| {
            item.score
                .penalties
                .as_ref()
                .and_then(|pair| pair.home.zip(pair.away))
        });
    let pair = full_time
        .or(latest_goal)
        .or(live_interval)
        .or_else(|| matches!(item.status.as_str(), "IN_PLAY" | "PAUSED").then_some((0, 0)));
    Score {
        home: pair.map(|(home, _)| home),
        away: pair.map(|(_, away)| away),
    }
}

fn match_status_from_football_data(status: &str) -> String {
    match status {
        "FINISHED" => "finished".to_string(),
        "IN_PLAY" | "PAUSED" => "live".to_string(),
        _ => "scheduled".to_string(),
    }
}

fn stage_label_from_status(status: &str) -> String {
    match status {
        "FINISHED" => "已结束".to_string(),
        "IN_PLAY" | "PAUSED" => "进行中".to_string(),
        _ => "未开始".to_string(),
    }
}

fn utc_to_beijing(value: &str) -> Option<(String, String)> {
    let instant = chrono::DateTime::parse_from_rfc3339(value).ok()?;
    let beijing = instant.with_timezone(&FixedOffset::east_opt(8 * 3600)?);
    Some((beijing.format("%Y-%m-%d").to_string(), beijing.format("%H:%M").to_string()))
}

fn team_id_from_name(name: &str) -> String {
    team_name_map()
        .get(name)
        .copied()
        .map(str::to_string)
        .unwrap_or_else(|| format!("slot-{}", name.to_ascii_lowercase().replace([' ', '/', '&'], "-")))
}

fn football_data_stage_label(stage: Option<String>) -> Option<String> {
    stage.map(|value| match value.as_str() {
        "GROUP_STAGE" => "小组赛".to_string(),
        "LAST_16" => "16 强".to_string(),
        "QUARTER_FINALS" => "1/4 决赛".to_string(),
        "SEMI_FINALS" => "半决赛".to_string(),
        "THIRD_PLACE" => "季军赛".to_string(),
        "FINAL" => "决赛".to_string(),
        other => other.to_string(),
    })
}

fn football_data_group_label(group: Option<String>) -> String {
    group
        .unwrap_or_default()
        .replace("GROUP_", "")
        .replace("Group ", "")
        .replace('_', "")
}

fn latest_goal_score(item: &FootballDataMatch) -> Option<(i64, i64)> {
    item.goals.as_ref().and_then(|goals| {
        goals
            .iter()
            .rev()
            .find_map(|goal| goal.score.as_ref().and_then(|score| score.home.zip(score.away)))
    })
}

pub fn merge_matches_with_football_data(base: Vec<Match>, overlay: Vec<Match>) -> Vec<Match> {
    let mut overlay_map: HashMap<String, Match> = overlay
        .into_iter()
        .map(|item| (match_key(&item), item))
        .collect();
    let mut merged = Vec::with_capacity(base.len().max(overlay_map.len()));

    for mut item in base {
        let key = match_key(&item);
        if let Some(fd) = overlay_map.remove(&key) {
            let should_merge = fd.score.home.is_some()
                || fd.score.away.is_some()
                || fd.status != "scheduled";
            if should_merge {
                if fd.score.home.is_some() || fd.score.away.is_some() {
                    item.score = fd.score;
                }
                if fd.status != "scheduled" {
                    item.status = fd.status;
                }
                if !fd.stage.is_empty() {
                    item.stage = fd.stage;
                }
                if !fd.venue.is_empty() {
                    item.venue = fd.venue;
                }
                if !fd.city.is_empty() {
                    item.city = fd.city;
                }
                if fd.updated_at.is_some() {
                    item.updated_at = fd.updated_at;
                }
            }
        }
        merged.push(item);
    }

    merged.extend(overlay_map.into_values());
    normalize_matches(merged)
}

fn match_key(item: &Match) -> String {
    format!("{}|{}|{}|{}", item.home_team_id, item.away_team_id, item.date, item.time)
}

fn score_count(matches: &[Match]) -> usize {
    matches
        .iter()
        .filter(|item| item.score.home.is_some() && item.score.away.is_some())
        .count()
}

fn beijing_datetime(date: &str, time: &str) -> Option<(String, String)> {
    let mut parts = time.split_whitespace();
    let clock = parts.next()?;
    let offset_text = parts.next()?.strip_prefix("UTC")?;
    let source_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let source_time = NaiveTime::parse_from_str(clock, "%H:%M").ok()?;
    let offset_hours = offset_text.parse::<i32>().ok()?;
    let source_offset = FixedOffset::east_opt(offset_hours * 3600)?;
    let beijing_offset = FixedOffset::east_opt(8 * 3600)?;
    let local = source_offset.from_local_datetime(&source_date.and_time(source_time)).single()?;
    let beijing = local.with_timezone(&beijing_offset);
    Some((beijing.format("%Y-%m-%d").to_string(), beijing.format("%H:%M").to_string()))
}

fn stage_label(round: &str) -> String {
    match round {
        value if value.starts_with("Matchday") => "小组赛",
        "Round of 32" => "32 强",
        "Round of 16" => "16 强",
        "Quarter-final" => "1/4 决赛",
        "Semi-final" => "半决赛",
        "Match for third place" => "季军赛",
        "Final" => "决赛",
        value => value,
    }
    .to_string()
}

fn lookup_team_id(team_ids: &HashMap<&'static str, &'static str>, name: &str) -> String {
    team_ids
        .get(name)
        .copied()
        .map(str::to_string)
        .unwrap_or_else(|| format!("slot-{}", name.to_ascii_lowercase().replace([' ', '/', '&'], "-")))
}

fn team_name_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("Mexico", "mex"),
        ("South Africa", "rsa"),
        ("South Korea", "kor"),
        ("Czech Republic", "cze"),
        ("Canada", "can"),
        ("Bosnia & Herzegovina", "bih"),
        ("Qatar", "qat"),
        ("Switzerland", "sui"),
        ("Brazil", "bra"),
        ("Morocco", "mar"),
        ("Haiti", "hai"),
        ("Scotland", "sco"),
        ("USA", "usa"),
        ("Paraguay", "par"),
        ("Australia", "aus"),
        ("Turkey", "tur"),
        ("Germany", "ger"),
        ("Curaçao", "cuw"),
        ("Ecuador", "ecu"),
        ("Ivory Coast", "civ"),
        ("Sweden", "swe"),
        ("Tunisia", "tun"),
        ("Netherlands", "ned"),
        ("Japan", "jpn"),
        ("Iran", "irn"),
        ("New Zealand", "nzl"),
        ("Belgium", "bel"),
        ("Egypt", "egy"),
        ("Saudi Arabia", "ksa"),
        ("Uruguay", "uru"),
        ("Cape Verde", "cpv"),
        ("Spain", "esp"),
        ("France", "fra"),
        ("Iraq", "irq"),
        ("Norway", "nor"),
        ("Senegal", "sen"),
        ("Algeria", "alg"),
        ("Argentina", "arg"),
        ("Austria", "aut"),
        ("Jordan", "jor"),
        ("DR Congo", "cod"),
        ("Colombia", "col"),
        ("Portugal", "por"),
        ("Uzbekistan", "uzb"),
        ("Croatia", "cro"),
        ("England", "eng"),
        ("Ghana", "gha"),
        ("Panama", "pan"),
    ])
}
