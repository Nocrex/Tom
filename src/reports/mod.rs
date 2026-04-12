use anyhow::Result;
use poise::serenity_prelude::UserId;
pub mod exports;
pub mod sql;

#[async_trait::async_trait]
pub trait ReportDB {
    async fn find_reports(&self, steamid: &steamid_ng::SteamID) -> Result<Vec<PlayerReport>>;
    async fn all_reports(&self) -> Result<Vec<PlayerReport>>;
    async fn reported_count(&self) -> Result<u64>;

    async fn report(&self, url: &str, fetch_players: bool) -> Result<Option<Report>>;
    async fn reporter(&self, user_id: UserId) -> Result<Option<Reporter>>;
    async fn reporter_with_points(
        &self,
        user_id: UserId,
        fetch_reports: bool,
    ) -> Result<Option<(Reporter, i32)>>;

    async fn reporters_with_points(&self) -> Result<Vec<(Reporter, i32)>>;
}

#[repr(u8)]
#[derive(Debug, Clone, int_enum::IntEnum)]
pub enum PlayerAttribute {
    CHEATER = 0,
    EXPLOITER = 1,
}

impl std::fmt::Display for PlayerAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CHEATER => f.write_str("cheater"),
            Self::EXPLOITER => f.write_str("exploiter"),
        }
    }
}

pub struct ReportedPlayer {
    steamid: steamid_ng::SteamID,
    last_seen: chrono::DateTime<chrono::Utc>,
    attribute: PlayerAttribute,
    verified: bool,
}

pub struct Report {
    pub players: Vec<ReportedPlayer>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub reporter_id: UserId,
    pub points: i16,
    pub thread_url: String,
    pub message: String,
}

pub struct Reporter {
    pub id: UserId,
    pub steamid: Option<steamid_ng::SteamID>,
    pub reports: Vec<Report>,
}

struct AddReport {
    players: Vec<ReportedPlayer>,
    reporter_id: UserId,
    points: u8,
    thread_url: String,
    message: String,
}

#[derive(Clone)]
pub struct PlayerReport {
    pub steamid: steamid_ng::SteamID,
    pub report_timestamp: chrono::DateTime<chrono::Utc>,
    pub reporter_id: UserId,
    pub thread_url: String,

    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub attribute: PlayerAttribute,
    pub verified: bool,
}

impl From<(Report, ReportedPlayer)> for PlayerReport {
    fn from((report, player): (Report, ReportedPlayer)) -> Self {
        Self {
            steamid: player.steamid,
            report_timestamp: report.timestamp,
            reporter_id: report.reporter_id,
            thread_url: report.thread_url,
            last_seen: player.last_seen,
            attribute: player.attribute,
            verified: player.verified,
        }
    }
}