# markov-bot

A discord chat and music bot written in Rust

## Deployment instructions

### Prerequisites

- Docker
- [Docker Compose](https://docs.docker.com/compose/install/#install-compose)
- Discord bot token
- Discord bot application ID

### Running the bot

Download the docker-compose.yml.

In the same folder as the docker-compose.yml create a `.env` file with the environment variables `DB_PASS`, `DISCORD_TOKEN` and `APPLICATION_ID`.

Example:

````env
DISCORD_TOKEN=aPc7yOzdaGg8gegTU2.uakxzW23dh6g4G46GAD6JBZs
APPLICATION_ID=973467367436746574
DB_PASS=examplepassword
````

Open the folder in a terminal.

Run the bot with the following command:

```shell
docker compose up --no-build
```
