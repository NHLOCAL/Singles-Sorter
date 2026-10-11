//! Narrow, non-production proof for the frozen spaCy 3.7.5 NER graph.
//! Scalar float32 only; no fast-math, fused multiply-add or quantization.
pub mod control;
mod loader;
pub use control::{CancellationToken, InferenceError, Limits, RequestControl};
pub mod assets;
pub use assets::LoadError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const ACTIONS: [&str; 22] = [
    "U-", "O", "B-SINGER", "I-SINGER", "L-SINGER", "U-SINGER", "B-SONG", "I-SONG", "L-SONG",
    "U-SONG", "B-MISC", "I-MISC", "L-MISC", "U-MISC", "B-ALBUM", "I-ALBUM", "L-ALBUM", "U-ALBUM",
    "B-GENRE", "I-GENRE", "L-GENRE", "U-GENRE",
];
pub const LABELS: [&str; 5] = ["SINGER", "SONG", "MISC", "ALBUM", "GENRE"];
#[derive(Debug, Clone)]
pub struct Tensor {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}
pub struct Model {
    tensors: BTreeMap<(usize, String), Tensor>,
}
impl Model {
    pub fn tensor(&self, n: usize, p: &str, shape: &[usize]) -> &[f32] {
        let t = &self.tensors[&(n, p.into())];
        assert_eq!(t.shape, shape, "node {n} {p}");
        assert_eq!(t.data.len(), shape.iter().product::<usize>());
        &t.data
    }
    pub fn validate(&self) {
        let mut count = 0;
        for (n, p, shape) in [
            (2, "W", vec![3, 64, 2, 64]),
            (2, "b", vec![64, 2]),
            (2, "pad", vec![1, 3, 64, 2]),
            (3, "W", vec![22, 64]),
            (3, "b", vec![22]),
            (6, "W", vec![64, 96]),
            (6, "b", vec![64]),
        ] {
            count += self.tensor(n, p, &shape).len();
        }
        for (n, r) in [(32, 2000), (34, 1000), (36, 1000), (38, 1000)] {
            count += self.tensor(n, "E", &[r, 96]).len();
        }
        for (n, w) in [(39, 384), (57, 288), (59, 288), (61, 288), (63, 288)] {
            count += self.tensor(n, "W", &[96, 3, w]).len() + self.tensor(n, "b", &[96, 3]).len();
            count += self.tensor(n + 1, "G", &[96]).len() + self.tensor(n + 1, "b", &[96]).len();
        }
        assert_eq!(count, 957494);
        assert_eq!(self.tensors.len(), 31);
    }
    pub fn forward(&self, buckets: &[[[u32; 4]; 4]]) -> Forward {
        self.forward_with_trace(buckets, true, None)
            .expect("unchecked inference")
    }
    /// Identical arithmetic without debug-stage tensor copies.
    pub fn forward_fast(&self, buckets: &[[[u32; 4]; 4]]) -> Forward {
        self.forward_with_trace(buckets, false, None)
            .expect("unchecked inference")
    }
    /// Bounded inference shares the exact numeric kernels with the proof API.
    pub fn forward_fast_bounded(
        &self,
        buckets: &[[[u32; 4]; 4]],
        control: &RequestControl<'_>,
    ) -> Result<Forward, InferenceError> {
        control.resource("tokens", buckets.len(), control.limits().tokens)?;
        // Conservative total bounds, including the four evolving padding layers.
        // Token limits make these checked constants safe on supported 64-bit hosts.
        let n = buckets.len();
        let padded = if n == 0 { 0 } else { n + 8 };
        control.ensure_work(n * 286_336 + padded * 672_000)?;
        control.ensure_tensor_bytes((n * 1_696 + padded * 2_784 + 384) * 4)?;
        self.forward_with_trace(buckets, false, Some(control))
    }
    fn forward_with_trace(
        &self,
        buckets: &[[[u32; 4]; 4]],
        trace: bool,
        control: Option<&RequestControl<'_>>,
    ) -> Result<Forward, InferenceError> {
        let n = buckets.len();
        let mut stages = BTreeMap::new();
        let mut embed = zeros(n * 384, control)?;
        for (f, (node, rows)) in [(32, 2000), (34, 1000), (36, 1000), (38, 1000)]
            .iter()
            .enumerate()
        {
            let e = self.tensor(*node, "E", &[*rows, 96]);
            for t in 0..n {
                charge(control, 4 * 96)?;
                for k in 0..4 {
                    let row = buckets[t][f][k] as usize % rows;
                    for j in 0..96 {
                        embed[t * 384 + f * 96 + j] += e[row * 96 + j];
                    }
                }
            }
        }
        if trace {
            stages.insert(
                "embedding_concat".into(),
                Tensor {
                    shape: vec![n, 384],
                    data: embed.clone(),
                },
            );
        }
        let mixed = self.maxnorm(&embed, n, 384, 39, control)?;
        if trace {
            stages.insert(
                "mixer".into(),
                Tensor {
                    shape: vec![n, 96],
                    data: mixed.clone(),
                },
            );
        }
        // with_array pads four rows BEFORE and AFTER each nonempty document.
        // These padding rows evolve through residual layers; do not zero them per layer.
        let padded = if n == 0 { 0 } else { n + 8 };
        let mut x = zeros(padded * 96, control)?;
        if n > 0 {
            x[4 * 96..(n + 4) * 96].copy_from_slice(&mixed);
        }
        for (layer, node) in [57, 59, 61, 63].iter().enumerate() {
            let mut window = zeros(padded * 288, control)?;
            for t in 0..padded {
                charge(control, 288)?;
                for pos in 0..3 {
                    let src = t as isize + pos as isize - 1;
                    if src >= 0 && (src as usize) < padded {
                        window[t * 288 + pos * 96..t * 288 + (pos + 1) * 96]
                            .copy_from_slice(&x[src as usize * 96..(src as usize + 1) * 96]);
                    }
                }
            }
            let y = self.maxnorm(&window, padded, 288, *node, control)?;
            for (xr, yr) in x.chunks_mut(96).zip(y.chunks(96)) {
                charge(control, xr.len())?;
                for (a, b) in xr.iter_mut().zip(yr) {
                    *a += *b;
                }
            }
            if trace {
                stages.insert(
                    format!("cnn_{layer}_padded"),
                    Tensor {
                        shape: vec![padded, 96],
                        data: x.clone(),
                    },
                );
            }
        }
        let encoded = if n == 0 {
            Vec::new()
        } else {
            copy_tensor(&x[384..(n + 4) * 96], control)?
        };
        if trace {
            stages.insert(
                "encoded".into(),
                Tensor {
                    shape: vec![n, 96],
                    data: encoded.clone(),
                },
            );
        }
        let projected = linear_inner(
            &encoded,
            n,
            96,
            64,
            self.tensor(6, "W", &[64, 96]),
            Some(self.tensor(6, "b", &[64])),
            control,
        )?;
        if trace {
            stages.insert(
                "projected".into(),
                Tensor {
                    shape: vec![n, 64],
                    data: projected.clone(),
                },
            );
        }
        let mut affine = zeros((n + 1) * 384, control)?;
        affine[..384].copy_from_slice(self.tensor(2, "pad", &[1, 3, 64, 2]));
        affine[384..].copy_from_slice(&linear_inner(
            &projected,
            n,
            64,
            384,
            self.tensor(2, "W", &[3, 64, 2, 64]),
            None,
            control,
        )?);
        if trace {
            stages.insert(
                "affine".into(),
                Tensor {
                    shape: vec![n + 1, 3, 64, 2],
                    data: affine.clone(),
                },
            );
        }
        checkpoint(control)?;
        Ok(Forward {
            stages,
            affine,
            n_tokens: n,
        })
    }
    fn maxnorm(
        &self,
        x: &[f32],
        n: usize,
        width: usize,
        node: usize,
        control: Option<&RequestControl<'_>>,
    ) -> Result<Vec<f32>, InferenceError> {
        let a = linear_inner(
            x,
            n,
            width,
            288,
            self.tensor(node, "W", &[96, 3, width]),
            Some(self.tensor(node, "b", &[96, 3])),
            control,
        )?;
        let mut y = zeros(n * 96, control)?;
        for i in 0..n * 96 {
            if i % 96 == 0 {
                charge(control, 96 * 3)?;
            }
            let mut best = a[i * 3];
            for p in 1..3 {
                if a[i * 3 + p] > best {
                    best = a[i * 3 + p];
                }
            }
            y[i] = best;
        }
        let gain = self.tensor(node + 1, "G", &[96]);
        let bias = self.tensor(node + 1, "b", &[96]);
        for row in y.chunks_exact_mut(96) {
            charge(control, 96 * 12)?;
            let mean = numpy_sum(row) / 96f32;
            let squared: [f32; 96] = std::array::from_fn(|j| {
                let d = row[j] - mean;
                d * d
            });
            let var = numpy_sum(&squared) / 96f32 + 1e-8f32;
            let inv = var.powf(-0.5f32);
            for j in 0..96 {
                row[j] = ((row[j] - mean) * inv) * gain[j] + bias[j];
            }
        }
        Ok(y)
    }
    pub fn logits(&self, forward: &Forward, context: [isize; 3]) -> Vec<f32> {
        self.logits_inner(forward, context, None)
            .expect("unchecked logits")
    }
    pub fn logits_bounded(
        &self,
        forward: &Forward,
        context: [isize; 3],
        control: &RequestControl<'_>,
    ) -> Result<Vec<f32>, InferenceError> {
        control.resource("tokens", forward.n_tokens, control.limits().tokens)?;
        if context
            .iter()
            .any(|&i| i < -1 || i >= forward.n_tokens as isize)
            || forward.affine.len() != (forward.n_tokens + 1) * 384
        {
            return Err(InferenceError::Model(
                "invalid forward context or tensor shape".into(),
            ));
        }
        control.ensure_work(4000)?;
        self.logits_inner(forward, context, Some(control))
    }
    fn logits_inner(
        &self,
        forward: &Forward,
        context: [isize; 3],
        control: Option<&RequestControl<'_>>,
    ) -> Result<Vec<f32>, InferenceError> {
        charge(control, 768)?;
        let mut hidden = [0f32; 128];
        for f in 0..3 {
            assert!(context[f] >= -1 && context[f] < (forward.n_tokens as isize));
            let row = (context[f] + 1) as usize;
            for j in 0..128 {
                hidden[j] += forward.affine[row * 384 + f * 128 + j];
            }
        }
        let bias = self.tensor(2, "b", &[64, 2]);
        for j in 0..128 {
            hidden[j] += bias[j];
        }
        let mut h = [0.; 64];
        for j in 0..64 {
            h[j] = if hidden[j * 2 + 1] > hidden[j * 2] {
                hidden[j * 2 + 1]
            } else {
                hidden[j * 2]
            };
        }
        linear_inner(
            &h,
            1,
            64,
            22,
            self.tensor(3, "W", &[22, 64]),
            Some(self.tensor(3, "b", &[22])),
            control,
        )
    }
}
pub struct Forward {
    pub stages: BTreeMap<String, Tensor>,
    pub affine: Vec<f32>,
    pub n_tokens: usize,
}
pub fn linear(
    x: &[f32],
    rows: usize,
    ni: usize,
    no: usize,
    w: &[f32],
    bias: Option<&[f32]>,
) -> Vec<f32> {
    linear_inner(x, rows, ni, no, w, bias, None).expect("unchecked linear")
}
/// Checked fixed-graph matrix helper. Dimensions outside the frozen graph fail closed.
pub fn linear_bounded(
    x: &[f32],
    rows: usize,
    ni: usize,
    no: usize,
    w: &[f32],
    bias: Option<&[f32]>,
    control: &RequestControl<'_>,
) -> Result<Vec<f32>, InferenceError> {
    control.resource("matrix_rows", rows, control.limits().tokens + 8)?;
    control.resource("matrix_input_width", ni, 384)?;
    control.resource("matrix_output_width", no, 384)?;
    if x.len() != rows * ni || w.len() != no * ni || bias.is_some_and(|b| b.len() != no) {
        return Err(InferenceError::Model("invalid linear tensor shape".into()));
    }
    control.ensure_work(rows * no * (ni * 2 + 1))?;
    linear_inner(x, rows, ni, no, w, bias, Some(control))
}
fn checkpoint(control: Option<&RequestControl<'_>>) -> Result<(), InferenceError> {
    if let Some(c) = control {
        c.checkpoint()?;
    }
    Ok(())
}
fn charge(control: Option<&RequestControl<'_>>, units: usize) -> Result<(), InferenceError> {
    if let Some(c) = control {
        c.charge(units)?;
    }
    Ok(())
}
fn zeros(len: usize, control: Option<&RequestControl<'_>>) -> Result<Vec<f32>, InferenceError> {
    if let Some(c) = control {
        let bytes =
            len.checked_mul(std::mem::size_of::<f32>())
                .ok_or(InferenceError::ResourceLimit {
                    resource: "tensor_bytes",
                    limit: c.limits().tensor_bytes,
                })?;
        c.tensor_bytes(bytes)?;
        let mut data = Vec::new();
        data.try_reserve_exact(len)
            .map_err(|_| InferenceError::Allocation { resource: "tensor" })?;
        data.resize(len, 0.);
        c.checkpoint()?;
        Ok(data)
    } else {
        Ok(vec![0.; len])
    }
}
fn copy_tensor(
    source: &[f32],
    control: Option<&RequestControl<'_>>,
) -> Result<Vec<f32>, InferenceError> {
    let mut data = zeros(source.len(), control)?;
    data.copy_from_slice(source);
    Ok(data)
}
fn linear_inner(
    x: &[f32],
    rows: usize,
    ni: usize,
    no: usize,
    w: &[f32],
    bias: Option<&[f32]>,
    control: Option<&RequestControl<'_>>,
) -> Result<Vec<f32>, InferenceError> {
    assert_eq!(x.len(), rows * ni);
    assert_eq!(w.len(), no * ni);
    let mut y = zeros(rows * no, control)?;
    // Zero-width matrices remain supported by this public helper.
    if ni == 0 {
        for r in 0..rows {
            charge(control, no)?;
            for o in 0..no {
                y[r * no + o] = 0f32 + bias.map_or(0., |b| b[o]);
            }
        }
        return Ok(y);
    }
    // Independent output accumulators expose instruction-level parallelism while
    // retaining the exact left-to-right f32 reduction order of every dot product.
    for (r, input) in x.chunks_exact(ni).enumerate() {
        let out = &mut y[r * no..(r + 1) * no];
        let blocks = no / 4 * 4;
        for o in (0..blocks).step_by(4) {
            charge(control, ni * 8 + 4)?;
            let w0 = &w[o * ni..(o + 1) * ni];
            let w1 = &w[(o + 1) * ni..(o + 2) * ni];
            let w2 = &w[(o + 2) * ni..(o + 3) * ni];
            let w3 = &w[(o + 3) * ni..(o + 4) * ni];
            let (mut a, mut b, mut c, mut d) = (0f32, 0f32, 0f32, 0f32);
            for i in 0..ni {
                a += input[i] * w0[i];
                b += input[i] * w1[i];
                c += input[i] * w2[i];
                d += input[i] * w3[i];
            }
            for (j, v) in [a, b, c, d].into_iter().enumerate() {
                out[o + j] = v + bias.map_or(0., |b| b[o + j]);
            }
        }
        for o in blocks..no {
            charge(control, ni * 2 + 1)?;
            let mut sum = 0f32;
            for (&v, &weight) in input.iter().zip(&w[o * ni..(o + 1) * ni]) {
                sum += v * weight;
            }
            out[o] = sum + bias.map_or(0., |b| b[o]);
        }
    }
    checkpoint(control)?;
    Ok(y)
}
// NumPy's contiguous float32 pairwise sum: 8 accumulators, cutoff 128.
fn numpy_sum(a: &[f32]) -> f32 {
    if a.len() < 8 {
        return a.iter().fold(-0f32, |s, &v| s + v);
    }
    if a.len() <= 128 {
        let mut r = [0f32; 8];
        r.copy_from_slice(&a[..8]);
        let mut i = 8;
        while i + 7 < a.len() {
            for k in 0..8 {
                r[k] += a[i + k];
            }
            i += 8;
        }
        let mut s = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        while i < a.len() {
            s += a[i];
            i += 1;
        }
        s
    } else {
        let half = (a.len() / 2) / 8 * 8;
        numpy_sum(&a[..half]) + numpy_sum(&a[half..])
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub is_space: bool,
    #[serde(default)]
    pub sent_start: Option<bool>,
    #[serde(default)]
    pub ent_iob: u8,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub label: String,
    pub text: String,
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub cursor: usize,
    pub open: Option<(usize, usize)>,
}
impl State {
    pub fn context(&self, n: usize) -> [isize; 3] {
        if self.cursor >= n {
            return [-1, -1, -1];
        }
        match self.open {
            None => [self.cursor as isize, -1, -1],
            Some((start, _)) => [
                self.cursor as isize,
                start as isize,
                self.cursor as isize - 1,
            ],
        }
    }
    pub fn valid(&self, tokens: &[Token]) -> [bool; 22] {
        let mut valid = [false; 22];
        if self.cursor >= tokens.len() {
            return valid;
        }
        let tok = &tokens[self.cursor];
        let can_continue =
            self.cursor + 1 < tokens.len() && tokens[self.cursor + 1].sent_start != Some(true);
        if let Some((_, label)) = self.open {
            valid[2 + label * 4 + 1] = can_continue;
            valid[2 + label * 4 + 2] = true;
        } else {
            valid[1] = true;
            for label in 0..5 {
                valid[2 + label * 4] = !tok.is_space && can_continue;
                valid[2 + label * 4 + 3] = !tok.is_space;
            }
        }
        valid
    }
    pub fn apply(
        &mut self,
        action: usize,
        tokens: &[Token],
        text: &str,
        spans: &mut Vec<Span>,
    ) -> Result<(), String> {
        if action >= 22 || !self.valid(tokens)[action] {
            return Err(format!("invalid action {action} at {}", self.cursor));
        }
        if action >= 2 {
            let label = (action - 2) / 4;
            match (action - 2) % 4 {
                0 => self.open = Some((self.cursor, label)),
                1 => {}
                2 | 3 => {
                    let start = if (action - 2) % 4 == 3 {
                        self.cursor
                    } else {
                        self.open.take().unwrap().0
                    };
                    let start = tokens[start].start;
                    let end = tokens[self.cursor].end;
                    let chars: Vec<char> = text.chars().collect();
                    if start > end || end > chars.len() {
                        return Err("invalid codepoint offsets".into());
                    }
                    spans.push(Span {
                        start,
                        end,
                        label: LABELS[label].into(),
                        text: chars[start..end].iter().collect(),
                    });
                }
                _ => unreachable!(),
            }
        }
        self.cursor += 1;
        Ok(())
    }
}
pub fn choose(scores: &[f32], valid: &[bool; 22]) -> Result<usize, String> {
    if scores.len() != 22 || scores.iter().any(|s| !s.is_finite()) {
        return Err("expected 22 finite logits".into());
    }
    let mut best = None;
    for i in 0..22 {
        if valid[i] && best.is_none_or(|b| scores[i] > scores[b]) {
            best = Some(i);
        }
    }
    best.ok_or_else(|| "no valid transition".into())
}
pub fn validate_tokens(tokens: &[Token]) -> Result<(), String> {
    if tokens.iter().any(|t| t.ent_iob != 0) {
        return Err("preset entities unsupported: fresh raw-text docs only".into());
    }
    for (i, t) in tokens.iter().enumerate() {
        if t.start >= t.end || (i > 0 && tokens[i - 1].end > t.start) {
            return Err("invalid token offsets".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn tokens() -> Vec<Token> {
        vec![
            Token {
                start: 0,
                end: 1,
                is_space: false,
                sent_start: Some(true),
                ent_iob: 0,
            },
            Token {
                start: 1,
                end: 2,
                is_space: true,
                sent_start: None,
                ent_iob: 0,
            },
            Token {
                start: 2,
                end: 3,
                is_space: false,
                sent_start: None,
                ent_iob: 0,
            },
        ]
    }
    #[test]
    fn tie_first() {
        assert_eq!(
            choose(&[0.; 22], &State::default().valid(&tokens())).unwrap(),
            1
        );
    }
    #[test]
    fn all_labels_open_close() {
        let ts = tokens();
        for label in 0..5 {
            let mut s = State::default();
            let mut spans = vec![];
            s.apply(2 + label * 4, &ts, "a b", &mut spans).unwrap();
            assert_eq!(s.context(3), [1, 0, 0]);
            for other in 0..5 {
                assert_eq!(s.valid(&ts)[3 + other * 4], other == label);
                assert_eq!(s.valid(&ts)[4 + other * 4], other == label);
            }
            s.apply(3 + label * 4, &ts, "a b", &mut spans).unwrap();
            assert!(!s.valid(&ts)[3 + label * 4]);
            s.apply(4 + label * 4, &ts, "a b", &mut spans).unwrap();
            assert_eq!(spans[0].text, "a b");
            assert_eq!(s.context(3), [-1, -1, -1]);
            assert!(!s.valid(&ts).iter().any(|v| *v));
        }
    }
    #[test]
    fn blocked_and_space() {
        let ts = tokens();
        let mut s = State::default();
        assert!(!s.valid(&ts)[0]);
        s.cursor = 1;
        let v = s.valid(&ts);
        assert_eq!(v.iter().filter(|&&x| x).count(), 1);
        assert!(v[1]);
        s.cursor = 2;
        for l in 0..5 {
            assert!(!s.valid(&ts)[2 + l * 4]);
            assert!(s.valid(&ts)[5 + l * 4]);
        }
    }
    #[test]
    fn sentence_boundary() {
        let mut ts = tokens();
        ts[1].sent_start = Some(true);
        assert!(!State::default().valid(&ts)[2]);
    }
    #[test]
    fn reject_preset() {
        let mut ts = tokens();
        ts[0].ent_iob = 3;
        assert!(validate_tokens(&ts).is_err());
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;
    #[test]
    fn sum_is_scalar_order_and_bias_last() {
        assert_eq!(
            linear(&[1e20, 1., -1e20], 1, 3, 1, &[1., 1., 1.], Some(&[2.])),
            vec![2.]
        );
    }
    #[test]
    fn null_sentence_start_and_unicode_span() {
        let tokens: Vec<Token> = serde_json::from_str(
            r#"[{"start":0,"end":1,"is_space":false,"sent_start":null,"ent_iob":0}]"#,
        )
        .unwrap();
        let mut s = State::default();
        let mut spans = vec![];
        assert!(s.apply(2, &tokens, "🎵", &mut spans).is_err());
        s.apply(5, &tokens, "🎵", &mut spans).unwrap();
        assert_eq!(
            spans[0],
            Span {
                start: 0,
                end: 1,
                label: "SINGER".into(),
                text: "🎵".into()
            }
        );
    }
    #[test]
    fn nonfinite_logits_rejected() {
        let mut scores = [0.; 22];
        scores[2] = f32::NAN;
        assert!(choose(&scores, &[true; 22]).is_err());
        assert!(choose(&[0.; 22], &[false; 22]).is_err());
    }
}

#[cfg(test)]
mod optimized_linear_tests {
    use super::linear;
    #[test]
    fn interleaving_matches_scalar_bits_for_shapes_and_cancellation() {
        for rows in [0, 1, 3] {
            for ni in [0, 1, 3, 64, 96, 288, 384] {
                for no in [0, 1, 3, 4, 5, 22, 64, 288, 384] {
                    let x: Vec<f32> = (0..rows * ni)
                        .map(|i| [1e20, 1., -1e20, -0., 0.25, -0.125, 3.][i % 7])
                        .collect();
                    let w: Vec<f32> = (0..no * ni)
                        .map(|i| ((i * 17 % 23) as f32 - 11.) / 8.)
                        .collect();
                    let b: Vec<f32> = (0..no).map(|i| i as f32 / 16.).collect();
                    for bias in [None, Some(b.as_slice())] {
                        let actual = linear(&x, rows, ni, no, &w, bias);
                        for r in 0..rows {
                            for o in 0..no {
                                let mut sum = 0f32;
                                for i in 0..ni {
                                    sum += x[r * ni + i] * w[o * ni + i];
                                }
                                sum += bias.map_or(0., |b| b[o]);
                                assert_eq!(
                                    actual[r * no + o].to_bits(),
                                    sum.to_bits(),
                                    "shape {rows} {ni} {no}, output {r} {o}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod bounded_tests {
    use super::*;

    #[test]
    fn lower_limits_and_monotonic_cancellation() {
        let mut limits = Limits::default();
        limits.work_units = 7;
        limits.regex_calls = 1;
        limits.regex_callouts = 1;
        let token = CancellationToken::new();
        let c = RequestControl::new(&limits, &token).unwrap();
        c.charge(7).unwrap();
        assert_eq!(
            c.charge(1),
            Err(InferenceError::ResourceLimit {
                resource: "work_units",
                limit: 7
            })
        );
        assert_eq!(c.work_used(), 7);
        c.charge_regex_call().unwrap();
        assert!(c.charge_regex_call().is_err());
        c.charge_regex_callout().unwrap();
        assert!(c.charge_regex_callout().is_err());
        token.cancel();
        token.cancel();
        assert_eq!(c.checkpoint(), Err(InferenceError::Cancelled));
        assert!(matches!(
            RequestControl::new(&limits, &token),
            Err(InferenceError::Cancelled)
        ));
    }

    #[test]
    fn hard_max_and_overflow_fail_closed() {
        let token = CancellationToken::new();
        let mut limits = Limits::default();
        limits.tokens = usize::MAX;
        assert!(matches!(
            RequestControl::new(&limits, &token),
            Err(InferenceError::ResourceLimit {
                resource: "tokens",
                ..
            })
        ));
        let limits = Limits::default();
        let c = RequestControl::new(&limits, &token).unwrap();
        assert!(c.charge(usize::MAX).is_err());
        assert_eq!(c.work_used(), 0);
        assert!(linear_bounded(&[], usize::MAX, usize::MAX, usize::MAX, &[], None, &c).is_err());
        assert!(linear_bounded(&[], 1, 1, 1, &[1.], None, &c).is_err());
        assert!(linear_bounded(&[1.], 1, 1, 1, &[1.], Some(&[]), &c).is_err());
    }

    #[test]
    fn checked_linear_matches_every_bit() {
        let limits = Limits::default();
        let token = CancellationToken::new();
        for rows in [0, 1, 3] {
            for ni in [0, 1, 3, 64, 96, 288, 384] {
                for no in [0, 1, 3, 4, 5, 22, 64, 288, 384] {
                    let c = RequestControl::new(&limits, &token).unwrap();
                    let x: Vec<f32> = (0..rows * ni)
                        .map(|i| [1e20, 1., -1e20, -0., 0.25][i % 5])
                        .collect();
                    let w: Vec<f32> = (0..no * ni).map(|i| (i % 7) as f32 / 8.).collect();
                    let b: Vec<f32> = (0..no).map(|i| i as f32 / 16.).collect();
                    for bias in [None, Some(b.as_slice())] {
                        let checked = linear_bounded(&x, rows, ni, no, &w, bias, &c).unwrap();
                        let expected = linear(&x, rows, ni, no, &w, bias);
                        assert_eq!(
                            checked.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                            expected.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn cancel_during_four_output_blocks_and_reuse_fresh_request() {
        let limits = Limits::default();
        let token = CancellationToken::new();
        let c = RequestControl::new(&limits, &token).unwrap();
        c.cancel_after_checkpoints(12);
        let x = vec![1.; 384];
        let w = vec![1.; 384 * 384];
        assert!(matches!(
            linear_bounded(&x, 1, 384, 384, &w, None, &c),
            Err(InferenceError::Cancelled)
        ));
        assert!(c.work_used() > 0 && c.work_used() < 384 * 769);
        let token = CancellationToken::new();
        let c = RequestControl::new(&limits, &token).unwrap();
        assert_eq!(
            linear_bounded(&[2.], 1, 1, 1, &[3.], None, &c).unwrap(),
            vec![6.]
        );
    }

    #[test]
    #[ignore = "requires ORIGINAL_NER_EXPORT verified original assets"]
    fn cancel_inside_forward_then_reuse_model() {
        let dir = std::env::var_os("ORIGINAL_NER_EXPORT").expect("ORIGINAL_NER_EXPORT");
        let model = Model::load(std::path::Path::new(&dir)).unwrap();
        let limits = Limits::default();
        let token = CancellationToken::new();
        let c = RequestControl::new(&limits, &token).unwrap();
        c.cancel_after_checkpoints(100);
        let buckets = [[[7; 4]; 4]; 17];
        assert!(matches!(
            model.forward_fast_bounded(&buckets, &c),
            Err(InferenceError::Cancelled)
        ));
        assert!(c.work_used() > 0);
        assert!(c.tensor_bytes_used() > 0);
        let token = CancellationToken::new();
        let c = RequestControl::new(&limits, &token).unwrap();
        let actual = model.forward_fast_bounded(&buckets, &c).unwrap();
        let expected = model.forward_fast(&buckets);
        assert_eq!(
            actual
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
        let limits = Limits {
            work_units: 1,
            ..Limits::default()
        };
        let c = RequestControl::new(&limits, &token).unwrap();
        assert!(matches!(
            model.forward_fast_bounded(&buckets, &c),
            Err(InferenceError::ResourceLimit {
                resource: "work_units",
                ..
            })
        ));
        assert_eq!(c.tensor_bytes_used(), 0);
        let limits = Limits {
            tensor_bytes: 1,
            ..Limits::default()
        };
        let c = RequestControl::new(&limits, &token).unwrap();
        assert!(matches!(
            model.forward_fast_bounded(&buckets, &c),
            Err(InferenceError::ResourceLimit {
                resource: "tensor_bytes",
                ..
            })
        ));
        assert_eq!(c.tensor_bytes_used(), 0);
    }

    #[test]
    fn reject_work_and_tensor_before_allocating() {
        let token = CancellationToken::new();
        let mut limits = Limits::default();
        limits.work_units = 2;
        let c = RequestControl::new(&limits, &token).unwrap();
        assert!(matches!(
            linear_bounded(&[1.], 1, 1, 1, &[2.], None, &c),
            Err(InferenceError::ResourceLimit {
                resource: "work_units",
                ..
            })
        ));
        assert_eq!(c.tensor_bytes_used(), 0);
        limits.work_units = 3;
        limits.tensor_bytes = 3;
        let c = RequestControl::new(&limits, &token).unwrap();
        assert!(matches!(
            linear_bounded(&[1.], 1, 1, 1, &[2.], None, &c),
            Err(InferenceError::ResourceLimit {
                resource: "tensor_bytes",
                ..
            })
        ));
        assert_eq!(c.tensor_bytes_used(), 0);
    }
}
