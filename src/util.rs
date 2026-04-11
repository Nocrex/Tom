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
        Regex::new("https://steamcommunity.com/id/[\\w-]+)").unwrap();
    static ref STEAMID_XML_PATTERN: Regex = Regex::new("<steamID64>(\\d+)</steamID64>").unwrap();
}

const PERM_LINK_PREFIX: &str = "https://steamcommunity.com/profiles/";

pub(crate) async fn get_steamid(steamid: &str) -> Result<SteamID> {
    if let Ok(sid) = SteamID::from_str(steamid) {
        Ok(sid)
    } else if let Some(cap) = PERM_LINK_PATTERN.captures(steamid) {
        Ok(SteamID::from_str(cap.get(1).unwrap().as_str())?)
    } else if let Some(mat) = VANITY_LINK_PATTERN.find(steamid) {
        let url = mat.as_str();
        resolve_vanity(url).await
    } else {
        anyhow::bail!("steamid did not match")
    }
}

pub(crate) async fn resolve_vanity(url: &str) -> Result<SteamID> {
    let profile_info = reqwest::get(format!("{url}?xml=1")).await?.text().await?;
    STEAMID_XML_PATTERN
        .find(&profile_info)
        .ok_or(anyhow::anyhow!(
            "Could not find steam id in server response"
        ))
        .and_then(|id| Ok(SteamID::from_str(id.as_str())?))
}

pub(crate) trait SteamIDProfileLink {
    fn profile(&self) -> String;
}

impl SteamIDProfileLink for SteamID {
    fn profile(&self) -> String {
        format!("{PERM_LINK_PREFIX}{}", self.steam64())
    }
}

pub(crate) fn load_lists(dir: &str) -> Result<HashMap<SteamID, Vec<String>>> {
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

    Ok(lists)
}
