[![](https://docs.rs/markov_strings/badge.svg)](https://docs.rs/markov_strings)

# markov_strings

A simplistic, configurable, Markov chain text generator. Give it an array of strings, and it will output a randomly generated string.

<sup><i>This is a port of the npm library [markov-strings](https://github.com/scambier/markov-strings), from the same author.</i></sup>

## Usage

```rust
use markov_strings::*;

// Instantiate the generator
let mut markov = Markov::new();

// Optional: specify a state size
markov.set_state_size(3); // Default: 2

// Feed it data
let data: Vec<InputData> = vec![/* a lot of data */];
markov.add_to_corpus(data);

// Define a results filter
markov
    .set_filter(|r| {
        // A minimal relative score and number of references
        // The thresholds are relative to your input
        r.score > 5 && r.refs.len() > 2
            // We want to generate random tweets
            && r.text.len() <= 280
            // No mentions
            && !r.text.contains("@")
            // No urls
            && !r.text.contains("http")
    })
    .set_max_tries(100);

let result: MarkovResult = markov.generate().unwrap();
```

## Repository

The code is hosted on [this Fossil repository](https://fossil.scambier.xyz/markov-strings-rust).

To clone this project, install [Fossil SCM](https://fossil-scm.org/home/doc/trunk/www/quickstart.wiki), then:

```text
$ mkdir markov_strings
$ cd markov_strings
$ fossil clone https://fossil.scambier.xyz/markovStringsRust markov_strings.fossil
$ fossil open markov_strings.fossil
```
