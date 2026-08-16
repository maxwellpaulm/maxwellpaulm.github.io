pub mod engine;
pub mod text;

use wasm_bindgen::prelude::*;

/// The `/ask/` page's handle: constructed once from the fetched
/// `/ask/index.json`, then queried per keystroke-free submission.
#[wasm_bindgen]
pub struct Terminal {
    engine: engine::Engine,
}

#[wasm_bindgen]
impl Terminal {
    #[wasm_bindgen(constructor)]
    pub fn new(index_json: &str) -> Result<Terminal, JsError> {
        let engine = engine::Engine::new(index_json).map_err(|e| JsError::new(&e))?;
        Ok(Terminal { engine })
    }

    /// Returns the response as JSON — rendering is terminal.js's job.
    pub fn ask(&self, query: &str) -> String {
        serde_json::to_string(&self.engine.ask(query))
            .unwrap_or_else(|_| r#"{"kind":"miss","suggest":[]}"#.to_string())
    }
}
