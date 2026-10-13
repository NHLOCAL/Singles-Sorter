//! Same-host exact-bit before/after proof; identical source is used for both builds.
use released_ner_probe::{Model, choose};
use sha2::{Digest, Sha256};
fn digest(values: &[f32]) -> String {
    let mut h = Sha256::new();
    for value in values {
        h.update(value.to_bits().to_le_bytes());
    }
    format!("{:x}", h.finalize())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .ok_or("usage: stage-bits EXPORT_DIRECTORY")?;
    let model = Model::load(std::path::Path::new(&directory))?;
    for n in [0usize, 1, 2, 3, 7, 58, 129, 257] {
        let hashes: Vec<[[u32; 4]; 4]> = (0..n)
            .map(|t| {
                std::array::from_fn(|f| {
                    std::array::from_fn(|k| {
                        ((t * 1997 + f * 383 + k * 65521) as u32).wrapping_mul(2654435761)
                    })
                })
            })
            .collect();
        let forward = model.forward(&hashes);
        for (name, tensor) in &forward.stages {
            println!("{n} {name} {:?} {}", tensor.shape, digest(&tensor.data));
        }
        println!("{n} affine {}", digest(&forward.affine));
        if n > 0 {
            for context in [[0, -1, -1], [(n - 1) as isize, 0, 0], [-1, -1, -1]] {
                println!(
                    "{n} logits {context:?} {}",
                    digest(&model.logits(&forward, context))
                );
            }
        }
    }
    let mut valid = [false; 22];
    valid[1] = true;
    valid[5] = true;
    let mut scores = [0f32; 22];
    scores[1] = 1.;
    scores[5] = f32::from_bits(1f32.to_bits() + 1);
    assert_eq!(choose(&scores, &valid)?, 5);
    scores[5] = scores[1];
    assert_eq!(choose(&scores, &valid)?, 1);
    println!("one_ulp_margin_and_tie PASS");
    Ok(())
}
