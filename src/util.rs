use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use std::{collections::HashMap, str::FromStr};
use steamid_ng::SteamID;

lazy_static! {
    static ref STEAMID_REGEX: Regex = Regex::new("7656\\d{13}").unwrap();
    static ref STEAMID3_REGEX: Regex = Regex::new(r"\[U:1:d+\]").unwrap();
    pub static ref PERM_LINK_PATTERN: Regex = Regex::new(
        "https://(?:steamcommunity.com/profiles|steamhistory.net/id|shadefall.net/daemon)/(\\d+)"
    )
    .unwrap();
    pub static ref VANITY_LINK_PATTERN: Regex =
        Regex::new("https://steamcommunity.com/id/([\\w-]+)").unwrap();
    static ref STEAMID_XML_PATTERN: Regex = Regex::new("<steamID64>(\\d+)</steamID64>").unwrap();
}

const PERM_LINK_PREFIX: &str = "https://steamcommunity.com/profiles/";

pub async fn get_steamid(steamid: &str) -> Result<Option<SteamID>> {
    if let Ok(sid) = SteamID::from_str(steamid) {
        Ok(Some(sid))
    } else if let Some(cap) = PERM_LINK_PATTERN.captures(steamid) {
        Ok(Some(SteamID::from_str(cap.get(1).unwrap().as_str())?))
    } else if let Some(mat) = VANITY_LINK_PATTERN.find(steamid) {
        let url = mat.as_str();
        resolve_vanity(url).await
    } else {
        anyhow::bail!("steamid did not match")
    }
}

pub async fn resolve_vanity(url: &str) -> Result<Option<SteamID>> {
    let response = reqwest::get(format!("{url}?xml=1")).await?;
    if !response.status().is_success() {
        anyhow::bail!(
            "Vanity resolve query replied with status {}",
            response.status()
        );
    }
    let profile_info = response.text().await?;
    let Some(id_str) = STEAMID_XML_PATTERN
        .captures(&profile_info)
        .map(|c| c.get(1).unwrap().as_str().to_string())
    else {
        return Ok(None);
    };

    Ok(Some(SteamID::from_str(&id_str)?))
}

pub trait SteamIDProfileLink {
    fn profile(&self) -> String;
}

impl SteamIDProfileLink for SteamID {
    fn profile(&self) -> String {
        format!("{PERM_LINK_PREFIX}{}", self.steam64())
    }
}

pub fn load_lists(dir: &str) -> Result<HashMap<SteamID, Vec<String>>> {
    let mut lists: HashMap<SteamID, Vec<String>> = HashMap::new();
    let dir = std::fs::read_dir(dir)?;
    for entry in dir {
        let list = entry?;
        let path = list.path();
        let list_name = path.file_stem().unwrap().to_str().unwrap();
        let list_content = std::fs::read_to_string(list.path())?;

        STEAMID_REGEX
            .find_iter(&list_content)
            .map(|mat| SteamID::from_steam64(mat.as_str().parse::<u64>().unwrap()).unwrap())
            .chain(
                STEAMID3_REGEX
                    .find_iter(&list_content)
                    .map(|mat| SteamID::from_steam3(mat.as_str()).unwrap()),
            )
            .for_each(|id| lists.entry(id).or_default().push(list_name.to_owned()));
    }

    log::info!("Loaded {} external list entries", lists.len());
    Ok(lists)
}

pub trait GetJumpUrl {
    fn jump_url(&self) -> String;
}

impl GetJumpUrl for poise::serenity_prelude::GuildChannel {
    fn jump_url(&self) -> String {
        format!("https://discord.com/channels/{}/{}", self.guild_id, self.id)
    }
}