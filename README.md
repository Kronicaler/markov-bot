# markov-bot

A discord chat and music bot written in Rust

## Deployment instructions

### Prerequisites

- Docker
- [Docker Compose](https://docs.docker.com/compose/install/#install-compose)
- Discord bot token
- Discord bot application ID

### Running the bot

$$
x := \{V, T\} \quad \text{where V(value)} \in \mathbb{Q} \text{ and T(timestamp) } \in \mathbb{N_0}
$$

$$
F(factor) \in \mathbb{Q}, \text{  } O(offset) \in \mathbb{Z}
$$

$$
f(x) = 
\begin{cases} 
x.V \times \text{F}_1 + \text{O}_1 & \text{if } x.T \in [T_{1a}, T_{1b}) 
\\
x.V \times \text{F}_2 + \text{O}_2 & \text{if } x.T \in [T_{2a}, T_{2b})
\\
\vdots
\\
x.V \times \text{F}_{n-1} + \text{O}_{n-1} & \text{if } x.T \in [T_{(n-1)a}, T_{(n-1)b})
\\
x.V \times \text{F}_n + \text{O}_n & \begin{cases} \text{if } x.T \in [T_{na}, T_{nb}) & \text{(when } T_{na} \text{ and } T_{nb} \text{ are defined)}
\\
\text{if } x.T \geq T_{na} & \text{(when } T_{nb} \text{ is not defined)} \end{cases}
\\
x.V & \text{otherwise}
\end{cases}
$$

$$
\text{Where:} \quad \text{T}_{i\text{a}} < \text{T}_{i\text{b}} \leq \text{T}_{(i+1)\text{a}} \text{ for all } i \in \mathbb{N}
$$

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
