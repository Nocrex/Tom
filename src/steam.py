import asyncio
import re
import logging
import aiohttp

log = logging.getLogger(__name__)

VANITY_LINK_PATTERN = re.compile("(https://steamcommunity.com/id/([\\w-]+))")
PERM_LINK_PATTERN = re.compile(
    "https://(?:steamcommunity.com/profiles|steamhistory.net/id|shadefall.net/archive)/(\\d+)"
)
PERM_LINK_PREFIX = "https://steamcommunity.com/profiles/"
STEAMID_REGEX = re.compile("7656\\d{13}")
STEAMID3_REGEX = re.compile(r"\[U:1:(\d+)\]")


def sid3_to64(id: str) -> None | int:
    res = STEAMID3_REGEX.search(id)
    if not res:
        return None
    return int(res.group(1)) + 76561197960265728


steam_token = open("steamtoken.txt").read().strip()

async def resolve_vanity_url(url: str) -> int | None:
    vanity = VANITY_LINK_PATTERN.match(url)
    if vanity is None:
        return None
    vanity = vanity.group(2)
    async with aiohttp.ClientSession() as session:
        async with session.get(f"https://api.steampowered.com/ISteamUser/ResolveVanityURL/v1/?vanityurl={vanity}&key={steam_token}") as resp:
            js = await resp.json()
            status = js["response"]["success"]
            if status == 1:
                return int(js["response"]["steamid"])
            if status == 42:
                return None
            log.error(f"Failed to resolve profile '{url}', response: \n{js}")
            return None
