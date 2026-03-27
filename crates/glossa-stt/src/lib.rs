//! Speech-to-text clients for Groq, OpenAI, and compatible APIs.

mod client;
mod compatible;
mod dto;
mod factory;
mod groq;
mod openai;

pub use self::factory::build_client;
