use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub index: usize,
    pub title: String,
    pub url: String,
    pub domain: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Identifying,
    Searching,
    Insights,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub query: String,
    pub phase: Phase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawHtml {
    #[serde(rename = "answerHtml")]
    pub answer_html: String,
    #[serde(rename = "sourcesHtml")]
    pub sources_html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerplexityResult {
    pub answer: String,
    pub sources: Vec<Source>,
    #[serde(rename = "threadId")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<Step>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<RawHtml>,
    #[serde(rename = "jobId", skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}
