use super::{
    create_default_chain,
    markov_chain::filter_string_for_markov_file,
    model::{MARKOV_DATA_SET_PATH, MARKOV_EXPORT_PATH},
};
use crate::client::file_operations::create_file_if_missing;
use anyhow::{Context, Result};
use markov_str::MarkovChain;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    fs::{self, OpenOptions},
    io::Write,
};
use tracing::{info_span, instrument};

/// Append a sentence to the markov file
pub fn append_to_markov_file(str: &str) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new().append(true).open(MARKOV_DATA_SET_PATH)?;

    Ok(writeln!(file, "{str}\n")?)
}

/// If the way that messages are filtered before being added to the data set is changed then
/// it's helpful to call this function when the bot starts so the filtering is consistent across the file.
#[allow(dead_code)]
pub fn clean_markov_file() {
    let file = fs::read_to_string(MARKOV_DATA_SET_PATH)
        .expect("Something went wrong while reading the file.");
    let messages = file.split("\n\n").collect::<Vec<&str>>();

    fs::write(MARKOV_DATA_SET_PATH, "").expect("Something went wrong while writing to file.");

    let filtered_messages: Vec<String> = messages
        .into_par_iter()
        .map(filter_string_for_markov_file)
        .collect();

    for message in filtered_messages {
        if let Err(why) = append_to_markov_file(&message) {
            eprintln!("{why}");
        }
    }
}

#[instrument]
pub fn import_chain_from_file() -> Result<MarkovChain> {
    let file_contents = info_span!("Reading file").in_scope(|| {
        let x = create_file_if_missing(MARKOV_EXPORT_PATH, "").context("Failed to create file")?;
        fs::read_to_string(x).context("Failed to read file")
    })?;

    let import_export = info_span!("Parsing file contents")
        .in_scope(|| serde_json::from_str::<MarkovChain>(&file_contents))?;

    Ok(import_export)
}

#[instrument]
/// Reads the Markov data set from [`MARKOV_DATA_SET_PATH`]
pub fn get_messages_from_file() -> Result<Vec<String>> {
    let text_from_file = fs::read_to_string(create_file_if_missing(MARKOV_DATA_SET_PATH, "")?)?;
    let text_array: Vec<&str> = text_from_file.split("\n\n").collect();
    Ok(text_array.into_par_iter().map(std::borrow::ToOwned::to_owned).collect())
}

#[instrument]
/// also writes the serialized version to file
pub fn generate_new_chain_from_msg_file() -> Result<MarkovChain> {
    let messages = get_messages_from_file()?;

    let mut markov = create_default_chain();
    for msg in messages {
        markov.add_text(&msg);
    }

    let export = serde_json::to_string(&markov)?;

    fs::write(MARKOV_EXPORT_PATH, export)?;

    Ok(markov)
}
