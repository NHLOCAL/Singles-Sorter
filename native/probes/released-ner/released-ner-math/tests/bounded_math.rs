//! Uses only the verified original export and deterministic synthetic buckets.
use released_ner_probe::{CancellationToken, InferenceError, Limits, Model, RequestControl};

#[test]
#[ignore = "requires ORIGINAL_NER_EXPORT verified original assets"]
fn bounded_forward_matches_proof_at_token_boundary() {
    let dir = std::env::var_os("ORIGINAL_NER_EXPORT").expect("ORIGINAL_NER_EXPORT");
    let model = Model::load(std::path::Path::new(&dir)).unwrap();
    let limits = Limits::default();
    for n in [0, 1, 17, 1024] {
        let buckets: Vec<[[u32; 4]; 4]> = (0..n)
            .map(|i| {
                std::array::from_fn(|f| std::array::from_fn(|k| (i * 997 + f * 97 + k * 13) as u32))
            })
            .collect();
        let token = CancellationToken::new();
        let c = RequestControl::new(&limits, &token).unwrap();
        let checked = model.forward_fast_bounded(&buckets, &c).unwrap();
        let expected = model.forward_fast(&buckets);
        assert_eq!(
            checked
                .affine
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            expected
                .affine
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>()
        );
        assert!(checked.stages.is_empty());
        let padded = if n == 0 { 0 } else { n + 8 };
        assert_eq!(c.work_used(), n * 286_336 + padded * 672_000);
        assert_eq!(c.tensor_bytes_used(), (n * 1696 + padded * 2784 + 384) * 4);
        let context = if n == 0 {
            [-1; 3]
        } else {
            [n as isize - 1, 0, -1]
        };
        let actual = model.logits_bounded(&checked, context, &c).unwrap();
        let expected = model.logits(&expected, context);
        assert_eq!(
            actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        token.cancel();
        assert!(matches!(
            model.forward_fast_bounded(&buckets, &c),
            Err(InferenceError::Cancelled)
        ));
    }
    let token = CancellationToken::new();
    let c = RequestControl::new(&limits, &token).unwrap();
    assert!(matches!(
        model.forward_fast_bounded(&vec![[[0; 4]; 4]; 1025], &c),
        Err(InferenceError::ResourceLimit {
            resource: "tokens",
            ..
        })
    ));
    assert_eq!(c.tensor_bytes_used(), 0);
    let fresh = model.forward_fast_bounded(&[[[0; 4]; 4]], &c).unwrap();
    assert_eq!(fresh.n_tokens, 1);
    assert!(matches!(
        model.logits_bounded(&fresh, [-2, -1, -1], &c),
        Err(InferenceError::Model(_))
    ));
}
