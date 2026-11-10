//! Isolated full-input inference boundary. No product/CLI activation.
//!
//! The legacy diagnostic helpers remain unbounded and must not be used as this
//! boundary. Resource errors reject the whole request, never truncate/fallback.
use crate::{assets::ReleasedAssets, features, is_space};
pub use released_ner_probe::control::{CancellationToken, InferenceError, Limits, RequestControl};
use released_ner_probe::{choose, validate_tokens, Span, State, Token};

#[derive(Debug)]
pub struct InferenceOutput {
    pub tokens: Vec<Token>,
    pub actions: Vec<usize>,
    pub spans: Vec<Span>,
    pub margins: Vec<Option<f32>>,
}
fn reserved<T>(count: usize, resource: &'static str) -> Result<Vec<T>, InferenceError> {
    let mut value = Vec::new();
    value
        .try_reserve_exact(count)
        .map_err(|_| InferenceError::Allocation { resource })?;
    Ok(value)
}
impl ReleasedAssets {
    /// All limits are required explicitly; no implicit unrestricted mode exists.
    /// Cancellation is monotonic and observed before successful publication.
    /// A request linearizes at the final checkpoint; cancellation after that
    /// point belongs to the caller's result-delivery/generation gate.
    pub fn infer(
        &self,
        text: &str,
        limits: &Limits,
        cancellation: &CancellationToken,
    ) -> Result<InferenceOutput, InferenceError> {
        let control = RequestControl::new(limits, cancellation)?;
        control.resource("input bytes", text.len(), limits.input_bytes)?;
        control.resource("input characters", text.chars().count(), limits.input_chars)?;
        let text_tokens = self.tokenizer().tokenize_bounded(text, &control)?;
        control.resource("tokens", text_tokens.len(), limits.tokens)?;
        let mut tokens = reserved(text_tokens.len(), "decoder tokens")?;
        let mut hashes = reserved(text_tokens.len(), "token hashes")?;
        for t in &text_tokens {
            let n = t.text.chars().count();
            // Frozen final-sigma lowercasing can inspect both surrounding slices.
            // Precharge its conservative quadratic bound without changing rules.
            control.charge(n.checked_mul(n + 1).ok_or(InferenceError::ResourceLimit {
                resource: "work units",
                limit: limits.work_units,
            })?)?;
            tokens.push(Token {
                start: t.offset,
                end: t.offset + n,
                is_space: !t.text.is_empty() && t.text.chars().all(is_space),
                sent_start: t.sent_start,
                ent_iob: 0,
            });
            hashes.push(features(&t.text, t.norm_override.as_deref()).hashes);
            control.checkpoint()?;
        }
        validate_tokens(&tokens).map_err(InferenceError::Model)?;
        #[cfg(test)]
        FEATURE_BOUNDARY_GATE.with(|gate| {
            if let Some((started, cancelled)) = gate.borrow().as_ref() {
                started
                    .send(())
                    .expect("test cancellation observer disconnected");
                cancelled
                    .recv_timeout(std::time::Duration::from_secs(30))
                    .expect("test cancellation acknowledgment missing");
            }
        });
        let forward = self.model().forward_fast_bounded(&hashes, &control)?;
        let mut state = State::default();
        let mut spans = reserved(tokens.len(), "output spans")?;
        let mut actions = reserved(tokens.len(), "output actions")?;
        let mut margins = reserved(tokens.len(), "output margins")?;
        while state.cursor < tokens.len() {
            control.charge(text.len() + 22)?;
            let valid = state.valid(&tokens);
            let logits =
                self.model()
                    .logits_bounded(&forward, state.context(tokens.len()), &control)?;
            let action = choose(&logits, &valid).map_err(InferenceError::Model)?;
            let other = logits
                .iter()
                .enumerate()
                .filter(|&(i, _)| valid[i] && i != action)
                .map(|(_, v)| *v)
                .max_by(f32::total_cmp);
            margins.push(other.map(|v| logits[action] - v));
            actions.push(action);
            state
                .apply(action, &tokens, text, &mut spans)
                .map_err(InferenceError::Model)?;
        }
        // No partial tokens, actions or spans escape on any error/cancellation.
        control.checkpoint()?;
        Ok(InferenceOutput {
            tokens,
            actions,
            spans,
            margins,
        })
    }
}

#[cfg(test)]
thread_local! {
    static FEATURE_BOUNDARY_GATE: std::cell::RefCell<Option<(std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>)>> = const { std::cell::RefCell::new(None) };
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires verified ORIGINAL_NER_EXPORT"]
    fn asynchronous_cancel_during_request_and_fresh_request_reuse() {
        use std::sync::mpsc::channel;
        let dir = std::env::var_os("ORIGINAL_NER_EXPORT").unwrap();
        let assets = ReleasedAssets::load(std::path::Path::new(&dir)).unwrap();
        let limits = Limits::default();
        for _ in 0..3 {
            let token = CancellationToken::new();
            let (started_tx, started_rx) = channel();
            let (cancelled_tx, cancelled_rx) = channel();
            FEATURE_BOUNDARY_GATE
                .with(|gate| *gate.borrow_mut() = Some((started_tx, cancelled_rx)));
            let result = std::thread::scope(|scope| {
                let token_ref = &token;
                scope.spawn(move || {
                    started_rx
                        .recv_timeout(std::time::Duration::from_secs(30))
                        .expect("request did not reach feature boundary");
                    token_ref.cancel();
                    cancelled_tx
                        .send(())
                        .expect("request disconnected before cancellation");
                });
                assets.infer("אבג 🎵 שיר", &limits, &token)
            });
            FEATURE_BOUNDARY_GATE.with(|gate| *gate.borrow_mut() = None);
            assert!(matches!(result, Err(InferenceError::Cancelled)));
            assert!(assets
                .infer("אבג 🎵 שיר", &limits, &CancellationToken::new())
                .is_ok());
        }
    }
}
