# markov-bot

A discord chat and music bot written in Rust

# Deployment instructions

In the same folder as the executable you'll need to create a .env file with the environment variables DISCORD_TOKEN and APPLICATION_ID.
### Example: 
````
DISCORD_TOKEN=OPc7yOsdaGAEgegTU2.GakxzW23dh6g4G46GADKJBZs
APPLICATION_ID=973467367436746574
````

## Dependencies

The bot _will_ work without these dependencies but it's music functionality won't work.

* youtube-dl
* Opus
* FFmpeg

### Linux instructions for installing the dependencies:

* youtube-dl - ``apt install youtube-dl`` on Ubuntu or ``pacman -S youtube-dl`` on Arch Linux.
* Opus - ``apt install libopus-dev`` on Ubuntu or ``pacman -S opus`` on Arch Linux
* FFmpeg - ``apt install ffmpeg`` on Ubuntu or ``pacman -S ffmpeg`` on Arch Linux

### Windows instructions for installing the dependencies:

* youtube-dl - Download from [here](http://ytdl-org.github.io/youtube-dl/download.html)
* Opus - A prebuilt DLL is provided for you, you do not have to do anything.
* FFmpeg - Download from [here](https://ffmpeg.org/download.html) and follow [these instructions](https://www.wikihow.com/Install-FFmpeg-on-Windows). You can test if it works by opening up the cmd and typing in ``ffmpeg``.

##

After you've installed the dependencies and created a .env file with your discord token and application id you can run the bot.
