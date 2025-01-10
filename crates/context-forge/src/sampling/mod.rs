use std::future::Future;
use mcp_schema::{
    SamplingMessage, CreateMessageParams, CreateMessageResult,
    ModelPreferences, ModelHint, Role, SamplingContent,
};
use async_trait::async_trait;

use crate::Result;

/// Stop reason when sampling is complete.
#[derive(Debug, Clone, PartialEq)]
pub enum StopReason {
    /// Message signifies end of turn
    EndTurn,
    /// A stop sequence was matched
    StopSequence,
    /// Maximum tokens reached
    MaxTokens,
    /// Custom stop reason
    Custom(String),
}

impl From<StopReason> for String {
    fn from(reason: StopReason) -> Self {
        match reason {
            StopReason::EndTurn => "endTurn".to_string(),
            StopReason::StopSequence => "stopSequence".to_string(),
            StopReason::MaxTokens => "maxTokens".to_string(),
            StopReason::Custom(s) => s,
        }
    }
}

/// Information about a model that can be used for sampling.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Name of the model
    pub name: String,
    /// Relative cost of using this model (0.0 - 1.0)
    pub cost: f64,
    /// Relative speed of this model (0.0 - 1.0)
    pub speed: f64,
    /// Relative intelligence/capability of this model (0.0 - 1.0)
    pub intelligence: f64,
    /// Maximum context length in tokens
    pub max_tokens: usize,
}

/// A sampled response that is still in progress.
#[derive(Debug)]
pub struct PendingResponse {
    /// Model that generated this response
    pub model: String,
    /// Current content
    pub content: SamplingContent,
    /// Whether this is the final response
    pub is_final: bool,
    /// Stop reason if this is the final response
    pub stop_reason: Option<StopReason>,
}

/// A trait for systems that can sample from language models.
#[async_trait]
pub trait Sampler: Send + Sync {
    /// List available models that match the given preferences.
    fn list_models(&self, preferences: Option<&ModelPreferences>) -> Vec<ModelInfo>;

    /// Select a model based on preferences. Returns name of selected model.
    fn select_model(&self, preferences: Option<&ModelPreferences>) -> Result<String>;

    /// Sample a response from the given model with the provided parameters.
    ///
    /// Returns a stream of pending responses, ending with a final response that has
    /// is_final set to true.
    async fn sample<'a>(
        &'a self,
        model: &str,
        messages: &'a [SamplingMessage],
        max_tokens: i64,
        temperature: Option<f64>,
        stop_sequences: Option<&'a [String]>,
    ) -> Result<Box<dyn Stream<Item = Result<PendingResponse>> + Send + 'a>>;
}

/// A stream trait for pending responses.
pub trait Stream {
    type Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
}

/// A default implementation of model selection logic.
pub fn select_model_by_preferences(
    models: &[ModelInfo],
    preferences: Option<&ModelPreferences>,
) -> Result<String> {
    let preferences = match preferences {
        Some(p) => p,
        None => return Ok(models[0].name.clone()), // Default to first model if no preferences
    };

    // First try to match by hints if provided
    if let Some(hints) = &preferences.hints {
        for hint in hints {
            if let Some(name) = &hint.name {
                // Try exact match first
                if let Some(model) = models.iter().find(|m| m.name == *name) {
                    return Ok(model.name.clone());
                }
                
                // Try substring match
                if let Some(model) = models.iter().find(|m| m.name.contains(name)) {
                    return Ok(model.name.clone());
                }
            }
        }
    }

    // No hint matches, score models based on priorities
    let mut scored_models: Vec<_> = models
        .iter()
        .map(|m| {
            let mut score = 0.0;
            
            if let Some(p) = preferences.cost_priority {
                score += (1.0 - m.cost) * p; // Inverse cost for scoring
            }
            
            if let Some(p) = preferences.speed_priority {
                score += m.speed * p;
            }
            
            if let Some(p) = preferences.intelligence_priority {
                score += m.intelligence * p;
            }
            
            (m, score)
        })
        .collect();

    // Sort by score descending
    scored_models.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

    // Return highest scoring model
    Ok(scored_models[0].0.name.clone())
}
