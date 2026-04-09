import discord
import logging
from discord.ext import commands
from .hp_cog import HPCog
from .. import statics

from ..steam import resolve_vanity_url, VANITY_LINK_PATTERN, PERM_LINK_PATTERN

logger = logging.getLogger(__name__)

# Extra cog that watches channels for steam profile vanity links and attempts to find the perma link for them
class VanityCog(commands.Cog):
    def __init__(self, bot: commands.Bot):
        self.hp_cog = bot.get_cog("HPCog")
        if self.hp_cog is None:
            raise RuntimeError("Couldn't get HPCog")
        assert isinstance(self.hp_cog, HPCog)
        self.bot: commands.Bot = bot

    @commands.Cog.listener()
    async def on_message(self, message: discord.Message):
        if message.author.bot:
            return
        channel_id = message.channel.parent_id if isinstance(message.channel, discord.Thread) else message.channel.id
        if channel_id not in statics.VANITY_RESOLVER_CHANNELS:
            return

        matches = VANITY_LINK_PATTERN.findall(message.content)
        steamids = dict()
        unresolved_steamids = []

        for match in set(matches):
            res = await resolve_vanity_url(match[0])
            if res:
                steamids[match[1]] = str(res)
            else:
                unresolved_steamids.append(match[1])
                    
        matches = PERM_LINK_PATTERN.findall(message.content)
        reported_perms = dict()
        list_matches = dict()
        mentioned_ids = set(matches) | set(steamids.values())
        for sid in mentioned_ids:
            sid = int(sid)
            reports = self.hp_cog.reports.find_reported(sid)
            if len(reports) > 0:
                verified = any(map(lambda r: r.verified, reports))
                if verified:
                    reported_perms[sid] = {"report": next(filter(lambda r: r.verified, reports)).thread_url, "verified": True}
                else:
                    reported_perms[sid] = {"report": reports[0].thread_url, "verified": False}
            
            lists = self.hp_cog.reports.check_external_lists(sid)
            if len(lists) > 0:
                list_matches[sid] = lists

        # only reply if there were steamids found
        if len(steamids) > 0 or len(unresolved_steamids) > 0 or len(reported_perms) > 0 or len(list_matches) > 0:
            embed = discord.Embed()
            if len(steamids) > 0:
                embed.add_field(inline=False, name="Permanent links", value=
                    "\n".join(map(lambda sid: f'"{sid[0]}": {PERM_LINK_PREFIX+sid[1]}', steamids.items())) + "\n")
            if len(unresolved_steamids) > 0:
                embed.add_field(inline=False, name="", value=
                    "Could not find profile for " + ", ".join(map(lambda vid: f'"{vid}"', unresolved_steamids)))
            if len(reported_perms) > 0:
                embed.add_field(inline=False, name="Reports", value=
                    "\n".join(map(lambda s: f"`{s[0]}` -> {s[1]['report']}{' (unverified)' if not s[1]['verified'] else ''}", reported_perms.items())))
            elif len(steamids) > 0:
                embed.add_field(inline=False, name="SteamIDs have not been reported", value="")
            
            if len(list_matches) > 0:
                embed.add_field(inline=False, name="Players present in lists", value=
                "\n".join(map(lambda s: f"`{s[0]}` -> {', '.join(s[1])}", list_matches.items())))

            if len(reported_perms) > 0:
                color = discord.Color.orange()
            elif len(list_matches) > 0:
                color = discord.Color.yellow()
            else:
                color = discord.Color.blue()
            embed.color = color
            await message.reply(embed=embed, mention_author=False)

    
async def setup(bot: commands.Bot):
    await bot.add_cog(VanityCog(bot))
