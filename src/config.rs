use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub error_channel: u64,

    pub report: ReportConfig,
    pub react: ReactConfig,
    pub vanity: VanityConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ReportConfig {
    pub approve_image: String,
    pub log_channel: u64,
    pub officer_roles: Vec<u64>,
    pub confirmed_tag: u64,

    pub ext_list_dir: String,
    pub export: ExportConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ExportConfig {
    pub id_list_filename: String,
    pub tfbd_list_filename: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VanityConfig {
    pub resolve_channels: Vec<u64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ReactConfig {
    pub image_channel: u64,
    pub cooldown_seconds: u64,
}
