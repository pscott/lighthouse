use criterion::{black_box, criterion_group, criterion_main, Criterion};
use slashing_protection::attester_slashings::SignedAttestation;
use slashing_protection::slashing_protection::{HistoryInfo, SlashingProtection};
use types::{AttestationData, Epoch, Hash256, Checkpoint, Crosslink};
use tempfile::NamedTempFile;
use rand::Rng;

    fn random_attestation_builder(mut i: usize) -> AttestationData {
        if i == 0 {
            i += 2;
        }

        let mut rng = rand::thread_rng();
        let source = rng.gen_range(i - 2, i + 2) as u64;
        let target = rng.gen_range(i + 2, i + 5) as u64;
        attestation_data_builder(source, target)
    }

	fn attestation_data_builder(source: u64, target: u64) -> AttestationData {
        let source = build_checkpoint(source);
        let target = build_checkpoint(target);
        let crosslink = Crosslink::default();

        AttestationData {
            beacon_block_root: Hash256::zero(),
            source,
            target,
            crosslink,
        }
    }

    fn build_checkpoint(epoch_num: u64) -> Checkpoint {
        Checkpoint {
            epoch: Epoch::from(epoch_num),
            root: Hash256::zero(),
        }
    }

pub fn criterion_benchmark(c: &mut Criterion) {
    let attestation_file = NamedTempFile::new().expect("couldn't create temporary file");
    let filename = attestation_file.path();

    let mut attestation_history: HistoryInfo<SignedAttestation> =
        HistoryInfo::empty(filename).expect("IO error with file");

    let mut i = 2;
    c.bench_function("history rnd", |b| b.iter(|| {
        let attest = random_attestation_builder(i);
        let _ = attestation_history.update_if_valid(black_box(&attest));
        i += 1;
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
