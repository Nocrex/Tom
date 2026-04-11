use anyhow::Result;
use poise::serenity_prelude::UserId;
pub(crate) mod exports;

#[async_trait::async_trait]
pub trait ReportDB {
    async fn add_report(&mut self, report: AddReport) -> Result<()>;
    async fn remove_report(&mut self, thread_url: String) -> Result<()>;

    async fn report(&self, thread_url: &str) -> Result<Option<Report>>;

    async fn find_reports(&self, steamid: &steamid_ng::SteamID) -> Result<Vec<PlayerReport>>;
    async fn all_reports(&self) -> Result<Vec<PlayerReport>>;

    async fn reporter(&self, user_id: UserId) -> Result<Option<Reporter>>;
    async fn reporter_with_points(
        &self,
        user_id: UserId,
        fetch_reports: bool,
    ) -> Result<Option<(Reporter, u32)>>;

    async fn reporters_with_points(&self) -> Result<Vec<(Reporter, u32)>>;
}

#[derive(Debug, Clone)]
enum PlayerAttribute {
    CHEATER,
    EXPLOITER,
}

impl std::fmt::Display for PlayerAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CHEATER => f.write_str("cheater"),
            Self::EXPLOITER => f.write_str("exploiter"),
        }
    }
}

struct ReportedPlayer {
    steamid: steamid_ng::SteamID,
    last_seen: chrono::DateTime<chrono::Utc>,
    attribute: PlayerAttribute,
    verified: bool,
}

type ReportId = u64;

pub struct Report {
    pub id: ReportId,
    pub players: Vec<ReportedPlayer>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub reporter_id: UserId,
    pub points: u8,
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
    pub id: ReportId,
    pub steamid: steamid_ng::SteamID,
    pub report_timestamp: chrono::DateTime<chrono::Utc>,
    pub reporter_id: UserId,
    pub thread_url: String,

    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub attribute: PlayerAttribute,
    pub verified: bool,
}
