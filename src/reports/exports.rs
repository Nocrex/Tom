use std::{collections::HashSet, sync::Arc};

use crate::{config::ExportConfig, reports::{PlayerReport, ReportDB}};
use anyhow::Result;
use itertools::Itertools;
use tokio::fs;
use serde_json::json;

pub(crate) async fn export(db: Arc<dyn ReportDB + Send + Sync>, cfg: &ExportConfig) -> Result<()> {
    let reports = db.all_reports().await?;
    simple_export(&reports, cfg).await?;
    tf2bd_export(&reports, cfg).await?;
    Ok(())
}

async fn simple_export(reports: &Vec<PlayerReport>, cfg: &ExportConfig) -> Result<()> {
    let mut content = reports.iter().filter(|r|r.verified).map(|r|r.steamid.steam64().to_string()).collect::<Vec<_>>();
    content.sort();
    content.dedup();
    fs::write(&cfg.id_list_filename, content.into_iter().join("\n").as_bytes()).await?;
    Ok(())
}

async fn tf2bd_export(reports: &Vec<PlayerReport>, cfg: &ExportConfig) -> Result<()> {
    
    let players = reports.iter().filter(|r|r.verified).into_group_map_by(|r|r.steamid).into_iter().map(|(id, reps)|{
        let steamid = id.steam3();
        let attrs = reps.iter().map(|r|r.attribute.to_string()).collect::<HashSet<_>>();
        let proof = reps.iter().map(|r|&r.thread_url).collect::<Vec<_>>();
        let last_seen = reps.iter().map(|r|r.last_seen).max().unwrap().timestamp();
        
        json!({
            "attributes": attrs,
            "steamid": steamid,
            "proof": proof,
            "last_seen": {
                "time": last_seen
            }
        })
    }).collect::<Vec<_>>();
    
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let title = format!("vorobey-hackerpolice - {now}");
    let description = format!("List of cheaters reported in the hackerpolice channel on the Vorobey discord server, last updated {now}");
    
    let content = json!({
        "$schema": "https://raw.githubusercontent.com/PazerOP/tf2_bot_detector/master/schemas/v3/playerlist.schema.json",
        "file_info": {
            "authors": [
                "All contributors in the hackerpolice channel"
            ],
            "description": description,
            "title": title,
            "update_url": "https://raw.githubusercontent.com/Nocrex/Tom/refs/heads/main/playerlist.vorobey-hackerpolice.json"
        },
        "players": players,
    });
    
    fs::write(&cfg.tfbd_list_filename, &serde_json::to_vec_pretty(&content)?).await?;
    
    Ok(())
}