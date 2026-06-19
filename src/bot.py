import discord, logging, os, sys
import discord.ext.commands
from io import StringIO
import traceback
from . import statics

intents = discord.Intents.default()
intents.message_content = True
bot = discord.ext.commands.Bot(command_prefix='idkhowtodisablethissoilljustputsomethingunlikelyhere', intents=intents, help_command=None)

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)
logging.basicConfig(level=logging.INFO, format="{levelname:8s} | {asctime} | {name:15s} | {message}", style="{")

error_channel = None

cogs = [
    "hp_cog",
    "vanity_resolver_cog",
    "tom_react"
]

@bot.event
async def on_ready():
    global error_channel
    if error_channel:
        return
    error_channel = await bot.fetch_channel(statics.ERROR_CHANNEL_ID)
    for cog in cogs:
        await bot.load_extension(f"src.cogs.{cog}")
    await bot.tree.sync() # upload command tree to discord, so you can see all available commands in the client
    logger.info(f"{bot.user} is up and running meow")

@bot.event
async def on_error(event, *args, **kwargs):
    error = sys.exception()
    msg = "".join(traceback.format_exception(error))
    sio = StringIO(msg)
    await error_channel.send(file=discord.File(sio, filename="error.txt"))
    logger.error(msg)

if os.environ.get("DEBUG") == "1": # only enable command if debug is set in environment
    @bot.tree.command()
    async def reload(interaction: discord.Interaction):
        await interaction.response.defer(ephemeral=True)
        for cog in cogs:
            await bot.reload_extension(f"src.cogs.{cog}")
        await bot.tree.sync()
        await interaction.followup.send("Reloaded!", ephemeral=True)


