use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use diesel::{connection::DefaultLoadingMode, prelude::*};
use tokio::sync::Mutex;

use crate::reports::*;

mod models;
mod schema;

pub(crate) struct PostgresDB {
    connection: Arc<Mutex<PgConnection>>,
}

impl PostgresDB {
    pub async fn new(url: &str) -> Result<Self> {
        log::info!("Connecting to postgres");
        let connection = Arc::new(Mutex::new(PgConnection::establish(url)?));

        Ok(Self { connection })
    }
}

#[async_trait]
impl ReportDB for PostgresDB {
    async fn find_reports(&self, steamid: &steamid_ng::SteamID) -> Result<Vec<PlayerReport>> {
        let conn = self.connection.clone();
        let id = steamid.steam64() as i64;

        let t = tokio::task::spawn_blocking(move || -> Result<Vec<PlayerReport>> {
            let mut db = conn.blocking_lock();

            let mut reports = vec![];

            for row in schema::reports::table
                .inner_join(schema::playerreports::table)
                .filter(schema::playerreports::steamid.eq(id))
                .select((
                    models::Report::as_select(),
                    models::ReportedPlayer::as_select(),
                ))
                .load_iter::<(models::Report, models::ReportedPlayer), DefaultLoadingMode>(
                    &mut *db,
                )?
            {
                let (report, player) = row?;
                let rep: super::Report = report.into();
                let pl: super::ReportedPlayer = player.into();

                reports.push(super::PlayerReport::from((rep, pl)));
            }

            Ok(reports)
        });

        t.await?
    }

    async fn all_reports(&self) -> Result<Vec<PlayerReport>> {
        let conn = self.connection.clone();

        let t = tokio::task::spawn_blocking(move || -> Result<Vec<PlayerReport>> {
            let mut db = conn.blocking_lock();

            let mut reports = vec![];

            for row in schema::reports::table
                .inner_join(schema::playerreports::table)
                .select((
                    models::Report::as_select(),
                    models::ReportedPlayer::as_select(),
                ))
                .load_iter::<(models::Report, models::ReportedPlayer), DefaultLoadingMode>(
                    &mut *db,
                )?
            {
                let (report, player) = row?;
                let rep: super::Report = report.into();
                let pl: super::ReportedPlayer = player.into();

                reports.push(super::PlayerReport::from((rep, pl)));
            }

            Ok(reports)
        });

        t.await?
    }

    async fn reported_count(&self) -> Result<u64> {
        let conn = self.connection.clone();

        let t = tokio::task::spawn_blocking(move || -> Result<u64> {
            let mut db = conn.blocking_lock();

            Ok(schema::playerreports::table
                .select(diesel::dsl::count(schema::playerreports::steamid).aggregate_distinct())
                .get_result::<i64>(&mut *db)? as u64)
        });

        t.await?
    }

    async fn reporter(&self, user_id: UserId) -> Result<Option<Reporter>> {
        let conn = self.connection.clone();

        let t = tokio::task::spawn_blocking(move || {
            let mut db = conn.blocking_lock();
            let rep = schema::reporters::table
                .select(models::Reporter::as_returning())
                .find(u64::from(user_id) as i64)
                .first(&mut *db)
                .optional()?;

            Ok(rep.map(|r| r.into()))
        });

        t.await?
    }

    async fn reporter_with_points(
        &self,
        user_id: UserId,
        fetch_reports: bool,
    ) -> Result<Option<(Reporter, i32)>> {
        let conn = self.connection.clone();

        let t = tokio::task::spawn_blocking(move || {
            let mut db = conn.blocking_lock();

            let reporter: Option<(models::Reporter, Option<i64>)> = schema::reporters::table
                .find(u64::from(user_id) as i64)
                .inner_join(schema::reports::table)
                .group_by(schema::reporters::id)
                .select((
                    schema::reporters::all_columns,
                    diesel::dsl::sum(schema::reports::points),
                ))
                .first(&mut *db)
                .optional()?;

            let mut reporter =
                reporter.and_then(|(r, c)| c.map(|c| (Into::<Reporter>::into(r), c as i32)));
            if fetch_reports && let Some(reporter) = reporter.as_mut() {
                for row in schema::reports::table
                    .select(models::Report::as_select())
                    .filter(schema::reports::reporter.eq(u64::from(user_id) as i64))
                    .load_iter::<models::Report, DefaultLoadingMode>(&mut *db)?
                {
                    let report: models::Report = row?;

                    reporter.0.reports.push(report.into());
                }
            }
            Ok(reporter)
        });

        t.await?
    }

    async fn reporters_with_points(&self) -> Result<Vec<(Reporter, i32)>> {
        let conn = self.connection.clone();

        let t = tokio::task::spawn_blocking(move || {
            let mut db = conn.blocking_lock();

            let reporters: Vec<(models::Reporter, Option<i64>)> = schema::reporters::table
                .inner_join(schema::reports::table)
                .group_by(schema::reporters::id)
                .select((
                    schema::reporters::all_columns,
                    diesel::dsl::sum(schema::reports::points),
                ))
                .load(&mut *db)?;

            let reporters: Vec<_> = reporters
                .into_iter()
                .filter_map(|(rep, c)| c.map(|c| (Into::<Reporter>::into(rep), c as i32)))
                .collect();

            Ok(reporters)
        });

        t.await?
    }
}
