#![cfg(test)]
use super::*;
use crate::test_utils::*;

ssz_tests!(FoundationBeaconState);

fn test_beacon_proposer_index<T: EthSpec>() {
    let spec = T::default_spec();
    let relative_epoch = RelativeEpoch::Current;

    // Build a state for testing.
    let build_state = |validator_count: usize| -> BeaconState<T> {
        let builder: TestingBeaconStateBuilder<T> =
            TestingBeaconStateBuilder::from_default_keypairs_file_if_exists(validator_count, &spec);
        let (mut state, _keypairs) = builder.build();
        state
            .build_committee_cache(relative_epoch, &spec)
            .expect("built committee cache");
        state
            .update_proposer_indices_cache(state.slot, &spec)
            .expect("built proposer indices cache");

        state
    };

    // Get the i'th candidate proposer for the given state and slot
    let ith_candidate = |state: &BeaconState<T>, slot: Slot, i: usize| {
        let epoch = slot.epoch(T::slots_per_epoch());
        let seed = state.get_beacon_proposer_seed(slot, &spec).unwrap();
        let active_validators = state.get_active_validator_indices(epoch);
        active_validators[compute_shuffled_index(
            i,
            active_validators.len(),
            &seed,
            spec.shuffle_round_count,
        )
        .unwrap()]
    };

    // Run a test on the state.
    let test = |state: &BeaconState<T>, slot: Slot, candidate_index: usize| {
        assert_eq!(
            state.get_beacon_proposer_index(slot),
            Ok(ith_candidate(state, slot, candidate_index))
        );
    };

    // Test where we have one validator per slot.
    // 0th candidate should be chosen every time.
    let state = build_state(T::slots_per_epoch() as usize);
    for i in 0..T::slots_per_epoch() {
        test(&state, Slot::from(i), 0);
    }

    // Test where we have two validators per slot.
    // 0th candidate should be chosen every time.
    let state = build_state(T::slots_per_epoch() as usize * 2);
    for i in 0..T::slots_per_epoch() {
        test(&state, Slot::from(i), 0);
    }

    // Test with two validators per slot, first validator has zero balance.
    let mut state = build_state(T::slots_per_epoch() as usize * 2);
    let slot0_candidate0 = ith_candidate(&state, Slot::new(0), 0);
    state.validators[slot0_candidate0].effective_balance = 0;
    state.drop_proposer_indices_cache();
    state
        .update_proposer_indices_cache(state.slot, &spec)
        .unwrap();
    test(&state, Slot::new(0), 1);
    for i in 1..T::slots_per_epoch() {
        test(&state, Slot::from(i), 0);
    }
}

#[test]
fn beacon_proposer_index() {
    test_beacon_proposer_index::<MinimalEthSpec>();
}

/// Test that
///
/// 1. Using the cache before it's built fails.
/// 2. Using the cache after it's build passes.
/// 3. Using the cache after it's dropped fails.
fn test_cache_initialization<'a, T: EthSpec>(
    state: &'a mut BeaconState<T>,
    relative_epoch: RelativeEpoch,
    spec: &ChainSpec,
) {
    let slot = relative_epoch
        .into_epoch(state.slot.epoch(T::slots_per_epoch()))
        .start_slot(T::slots_per_epoch());

    // Assuming the cache isn't already built, assert that a call to a cache-using function fails.
    assert_eq!(
        state.get_attestation_duties(0, relative_epoch),
        Err(BeaconStateError::CommitteeCacheUninitialized(Some(
            relative_epoch
        )))
    );

    // Build the cache.
    state.build_all_caches(spec).unwrap();

    // Assert a call to a cache-using function passes.
    let _ = state.get_beacon_proposer_index(slot).unwrap();

    // Drop the cache.
    state.drop_all_caches();

    // Assert a call to a cache-using function fail.
    assert_eq!(
        state.get_beacon_committee(slot, 0),
        Err(BeaconStateError::CommitteeCacheUninitialized(Some(
            relative_epoch
        )))
    );
}

#[test]
fn cache_initialization() {
    let spec = MinimalEthSpec::default_spec();

    let builder: TestingBeaconStateBuilder<MinimalEthSpec> =
        TestingBeaconStateBuilder::from_default_keypairs_file_if_exists(16, &spec);
    let (mut state, _keypairs) = builder.build();

    state.slot =
        (MinimalEthSpec::genesis_epoch() + 1).start_slot(MinimalEthSpec::slots_per_epoch());

    test_cache_initialization(&mut state, RelativeEpoch::Previous, &spec);
    test_cache_initialization(&mut state, RelativeEpoch::Current, &spec);
    test_cache_initialization(&mut state, RelativeEpoch::Next, &spec);
}

#[test]
fn tree_hash_cache() {
    use crate::test_utils::{SeedableRng, TestRandom, XorShiftRng};
    use tree_hash::TreeHash;

    let mut rng = XorShiftRng::from_seed([42; 16]);

    let mut state: FoundationBeaconState = BeaconState::random_for_test(&mut rng);

    let root = state.update_tree_hash_cache().unwrap();

    assert_eq!(root.as_bytes(), &state.tree_hash_root()[..]);

    state.slot += 1;

    let root = state.update_tree_hash_cache().unwrap();
    assert_eq!(root.as_bytes(), &state.tree_hash_root()[..]);
}

/// Tests committee-specific components
#[cfg(test)]
mod committees {
    use super::*;
    use crate::beacon_state::MinimalEthSpec;
    use swap_or_not_shuffle::shuffle_list;

    fn execute_committee_consistency_test<T: EthSpec>(
        state: BeaconState<T>,
        epoch: Epoch,
        validator_count: usize,
        spec: &ChainSpec,
    ) {
        let active_indices: Vec<usize> = (0..validator_count).collect();
        let seed = state.get_seed(epoch, Domain::BeaconAttester, spec).unwrap();
        let relative_epoch = RelativeEpoch::from_epoch(state.current_epoch(), epoch).unwrap();

        let mut ordered_indices = state
            .get_cached_active_validator_indices(relative_epoch)
            .unwrap()
            .to_vec();
        ordered_indices.sort_unstable();
        assert_eq!(
            active_indices, ordered_indices,
            "Validator indices mismatch"
        );

        let shuffling =
            shuffle_list(active_indices, spec.shuffle_round_count, &seed[..], false).unwrap();

        let mut expected_indices_iter = shuffling.iter();

        // Loop through all slots in the epoch being tested.
        for slot in epoch.slot_iter(T::slots_per_epoch()) {
            let beacon_committees = state.get_beacon_committees_at_slot(slot).unwrap();

            // Assert that the number of committees in this slot is consistent with the reported number
            // of committees in an epoch.
            assert_eq!(
                beacon_committees.len() as u64,
                state.get_epoch_committee_count(relative_epoch).unwrap() / T::slots_per_epoch()
            );

            for (committee_index, bc) in beacon_committees.iter().enumerate() {
                // Assert that indices are assigned sequentially across committees.
                assert_eq!(committee_index as u64, bc.index);
                // Assert that a committee lookup via slot is identical to a committee lookup via
                // index.
                assert_eq!(state.get_beacon_committee(bc.slot, bc.index).unwrap(), *bc);

                // Loop through each validator in the committee.
                for (committee_i, validator_i) in bc.committee.iter().enumerate() {
                    // Assert the validators are assigned contiguously across committees.
                    assert_eq!(
                        *validator_i,
                        *expected_indices_iter.next().unwrap(),
                        "Non-sequential validators."
                    );
                    // Assert a call to `get_attestation_duties` is consistent with a call to
                    // `get_beacon_committees_at_slot`
                    let attestation_duty = state
                        .get_attestation_duties(*validator_i, relative_epoch)
                        .unwrap()
                        .unwrap();
                    assert_eq!(attestation_duty.slot, slot);
                    assert_eq!(attestation_duty.index, bc.index);
                    assert_eq!(attestation_duty.committee_position, committee_i);
                    assert_eq!(attestation_duty.committee_len, bc.committee.len());
                }
            }
        }

        // Assert that all validators were assigned to a committee.
        assert!(expected_indices_iter.next().is_none());
    }

    fn committee_consistency_test<T: EthSpec>(
        validator_count: usize,
        state_epoch: Epoch,
        cache_epoch: RelativeEpoch,
    ) {
        let spec = &T::default_spec();

        let mut builder = TestingBeaconStateBuilder::from_single_keypair(
            validator_count,
            &Keypair::random(),
            spec,
        );

        let slot = state_epoch.start_slot(T::slots_per_epoch());
        builder.teleport_to_slot(slot);

        let (mut state, _keypairs): (BeaconState<T>, _) = builder.build();

        let distinct_hashes: Vec<Hash256> = (0..T::epochs_per_historical_vector())
            .map(|i| Hash256::from_low_u64_be(i as u64))
            .collect();
        state.randao_mixes = FixedVector::from(distinct_hashes);

        state
            .build_committee_cache(RelativeEpoch::Previous, spec)
            .unwrap();
        state
            .build_committee_cache(RelativeEpoch::Current, spec)
            .unwrap();
        state
            .build_committee_cache(RelativeEpoch::Next, spec)
            .unwrap();

        let cache_epoch = cache_epoch.into_epoch(state_epoch);

        execute_committee_consistency_test(state, cache_epoch, validator_count as usize, &spec);
    }

    fn committee_consistency_test_suite<T: EthSpec>(cached_epoch: RelativeEpoch) {
        let spec = T::default_spec();

        let validator_count = spec.max_committees_per_slot
            * T::slots_per_epoch() as usize
            * spec.target_committee_size
            + 1;

        committee_consistency_test::<T>(validator_count as usize, Epoch::new(0), cached_epoch);

        committee_consistency_test::<T>(
            validator_count as usize,
            T::genesis_epoch() + 4,
            cached_epoch,
        );

        committee_consistency_test::<T>(
            validator_count as usize,
            T::genesis_epoch() + T::slots_per_historical_root() as u64 * T::slots_per_epoch() * 4,
            cached_epoch,
        );
    }

    #[test]
    fn current_epoch_committee_consistency() {
        committee_consistency_test_suite::<MinimalEthSpec>(RelativeEpoch::Current);
    }

    #[test]
    fn previous_epoch_committee_consistency() {
        committee_consistency_test_suite::<MinimalEthSpec>(RelativeEpoch::Previous);
    }

    #[test]
    fn next_epoch_committee_consistency() {
        committee_consistency_test_suite::<MinimalEthSpec>(RelativeEpoch::Next);
    }
}
