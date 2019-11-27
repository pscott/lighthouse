pub mod signed_attestation;
pub mod signed_block;
mod utils;
pub mod validator_history;

use crate::signed_attestation::InvalidAttestation;
use crate::signed_block::InvalidBlock;
use rusqlite::Error as SQLError;
use std::io::{Error as IOError, ErrorKind};
use std::string::ToString;

impl From<IOError> for NotSafe {
    fn from(error: IOError) -> NotSafe {
        NotSafe::IOError(error.kind())
    }
}

impl From<SQLError> for NotSafe {
    fn from(error: SQLError) -> NotSafe {
        NotSafe::SQLError(error.to_string())
    }
}

impl From<r2d2::Error> for NotSafe {
    fn from(error: r2d2::Error) -> Self {
        NotSafe::SQLPoolError(format!("{:?}", error))
    }
}

impl ToString for NotSafe {
    fn to_string(&self) -> String {
        format!("{:?}", &self)
    }
}

#[derive(PartialEq, Debug)]
pub enum NotSafe {
    InvalidAttestation(InvalidAttestation),
    InvalidBlock(InvalidBlock),
    PruningError,
    NoSlotsPerEpochProvided,
    IOError(ErrorKind),
    SQLError(String),
    SQLPoolError(String),
}

#[derive(PartialEq, Debug)]
pub enum ValidityReason {
    // History is empty so inserting is safe
    EmptyHistory,
    // Re-signing a previous vote is safe
    SameVote,
    // Incoming data is safe from slashing
    Valid,
}

#[derive(PartialEq, Debug)]
pub struct Safe {
    /// Used to check if the attestation is a SameVote, in which case it should not get inserted.
    pub reason: ValidityReason,
}
