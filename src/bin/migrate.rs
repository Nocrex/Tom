use std::{collections::HashMap, env::args};

use diesel::{Connection, ExpressionMethods, PgConnection, RunQueryDsl, insert_into};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct JsonReporter {
    count: i64,
    profile_id: Option<i64>,
    reports: Vec<JsonReport>,
}

#[derive(Serialize, Deserialize)]
struct JsonReport {
    date: String,
    msg: String,
    points: i64,
    verified: bool,
    players: HashMap<String, String>,
}

fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let mut args = args();
    args.next();
    
    let mut con = PgConnection::establish(&dotenv::var("DATABASE_URL").unwrap()).unwrap();

    let data: HashMap<String, JsonReporter> =
        serde_json::from_str(&std::fs::read_to_string(args.next().unwrap()).unwrap()).unwrap();
    
    let reporter_count = data.len();

    for (i, (reporter_id, reporter_data)) in data.into_iter().enumerate() {
        println!("Migrating reporter {}/{}", i+1, reporter_count);
        let id = reporter_id.parse::<i64>().unwrap();
        let steamid = reporter_data.profile_id;

        insert_into(reporters::table)
            .values((reporters::id.eq(id), reporters::steamid.eq(steamid)))
            .on_conflict_do_nothing()
            .execute(&mut con)?;

        for mut report in reporter_data.reports {
            let time = chrono::DateTime::parse_from_rfc3339(&report.date)
                .unwrap()
                .to_utc();
            
            if report.points < 0 {
                report.points = 1;
                println!("Reporter {id} has a negative report count");
            }

            let report_id: i32 = insert_into(reports::table)
                .values((
                    reports::reporter.eq(id),
                    reports::time.eq(time.naive_utc()),
                    reports::points.eq(report.points as i16),
                    reports::threadurl.eq(&report.msg[0..report.msg.find(" ").unwrap_or(report.msg.len())]),
                    reports::message.eq(&report.msg),
                ))
                .on_conflict_do_nothing()
                .returning(reports::id)
                .get_result(&mut con)?;

            for (steamid, kind) in report.players {
                let steamid = steamid.parse::<i64>().unwrap();
                let kind = match kind.as_str() {
                    "cheater" => 0,
                    "exploiter" => 1,
                    _ => panic!(),
                };

                insert_into(playerreports::table)
                    .values((
                        playerreports::report.eq(report_id),
                        playerreports::steamid.eq(steamid),
                        playerreports::last_seen.eq(time.naive_utc()),
                        playerreports::attribute.eq(kind),
                        playerreports::verified.eq(report.verified),
                    ))
                    .on_conflict_do_nothing()
                    .execute(&mut con)?;
            }
        }
    }
    Ok(())
}

diesel::table! {
    playerreports (report, steamid) {
        report -> Int4,
        steamid -> Int8,
        last_seen -> Timestamp,
        attribute -> Int2,
        verified -> Bool,
    }
}

diesel::table! {
    reporters (id) {
        id -> Int8,
        steamid -> Nullable<Int8>,
    }
}

diesel::table! {
    reports (id) {
        id -> Int4,
        reporter -> Int8,
        time -> Timestamp,
        points -> Int2,
        threadurl -> Text,
        message -> Text,
    }
}

diesel::joinable!(playerreports -> reports (report));
diesel::joinable!(reports -> reporters (reporter));

diesel::allow_tables_to_appear_in_same_query!(playerreports, reporters, reports,);
