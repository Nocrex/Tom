from src.bot import bot

if __name__ == "__main__":
    token = open("token.txt").read().strip()
    bot.run(token, log_handler=None)
