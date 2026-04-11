use anyhow::Result;

use crate::Context;

pub(super) fn register() -> Vec<poise::Command<crate::BotData, anyhow::Error>> {
    vec![export()]
}

#[poise::command(slash_command, owners_only)]
pub(super) async fn export(ctx: Context<'_>) -> Result<()> {
    ctx.defer_ephemeral().await?;
    let now = std::time::Instant::now();
    crate::reports::exports::export(ctx.data().reports.clone(), &ctx.data().config.report.export)
        .await?;

    ctx.say(format!("Done {}s", now.elapsed().as_secs_f32()))
        .await?;
    Ok(())
}
