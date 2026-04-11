use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = super::schema::reporters)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Reporter {
    pub id: i64,
    pub steamid: Option<i64>,
}

impl Into<super::super::Reporter> for Reporter {
    fn into(self) -> super::super::Reporter {
        super::super::Reporter {
            id: poise::serenity_prelude::UserId::from(self.id as u64),
            steamid: self.steamid.map(|id|steamid_ng::SteamID::from_steam64(id as u64).unwrap()),
            reports: vec![],
        }
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = super::schema::reports)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Report {
    pub id: i32,
    pub reporter: i64,
    pub time: chrono::NaiveDateTime,
    pub points: i16,
    pub threadurl: String,
    pub message: String,
}

impl Into<super::super::Report> for Report {
    fn into(self) -> super::super::Report {
        super::super::Report {
            timestamp: self.time.and_utc(),
            reporter_id: poise::serenity_prelude::UserId::from(self.reporter as u64),
            points: self.points as u8,
            thread_url: self.threadurl,
            message: self.message,
            players: vec![],
        }
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = super::schema::playerreports)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ReportedPlayer {
    pub report: i32,
    pub steamid: i64,
    pub last_seen: chrono::NaiveDateTime,
    pub attribute: i16, 
    pub verified: bool,
}

impl Into<super::super::ReportedPlayer> for ReportedPlayer {
    fn into(self) -> super::super::ReportedPlayer {
        super::super::ReportedPlayer {
            steamid: steamid_ng::SteamID::from_steam64(self.steamid as u64).unwrap(),
            last_seen: self.last_seen.and_utc(),
            attribute: super::super::PlayerAttribute::try_from(self.attribute as u8).unwrap(),
            verified: self.verified,
        }
    }
}