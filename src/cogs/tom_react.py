import logging
import random
import time

import discord
from discord.ext import commands

from .. import statics

logger = logging.getLogger(__name__)


class ReactCog(commands.Cog):
    def __init__(self, bot: commands.Bot):
        self.bot: commands.Bot = bot
        self.cooldowns: dict[int, int] = dict()
        self.images: list[str] = []

    async def cog_load(self):
        self.img_channel: discord.TextChannel = await self.bot.fetch_channel(
            statics.Images.TOM_REACTS_CHANNEL
        )
        await self.load_imgs()

    async def load_imgs(self):
        async for message in self.img_channel.history(limit=None):
            for a in message.attachments:
                self.images.append(a.url)
        logger.info(f"Loaded {len(self.images)} images")

    @commands.Cog.listener()
    async def on_message(self, message: discord.Message):
        if message.channel.id == statics.Images.TOM_REACTS_CHANNEL:
            for a in message.attachments:
                self.images.append(a.url)
            logger.info(f"Added {len(message.attachments)} images")
            return

        if not self.bot.user.mention in message.content:
            return

        curtime = time.time()
        cooldowns = {i: ts for i, ts in self.cooldowns.items() if ts > curtime}
        self.cooldowns = cooldowns

        if message.author.id in self.cooldowns or len(self.images) < 1:
            await message.add_reaction("\N{SLEEPING SYMBOL}")
            return

        self.cooldowns[message.author.id] = (
            time.time() + statics.TOM_REACT_INTERVAL_SECONDS
        )
        m = await message.channel.send(random.choice(self.images))
        await m.add_reaction("\u2764")


async def setup(bot: commands.Bot):
    await bot.add_cog(ReactCog(bot))
