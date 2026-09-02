/// The agent's "SYSTEM" prompt
static DEFAULT_PREAMBLE: &str = "You are a helpful assistant that can answer questions about files.
You have access to the user's files from the context provided, you will be given a question and you should answer it, based on the context.";

pub mod agent;
pub mod config;
pub mod rag;
