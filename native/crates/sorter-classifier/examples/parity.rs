use serde_json::json;
use sorter_classifier::{Limits, Model};
use std::{
    io::{self, BufRead},
    path::Path,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args().nth(1).ok_or("Expected asset directory")?;
    if std::env::args().nth(2).is_some() {
        return Err("Only original classifier supported".into());
    }
    let model = Model::load_directory(Path::new(&dir), &|| false)?;
    for line in io::stdin().lock().lines() {
        let row: serde_json::Value = serde_json::from_str(&line?)?;
        let p = model.predict(
            row["text"].as_str().ok_or("Expected text")?,
            Limits::default(),
            &|| false,
        )?;
        println!(
            "{}",
            json!({"predicted_class":p.class,"accepted":p.class==0,"probabilities":p.probabilities,"scores":p.scores,"analyzed_features":p.features,"sparse_tfidf":p.sparse,"margin_top_two":p.margin})
        );
    }
    Ok(())
}
