import aiohttp, re

STEAMID_XML_PATTERN = re.compile("<steamID64>(\\d+)</steamID64>")
VANITY_LINK_PATTERN = re.compile("(https://steamcommunity.com/id/([\\w-]+))")
PERM_LINK_PATTERN = re.compile("https://(?:steamcommunity.com/profiles|steamhistory.net/id|shadefall.net/daemon)/(\\d+)")
PERM_LINK_PREFIX = "https://steamcommunity.com/profiles/"
STEAMID_REGEX = re.compile("7656\\d{13}")
STEAMID3_REGEX = re.compile(r"\[U:1:(\d+)\]")

def sid3_to64(id: str) -> None | int:
    res = STEAMID3_REGEX.search(id)
    if not res:
        return None
    return int(res.group(1)) + 76561197960265728

async def resolve_vanity_url(url: str) -> int | None:
    async with aiohttp.ClientSession() as session:
        async with session.get(url + "?xml=1") as resp:
            steamid = STEAMID_XML_PATTERN.search(await resp.text())
            if not steamid:
                return None
            return int(steamid.group(1))