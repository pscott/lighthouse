use super::validator_historical_attestation::ValidatorHistoricalAttestation;
use tree_hash::TreeHash;
use types::*;

#[derive(PartialEq, Debug)]
pub enum PruningError {
    TargetEpochTooSmall(Epoch),
    SourceEpochTooSmall(Epoch),
}

#[derive(PartialEq, Debug)]
pub enum AttestationError {
    DoubleVote,
    InvalidAttestationData {
        source: Checkpoint,
        target: Checkpoint,
    },
    PruningError(PruningError),
    Surrounded,
    Surrounding,
}

#[derive(PartialEq, Debug)]
pub enum ValidAttestation {
    EmptyHistory,
    SameVote,
    ValidAttestation,
}

fn check_attestation_validity(attestation_data: &AttestationData) -> Result<(), AttestationError> {
    if attestation_data.target.epoch <= attestation_data.source.epoch {
        Err(AttestationError::InvalidAttestationData {
            source: attestation_data.source.clone(),
            target: attestation_data.target.clone(),
        })
    } else {
        Ok(())
    }
}

fn check_surrounded(
    attestation_data: &AttestationData,
    attestation_history: &[ValidatorHistoricalAttestation],
) -> Result<(), AttestationError> {
    let surrounded = attestation_history.iter().any(|historical_attestation| {
        historical_attestation.source_epoch < attestation_data.source.epoch
    });
    if surrounded {
        Err(AttestationError::Surrounded)
    } else {
        Ok(())
    }
}

fn check_surrounding(
    attestation_data: &AttestationData,
    attestation_history: &[ValidatorHistoricalAttestation],
) -> Result<(), AttestationError> {
    let surrounding = attestation_history.iter().any(|historical_attestation| {
        historical_attestation.source_epoch > attestation_data.source.epoch
    });
    if surrounding {
        Err(AttestationError::Surrounding)
    } else {
        Ok(())
    }
}

pub fn should_sign_attestation(
    attestation_data: &AttestationData,
    attestation_history: &[ValidatorHistoricalAttestation],
) -> Result<(ValidAttestation), AttestationError> {
    check_attestation_validity(attestation_data)?;
    if attestation_history.is_empty() {
        return Ok(ValidAttestation::EmptyHistory);
    }

    let target_index = match attestation_history
        .iter()
        .rev()
        .position(|historical_attestation| {
            historical_attestation.target_epoch <= attestation_data.target.epoch
        }) {
        None => {
            return Err(AttestationError::PruningError(
                PruningError::TargetEpochTooSmall(attestation_data.target.epoch),
            ))
        }
        Some(index) => attestation_history.len() - index - 1,
    };

    check_surrounded(attestation_data, &attestation_history[target_index + 1..])?;
    if attestation_history[target_index].target_epoch == attestation_data.target.epoch {
        if attestation_history[target_index].signing_root
            == Hash256::from_slice(&attestation_data.tree_hash_root())
        {
            return Ok(ValidAttestation::SameVote);
        } else {
            return Err(AttestationError::DoubleVote);
        }
    }

    let source_index =
        match attestation_history[..=target_index]
            .iter()
            .rev()
            .position(|historical_attestation| {
                historical_attestation.target_epoch <= attestation_data.source.epoch
            }) {
            None => {
                if attestation_data.source.epoch == 0 {
                    // Special case for genesis
                    0
                } else {
                    return Err(AttestationError::PruningError(
                        PruningError::SourceEpochTooSmall(attestation_data.source.epoch),
                    ));
                }
            }
            Some(index) => target_index - index + 1,
        };

    check_surrounding(
        attestation_data,
        &attestation_history[source_index..=target_index],
    )?;

    Ok(ValidAttestation::ValidAttestation)
}