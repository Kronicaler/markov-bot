#![warn(missing_docs)]

//! A simplistic & configurable Markov chain text generator
//!
//! Give it a vec of strings and generate random results.
//! Works best with tweets, chat history, news headlines...
//!
//! Minimal example:
//! ```no_run
//! use markov_strings::*;
//!
//! let data: Vec<InputData> = vec![/* a lot of data */];
//! let mut markov = Markov::new();
//! markov.add_to_corpus(data);
//! let result: MarkovResult = markov.generate().unwrap();
//! ```

use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// The input struct to build the markov-strings corpus.
///
/// ```rust
/// # use markov_strings::*;
/// let data = vec![InputData {
///     text: "foo bar".to_string(),
///     meta: Some("serialized value".to_string())
/// }];
/// ```
///
/// Implements `impl From<String>` so you can do
/// ```rust
/// # use markov_strings::*;
/// let data: Vec<InputData> = vec!["foo bar".to_string()]
///     .iter()
///     .map(|s| s.to_owned().into())
///     .collect();
/// ```
#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct InputData {
    /// The required value from which the generator will build new strings
    pub text: String,
    /// An optional field can contain any serialized data that you may wish to retrieve later from the [`Result.refs`](struct.MarkovResult.html#structfield.refs) set
    pub meta: Option<String>,
}

impl From<String> for InputData {
    fn from(text: String) -> Self {
        InputData { text, meta: None }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DataMember {
    state_size: usize,
}

/// Struct holding the generator's results.
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarkovResult {
    /// The generated text
    pub text: String,
    /// A relative value based on the possible number of permutations that led to this result.
    /// Higher is usually "better", but the threshold depends on your corpus.
    pub score: u16,
    /// The list of references ids that were used to create this result. To retrieve the original values
    pub refs: Vec<usize>,
    /// The number of tries it took to generate the result
    pub tries: u16,
}

type Fragments = HashMap<String, Vec<usize>>;

/// Struct used to import and export data.
///
/// See [`Markov::export()`] for more information.
#[derive(Serialize, Deserialize, Debug)]
pub struct ImportExport {
    data: Vec<InputData>,
    corpus: Corpus,
    start_words: Fragments,
    end_words: Fragments,
    options: DataMember,
}

/// Struct for possible errors during the corpus building, or result generation.
#[derive(Debug, Eq, PartialEq)]
pub enum ErrorType {
    /// Returned by [`Markov::generate()`] if your corpus is empty
    CorpusEmpty,
    /// Returned by [´Markov::set_state_size()`] if your corpus already contain data.
    CorpusNotEmpty,
    /// Returned by [`Markov::generate()`] if it exceeds the maximum allowed tries to generate a result.
    TriesExceeded,
}

type MarkovResultFilter = fn(&MarkovResult) -> bool;
type Corpus = HashMap<String, Fragments>;

/// The Markov chain generator
///
/// 1. Initialize it empty or from saved corpus
/// 2. Add data to complete the corpus
/// 3. Generate results
#[derive(Serialize, Deserialize, Clone)]
pub struct Markov {
    /// Raw data (a list of strings)
    data: Vec<InputData>,
    options: DataMember,
    start_words: Fragments,
    end_words: Fragments,
    corpus: HashMap<String, Fragments>,
    #[serde(skip)]
    filter: Option<fn(&MarkovResult) -> bool>,
    #[serde(skip)]
    max_tries: u16,
}

impl Markov {
    /// Creates an empty Markov instance
    ///
    /// ```rust
    /// use markov_strings::*;
    ///
    /// let mut markov = Markov::new();
    /// ```
    pub fn new() -> Markov {
        let opts = DataMember { state_size: 2 };
        Markov {
            data: vec![],
            options: opts,
            start_words: HashMap::new(),
            end_words: HashMap::new(),
            corpus: HashMap::new(),
            filter: None,
            max_tries: 100,
        }
    }

    /// Creates a Markov instance from previously imported data
    ///
    /// See [`Markov::export()`] for more information.
    ///
    /// Example: load your saved corpus from a flat file with the `bincode` crate.
    /// ```ignore
    /// let file = File::open("dumped.db").unwrap();
    /// let reader = BufReader::new(file);
    /// let data = bincode::deserialize_from(reader).unwrap();
    /// let mut markov = Markov::from_export(data);
    /// ```
    pub fn from_export(export: ImportExport) -> Markov {
        Markov {
            data: export.data,
            options: export.options,
            corpus: export.corpus,
            filter: None,
            start_words: export.start_words,
            end_words: export.end_words,
            max_tries: 10,
        }
    }

    /// Sets the "state size" of your Markov generator.
    ///
    /// The result chain is made up of consecutive blocks of words, and each block is called a state.
    /// Each state is itself made up of one (1) or more words.
    ///
    /// ```rust
    /// # use markov_strings::*;
    /// let data: Vec<InputData> = vec![];
    /// # let data: Vec<InputData> = vec![
    /// #   InputData{ text: "foo bar lorem ipsum".to_string(), meta: None },
    /// # ];
    /// let mut markov = Markov::new();
    ///
    /// // We _must_ set the state_size before adding data...
    /// assert!(markov.set_state_size(3).is_ok());
    ///
    /// // ...or it will return an error
    /// markov.add_to_corpus(data);
    /// assert!(markov.set_state_size(4).is_err());
    /// ```
    ///
    /// - A state size of `1` word will mostly output non-sense gibberish.
    /// - A state size of `2` words can produce interesting results, when correctly filtered.
    /// - A state size of `3` or more words will produce more intelligible results,
    /// but you'll need a source material that will allow it while staying random enough.
    ///
    /// **! You CANNOT change the state_size once you've added data with [`Markov::add_to_corpus()`]**.<br>
    /// The internal data structure is reliant on the state size, and it cannot be changed without
    /// rebuilding the whole corpus.
    ///
    /// Default value `2`.
    pub fn set_state_size(&mut self, size: usize) -> Result<&mut Self, ErrorType> {
        if self.start_words.len() > 0 {
            return Err(ErrorType::CorpusNotEmpty);
        }
        self.options.state_size = size;
        Ok(self)
    }

    /// Adds data to your Markov instance's corpus.
    ///
    /// This is an expensive method that can take a few seconds,
    /// depending on the size of your input data.
    /// For example, adding 50.000 tweets while running on fairly decent computer takes more than 20 seconds.
    ///
    /// To avoid rebuilding the corpus each time you want to generate a text,
    /// you can use [`Markov::export()`] and [`Markov::from_export()`]
    ///
    /// You can call `.add_to_corpus()` as many times as you need it.
    pub fn add_to_corpus(&mut self, data: Vec<InputData>) {
        data.iter().for_each(|o| self.data.push(o.to_owned()));
        let state_size = self.options.state_size;
        // let data_len = data.len();
        // let mut data_done = 0;

        // Loop through all sentences
        for item in data.iter() {
            // Get position of current item in self.data
            let pos = self.data.iter().position(|o| o == item).unwrap();

            // data_done += 1;
            // println!("{:?}/{:?}", data_done, data_len);

            let words = item.text.split(' ').collect::<Vec<&str>>();

            let count = words.len();
            if count < self.options.state_size {
                continue;
            }

            // "Start words" is the list of words that can start a generated chain.
            let start = (&words)
                .iter()
                .take(state_size)
                .map(|s| s.to_owned())
                .collect::<Vec<_>>()
                .join(" ");
            self.start_words.entry(start).or_insert(vec![]).push(pos);

            // "End words" is the list of words that can end a generated chain
            let end = (&words)
                .iter()
                .skip(count - state_size)
                .take(state_size)
                .map(|s| s.to_owned())
                .collect::<Vec<&str>>()
                .join(" ");
            self.end_words.entry(end).or_insert(vec![]).push(pos);

            // Corpus generation

            // We loop through all words in the sentence to build "blocks" of `state_size`
            // e.g. for a state_size of 2, "lorem ipsum dolor sit amet" will have the following blocks:
            //    "lorem ipsum", "ipsum dolor", "dolor sit", and "sit amet"
            for (i, _) in words.clone().iter().enumerate() {
                let curr = (&words)
                    .iter()
                    .skip(i)
                    .take(state_size)
                    .map(|s| s.to_owned())
                    .collect::<Vec<&str>>()
                    .join(" ");

                let next = (&words)
                    .iter()
                    .skip(i + state_size)
                    .take(state_size)
                    .map(|s| s.to_owned())
                    .collect::<Vec<&str>>()
                    .join(" ");

                // Filter out fragments that are empty or too short
                if next.len() == 0 || next.split(' ').count() < state_size {
                    continue;
                }

                self.corpus
                    // Get or create the "curr" block
                    .entry(curr)
                    .or_insert(HashMap::new())
                    // Insert the "next" value
                    .entry(next)
                    .or_insert(vec![pos])
                    .push(pos);
            }
        }
    }

    /// Sets a filter to ensure that outputted results match your own criteria.
    ///
    /// A good filter is **essential** to get interesting results out of [`Markov::generate()`].
    /// The values you should check at minimum are the [`MarkovResult.score`](struct.MarkovResult.html#structfield.score) and [`MarkovResult.refs`](struct.MarkovResult.html#structfield.refs)' length.
    ///
    /// The higher these values, the "better" the results. The actual thresholds are entierely dependant
    /// of your source material.
    ///
    /// ```rust
    /// # use markov_strings::*;
    /// let mut markov = Markov::new();
    /// // We're going to generate tweets, so...
    /// markov
    ///     .set_filter(|r| {
    ///         // Minimum score and number of references
    ///         // to ensure good randomness
    ///         r.score > 50 && r.refs.len() > 10
    ///             // Max length of a tweet
    ///             && r.text.len() <= 280
    ///             // No mentions
    ///             && !r.text.contains("@")
    ///             // No urls
    ///             && !r.text.contains("http")
    ///   });
    /// ```
    pub fn set_filter(&mut self, f: MarkovResultFilter) -> &mut Self {
        self.filter = Some(f);
        self
    }

    /// Removes the filter, if any
    ///
    /// ```rust
    /// # use markov_strings::*;
    /// let mut markov = Markov::new();
    /// // Those two lines a functionally identical.
    /// markov.set_filter(|r| true);
    /// markov.unset_filter();
    /// ```
    pub fn unset_filter(&mut self) -> &mut Self {
        self.filter = None;
        self
    }

    /// Sets the maximum number of times the generator will try to generate a result.
    ///
    /// If [`Markov::generate`] fails [max_tries] times to generate a sentence,
    /// it returns an [`ErrorType.TriesExceeded`](enum.ErrorType.html#variant.TriesExceeded).
    ///
    /// Default value: `100`
    pub fn set_max_tries(&mut self, tries: u16) -> &mut Self {
        self.max_tries = tries;
        self
    }

    /// Generates a random result from your corpus.
    ///
    /// ```rust
    /// # use markov_strings::*;
    /// let mut markov = Markov::new();
    /// let data: Vec<InputData> = vec![/* lots of data */];
    /// # let data: Vec<InputData> = vec![
    /// #   InputData{ text: "foo bar lorem ipsum".to_string(), meta: None },
    /// # ];
    /// markov.add_to_corpus(data);
    /// let result = markov.generate().unwrap();
    /// ```
    pub fn generate(&self) -> Result<MarkovResult, ErrorType> {
        if self.corpus.len() == 0 {
            return Err(ErrorType::CorpusEmpty);
        }
        let max_tries = self.max_tries;
        let mut tries: u16 = 0;
        let mut rng = thread_rng();

        // Loop through fragments to create a complete sentence
        for _ in 0..max_tries {
            tries += 1;
            let mut ended = false;
            let mut references: HashSet<usize> = HashSet::new();

            // Create an array of MarkovCorpusItems
            // The first item is a random startWords element
            let mut arr = vec![self.start_words.iter().choose(&mut rng).unwrap()];
            let mut score: u16 = 0;

            // Loop to build a complete sentence
            for _ in 0..max_tries {
                // Last value in array
                let block = arr[arr.len() - 1];

                // Find a following item in the corpus
                let fragments = match self.corpus.get(block.0) {
                    Some(v) => v,
                    // If a state cannot be found, the sentence can't be completed
                    None => break,
                };
                let state = fragments.iter().choose(&mut rng).unwrap();

                // Add new state to list
                arr.push(state);
                // Save references
                state.1.iter().for_each(|o| {
                    references.insert(*o);
                });

                // Increment score
                score += (self.corpus.get(block.0).unwrap().len() - 1) as u16;

                // Is sentence finished?
                if self.end_words.get(state.0).is_some() {
                    ended = true;
                    break;
                }
            }
            let sentence = arr
                .iter()
                .map(|o| o.0.to_owned())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            let result = MarkovResult {
                text: sentence,
                score,
                refs: references.into_iter().collect::<Vec<_>>(),
                tries,
            };

            // Sentence is not ended or incorrect
            // let filter = options.filter_result.unwrap();
            if !ended || (self.filter.is_some() && !self.filter.unwrap()(&result)) {
                continue;
            }

            return Ok(result);
        }

        Err(ErrorType::TriesExceeded)
    }

    /// Gets an item from the original data.
    ///
    /// Use this with the indices from [`MarkovResult.refs`](struct.MarkovResult.html#structfield.refs)
    ///
    /// ```rust
    /// # use markov_strings::*;
    /// # use std::collections::{HashMap, HashSet};
    /// let data: Vec<InputData> = vec![
    ///   InputData{ text: "foo bar lorem ipsum".to_string(), meta: Some("something".to_string()) },
    /// ];
    /// let mut markov = Markov::new();
    /// markov.add_to_corpus(data);
    /// let result = markov.generate().unwrap();
    ///
    /// // Since we only have 1 string in our corpus, we have 1 ref...
    /// let mut expected: Vec<usize> = vec![];
    /// expected.push(0);
    /// assert_eq!(result.refs, expected);
    /// let input_ref = *result.refs.get(0).unwrap();
    /// assert_eq!(markov.get_input_ref(input_ref).unwrap().text, "foo bar lorem ipsum");
    /// assert_eq!(markov.get_input_ref(input_ref).unwrap().meta, Some("something".to_string()));
    /// ```
    pub fn get_input_ref(self: &Self, index: usize) -> Option<&InputData> {
        self.data.get(index)
    }

    /// Exports the corpus into a serializable structure.
    ///
    /// The [`Markov::add_to_corpus()`] method being expensive, you may want to build your corpus once,
    /// then export it to a serializable file file for later use.
    ///
    /// ```rust
    /// # use markov_strings::*;
    /// let data: Vec<InputData> = vec![];
    /// let mut markov = Markov::new();
    /// markov.add_to_corpus(data);
    /// let export = markov.export();
    ///
    /// let markov = Markov::from_export(export);
    /// let result = markov.generate();
    /// ```
    pub fn export(self) -> ImportExport {
        return ImportExport {
            data: self.data,
            options: self.options,
            corpus: self.corpus,
            start_words: self.start_words,
            end_words: self.end_words,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_example_data() -> Vec<InputData> {
        let data: Vec<&str> = vec![
            "Lorem ipsum dolor sit amet",
            "Lorem ipsum duplicate start words",
            "Consectetur adipiscing elit",
            "Quisque tempor, erat vel lacinia imperdiet",
            "Justo nisi fringilla dui",
            "Egestas bibendum eros nisi ut lacus",
            "fringilla dui avait annoncé une rupture avec le erat vel: il n'en est rien…",
            "Fusce tincidunt tempor, erat vel lacinia vel ex pharetra pretium lacinia imperdiet",
        ];
        data.iter()
            .map(|s| InputData {
                text: s.to_string(),
                meta: None,
            })
            .collect()
    }

    #[test]
    fn constructor_has_default_state_size() {
        let markov = Markov::new();
        assert!(markov.options.state_size == 2)
    }

    #[test]
    fn set_state_size_works() {
        let mut markov = Markov::new();
        markov.set_state_size(3).unwrap();
        assert_eq!(markov.options.state_size, 3)
    }

    #[test]
    fn add_to_corpus_works() {
        let mut markov = Markov::new();
        assert_eq!(markov.corpus.len(), 0);
        markov.add_to_corpus(get_example_data());
        assert_eq!(markov.corpus.len(), 28)
    }

    #[test]
    fn start_words_should_have_the_right_length() {
        let mut markov = Markov::new();
        markov.add_to_corpus(get_example_data());
        assert_eq!(markov.start_words.len(), 7 as usize);
    }

    #[test]
    fn start_words_should_contain_the_right_values() {
        let mut markov = Markov::new();
        markov.add_to_corpus(get_example_data());
        let fragments = &markov.start_words;
        assert!(fragments.iter().any(|o| o.0 == "Lorem ipsum"));
        assert!(fragments.iter().any(|o| o.0 == "Consectetur adipiscing"));
        assert!(fragments.iter().any(|o| o.0 == "Quisque tempor,"));
        assert!(fragments.iter().any(|o| o.0 == "Justo nisi"));
        assert!(fragments.iter().any(|o| o.0 == "Egestas bibendum"));
        assert!(fragments.iter().any(|o| o.0 == "fringilla dui"));
        assert!(fragments.iter().any(|o| o.0 == "Fusce tincidunt"));
    }

    #[test]
    fn end_words_should_have_the_right_length() {
        let mut markov = Markov::new();
        markov.add_to_corpus(get_example_data());
        assert_eq!(markov.end_words.len(), 7 as usize);
    }

    #[test]
    fn end_words_should_contain_the_right_values() {
        let mut markov = Markov::new();
        markov.add_to_corpus(get_example_data());
        let fragments = &markov.end_words;
        assert!(fragments.iter().any(|o| o.0 == "sit amet"));
        assert!(fragments.iter().any(|o| o.0 == "start words"));
        assert!(fragments.iter().any(|o| o.0 == "adipiscing elit"));
        assert!(fragments.iter().any(|o| o.0 == "fringilla dui"));
        assert!(fragments.iter().any(|o| o.0 == "ut lacus"));
        assert!(fragments.iter().any(|o| o.0 == "est rien…"));
    }

    #[test]
    fn corpus_should_have_the_right_values_for_the_right_keys() {
        let mut markov = Markov::new();
        markov.add_to_corpus(get_example_data());
        let fragments = &markov.corpus.get("Lorem ipsum").unwrap();
        assert!(fragments.iter().any(|f| f.0 == "dolor sit"));
        assert!(fragments.iter().any(|f| f.0 == "duplicate start"));
        let fragments = &markov.corpus.get("tempor, erat").unwrap();
        assert!(fragments.iter().any(|f| f.0 == "vel lacinia"));
    }

    #[test]
    fn generator_should_return_err_if_the_corpus_is_not_build() {
        let markov = Markov::new();
        let res = markov.generate();
        assert_eq!(res.unwrap_err(), ErrorType::CorpusEmpty);
    }

    #[test]
    fn generator_should_return_a_result_under_the_tries_limit() {
        let mut markov = Markov::new();
        markov.add_to_corpus(get_example_data());
        for _ in 0..10 {
            let sentence = markov.generate();
            assert!(sentence.unwrap().tries < 20);
        }
    }

    #[test]
    fn generator_should_return_error() {
        // Arrange
        let mut markov = Markov::new();
        markov.add_to_corpus(get_example_data());

        // Act
        let result = markov.set_filter(|_| false).generate();

        // Assert
        assert_eq!(result.unwrap_err(), ErrorType::TriesExceeded);
    }

    #[test]
    fn result_should_end_with_an_endwords_item() {
        // Arrange
        let mut markov = Markov::new();
        markov.add_to_corpus(get_example_data());

        for _ in 0..10 {
            // Act
            let result = markov.generate().unwrap();
            let arr = result.text.split(' ').collect::<Vec<_>>();
            let len = arr.len();
            let end = arr
                .into_iter()
                .skip(len - 2)
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");
            // Assert
            assert!(markov.end_words.iter().any(|f| f.0 == &end));
        }
    }

    #[test]
    fn input_data_from_string() {
        let text = "foo";
        let input = InputData::from(text.to_owned());
        assert_eq!(input.text, "foo");
        assert_eq!(input.meta, None);

        let texts = vec!["foo".to_string()];
        let mut markov = Markov::new();
        markov.add_to_corpus(
            texts
                .iter()
                .map(|t| t.to_owned().into())
                .collect::<Vec<_>>(),
        );
    }
}
