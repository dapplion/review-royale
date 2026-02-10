//! AI-powered review comment categorization
//!
//! Categories:
//! - `cosmetic`: Style, formatting, naming conventions
//! - `logic`: Bug fixes, correctness issues, edge cases
//! - `structural`: Architecture, design patterns, refactoring
//! - `nit`: Minor suggestions, nice-to-haves
//! - `question`: Clarifying questions, understanding requests
//!
//! Quality score (1-10):
//! - 1-3: Brief/superficial comments
//! - 4-6: Standard helpful feedback
//! - 7-10: Detailed, insightful, educational

use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
use tracing::{info, warn};
use uuid::Uuid;

/// Comment categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Cosmetic,
    Logic,
    Structural,
    Nit,
    Question,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cosmetic => "cosmetic",
            Self::Logic => "logic",
            Self::Structural => "structural",
            Self::Nit => "nit",
            Self::Question => "question",
        }
    }
}

/// Result of categorizing a single comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorizedComment {
    pub category: Category,
    pub quality_score: i32,
}

/// Batch categorization result
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchResult {
    results: Vec<CommentClassification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommentClassification {
    index: usize,
    category: Category,
    quality_score: i32,
}

#[derive(Error, Debug)]
pub enum CategorizeError {
    #[error("OpenAI API key not configured")]
    NoApiKey,
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Failed to parse AI response: {0}")]
    Parse(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Stats from a categorization run
#[derive(Debug, Clone, Default)]
pub struct CategorizeStats {
    pub processed: usize,
    pub skipped: usize,
    pub errors: usize,
}

/// OpenAI chat message
#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI chat request
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    response_format: ResponseFormat,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

/// OpenAI chat response
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: String,
}

const SYSTEM_PROMPT: &str = r#"You are a code review comment classifier. Analyze each review comment and classify it.

Categories:
- cosmetic: Style, formatting, naming conventions, typos
- logic: Bug fixes, correctness issues, edge cases, error handling
- structural: Architecture, design patterns, refactoring, code organization
- nit: Minor suggestions, nice-to-haves, opinions
- question: Clarifying questions, understanding requests

Quality score (1-10):
- 1-3: Brief/superficial (e.g., "nit: typo", "LGTM")
- 4-6: Standard helpful feedback with clear reasoning
- 7-10: Detailed, insightful, educational, catches subtle bugs

Respond with valid JSON only. Format:
{
  "results": [
    {"index": 0, "category": "logic", "quality_score": 7},
    {"index": 1, "category": "nit", "quality_score": 3}
  ]
}
"#;

/// Categorize uncategorized comments using AI
pub async fn categorize_batch(
    pool: &PgPool,
    api_key: &str,
    batch_size: usize,
) -> Result<CategorizeStats, CategorizeError> {
    let client = Client::new();
    let mut stats = CategorizeStats::default();

    // Fetch uncategorized comments
    let comments = fetch_uncategorized(pool, batch_size).await?;
    if comments.is_empty() {
        info!("No uncategorized comments to process");
        return Ok(stats);
    }

    info!("Processing {} uncategorized comments", comments.len());

    // Build prompt with comment bodies
    let mut user_content = String::from("Classify these code review comments:\n\n");
    for (i, (_, body, _)) in comments.iter().enumerate() {
        // Truncate very long comments
        let truncated = if body.len() > 500 {
            format!("{}...", &body[..500])
        } else {
            body.clone()
        };
        user_content.push_str(&format!("[{}] {}\n\n", i, truncated));
    }

    // Call OpenAI
    let request = ChatRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: SYSTEM_PROMPT.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ],
        temperature: 0.3,
        response_format: ResponseFormat {
            format_type: "json_object".to_string(),
        },
    };

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(CategorizeError::Parse(format!(
            "OpenAI API error {}: {}",
            status, body
        )));
    }

    let chat_response: ChatResponse = response.json().await?;
    let content = chat_response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    // Parse the JSON response
    let batch_result: BatchResult = serde_json::from_str(&content)
        .map_err(|e| CategorizeError::Parse(format!("JSON parse error: {} - content: {}", e, content)))?;

    // Update database
    for classification in batch_result.results {
        if classification.index >= comments.len() {
            warn!("Invalid index {} in AI response", classification.index);
            stats.errors += 1;
            continue;
        }

        let (id, _, _) = &comments[classification.index];
        let quality = classification.quality_score.clamp(1, 10);

        match db::review_comments::set_category(
            pool,
            *id,
            classification.category.as_str(),
            quality,
        )
        .await
        {
            Ok(_) => stats.processed += 1,
            Err(e) => {
                warn!("Failed to update comment {}: {}", id, e);
                stats.errors += 1;
            }
        }
    }

    // Count skipped (no classification returned)
    stats.skipped = comments.len().saturating_sub(stats.processed + stats.errors);

    info!(
        "Categorization complete: {} processed, {} skipped, {} errors",
        stats.processed, stats.skipped, stats.errors
    );

    Ok(stats)
}

/// Fetch uncategorized comments from database
async fn fetch_uncategorized(
    pool: &PgPool,
    limit: usize,
) -> Result<Vec<(Uuid, String, Option<String>)>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"
        SELECT id, body, diff_hunk
        FROM review_comments
        WHERE category IS NULL
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.get::<Uuid, _>("id"),
                row.get::<String, _>("body"),
                row.get::<Option<String>, _>("diff_hunk"),
            )
        })
        .collect())
}

/// Get categorization statistics
pub async fn get_stats(pool: &PgPool) -> Result<CategoryStats, sqlx::Error> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE category IS NOT NULL) as categorized,
            COUNT(*) FILTER (WHERE category = 'cosmetic') as cosmetic,
            COUNT(*) FILTER (WHERE category = 'logic') as logic,
            COUNT(*) FILTER (WHERE category = 'structural') as structural,
            COUNT(*) FILTER (WHERE category = 'nit') as nit,
            COUNT(*) FILTER (WHERE category = 'question') as question,
            AVG(quality_score) FILTER (WHERE quality_score IS NOT NULL) as avg_quality
        FROM review_comments
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(CategoryStats {
        total: row.get::<i64, _>("total") as usize,
        categorized: row.get::<i64, _>("categorized") as usize,
        by_category: CategoryBreakdown {
            cosmetic: row.get::<i64, _>("cosmetic") as usize,
            logic: row.get::<i64, _>("logic") as usize,
            structural: row.get::<i64, _>("structural") as usize,
            nit: row.get::<i64, _>("nit") as usize,
            question: row.get::<i64, _>("question") as usize,
        },
        avg_quality: row.get::<Option<f64>, _>("avg_quality").unwrap_or(0.0),
    })
}

/// Overall categorization statistics
#[derive(Debug, Clone, Serialize)]
pub struct CategoryStats {
    pub total: usize,
    pub categorized: usize,
    pub by_category: CategoryBreakdown,
    pub avg_quality: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryBreakdown {
    pub cosmetic: usize,
    pub logic: usize,
    pub structural: usize,
    pub nit: usize,
    pub question: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_as_str() {
        assert_eq!(Category::Logic.as_str(), "logic");
        assert_eq!(Category::Cosmetic.as_str(), "cosmetic");
        assert_eq!(Category::Structural.as_str(), "structural");
        assert_eq!(Category::Nit.as_str(), "nit");
        assert_eq!(Category::Question.as_str(), "question");
    }

    #[test]
    fn test_parse_batch_result() {
        let json = r#"{
            "results": [
                {"index": 0, "category": "logic", "quality_score": 7},
                {"index": 1, "category": "nit", "quality_score": 3}
            ]
        }"#;

        let result: BatchResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].category, Category::Logic);
        assert_eq!(result.results[0].quality_score, 7);
    }
}
