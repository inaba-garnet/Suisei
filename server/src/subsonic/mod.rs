//! Subsonic API の共通部分（引数、認証、応答の形式）。

mod auth;
mod params;
mod response;
mod xml;

pub use auth::authenticate;
pub use params::Params;
pub use response::{Error, ErrorCode, Format, error, ok};
