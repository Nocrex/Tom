pub(crate) mod users;
pub(crate) mod officers;

mod checks {
    use anyhow::Result;
    use poise::CreateReply;
    use crate::Context;
    pub(super) async fn is_officer(ctx: Context<'_>) -> Result<bool> {
        Ok(ctx.data().config.report.officer_roles.contains(&u64::from(ctx.author().id)))
    }
    
    pub(super) async fn in_thread(ctx: Context<'_>) -> Result<bool> {
        if !ctx.guild_channel().await.is_some_and(|c|c.thread_metadata.is_some()) {
            ctx.send(CreateReply::default().content("Cannot use this command outside of a thread").ephemeral(true)).await?;
            return Ok(false);
        }
        Ok(true)
    }
}