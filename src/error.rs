// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::ffi::OsString;
use std::io;
use std::{env::VarError, num::ParseIntError};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use nom::error::{ErrorKind, ParseError};
use thiserror::Error as ThisError;
use zip::result::ZipError;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("SQL error")]
    SqlError(#[from] sqlx::Error),
    #[error("Migrate error")]
    MigrateError(#[from] sqlx::migrate::MigrateError),
    #[error("Failed to read environment variable")]
    EnvVar(#[from] VarError),
    #[error("Not implemented: {}", .0)]
    NotImplementedError(String),
    #[error("Invalid request: {}", .0)]
    InvalidRequest(String),
    #[error("Not found")]
    NotFound,
    #[error("Parser error")]
    ParserError,
    #[error("Json Parser error")]
    JsonParserError(#[from] serde_json::Error),
    #[error("Failed to parse integer")]
    IntParserError(#[from] ParseIntError),
    #[error("Failed to convert OsString")]
    OsStringError(OsString),
    #[error("IO error")]
    IoError(#[from] io::Error),
    #[error("CompaRiPPSon error: {}", .0)]
    CompaRiPPsonError(String),
    #[error("Error compressing file")]
    CompressionError(#[from] ZipError),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        println!("->> {:<12} - {self:?}", "INTO_RES");

        match self {
            Self::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg.to_owned()),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                ClientError::NOT_FOUND.as_ref().to_string(),
            ),
            Self::NotImplementedError(msg) => (StatusCode::NOT_IMPLEMENTED, msg.to_owned()),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ClientError::UNHANDLED_SERVER_ERROR.as_ref().to_string(),
            ),
        }
        .into_response()
    }
}

impl<I> ParseError<I> for Error {
    fn from_error_kind(_input: I, _kind: ErrorKind) -> Self {
        Error::ParserError
    }
    fn append(_input: I, _kind: ErrorKind, other: Self) -> Self {
        other
    }
}

#[derive(Debug, strum::AsRefStr)]
#[allow(non_camel_case_types)]
pub enum ClientError {
    INVALID_PARAMS,
    NOT_FOUND,
    UNHANDLED_SERVER_ERROR,
}
