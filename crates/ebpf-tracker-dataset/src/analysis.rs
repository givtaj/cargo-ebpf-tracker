use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Serialize;
use serde_json::{json, Value};

const DEFAULT_LM_STUDIO_ENDPOINT: &str = "http://127.0.0.1:1234/v1";
const DEFAULT_LM_STUDIO_MODEL: &str = "qwen/qwen3.5-9b";
const DEFAULT_ANALYSIS_TEMPERATURE: f32 = 0.2;
const DEFAULT_ANALYSIS_MAX_TOKENS: u32 = 900;
const DEFAULT_EVENT_SAMPLE_COUNT: usize = 12;
const DEFAULT_PROMPT_INSTRUCTIONS: &str = "Analyze this eBPF_tracker dataset as a test-learning artifact. Focus on what likely happened in the app, how much of the trace is tooling noise, any anomalies or regressions, and the next concrete follow-up steps. Reply in markdown with these sections: Summary, App Signal, Tooling Noise, Anomalies, Next Steps.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelProvider {
    LmStudio,
    OpenAiCompatible,
}

impl ModelProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            ModelProvider::LmStudio => "lm-studio",
            ModelProvider::OpenAiCompatible => "openai-compatible",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "lm-studio" => Ok(Self::LmStudio),
            "openai-compatible" | "openai" => Ok(Self::OpenAiCompatible),
            _ => Err(format!("unsupported model provider: {raw}")),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnalyzeConfig {
    pub run_dir: PathBuf,
    pub provider: ModelProvider,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub instructions_path: Option<PathBuf>,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            run_dir: PathBuf::new(),
            provider: ModelProvider::LmStudio,
            endpoint: None,
            model: None,
            api_key: None,
            temperature: DEFAULT_ANALYSIS_TEMPERATURE,
            max_tokens: Some(DEFAULT_ANALYSIS_MAX_TOKENS),
            instructions_path: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyzeSummary {
    pub provider: ModelProvider,
    pub model: String,
    pub output_markdown: PathBuf,
    pub output_json: PathBuf,
}

struct ModelRequest {
    system_prompt: String,
    user_prompt: String,
    temperature: f32,
    max_tokens: Option<u32>,
}

struct ModelResponse {
    model: String,
    text: String,
    raw_json: Value,
}

trait AnalysisModel {
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, String>;
}

struct OpenAiCompatibleAdapter {
    provider: ModelProvider,
    endpoint: String,
    model: String,
    api_key: Option<String>,
    client: Client,
}

#[derive(Serialize)]
struct AnalysisRecord {
    provider: ModelProvider,
    endpoint: String,
    model: String,
    created_unix_ms: u64,
    prompt: PromptRecord,
    response_text: String,
    raw_response: Value,
}

#[derive(Serialize)]
struct PromptRecord {
    system: String,
    user: String,
    temperature: f32,
    max_tokens: Option<u32>,
}

pub fn analyze_run(config: &AnalyzeConfig) -> Result<AnalyzeSummary, String> {
    if !config.run_dir.is_dir() {
        return Err(format!(
            "dataset run directory not found: {}",
            config.run_dir.display()
        ));
    }

    let adapter = build_adapter(config)?;
    let request = build_model_request(config)?;
    let response = adapter.complete(&request)?;

    let analysis_dir = config.run_dir.join("analysis");
    fs::create_dir_all(&analysis_dir).map_err(|err| {
        format!(
            "failed to create analysis dir {}: {err}",
            analysis_dir.display()
        )
    })?;

    let stem = format!(
        "{}--{}",
        config.provider.as_str(),
        sanitize_name(&response.model)
    );
    let output_markdown = analysis_dir.join(format!("{stem}.md"));
    let output_json = analysis_dir.join(format!("{stem}.json"));

    fs::write(&output_markdown, &response.text).map_err(|err| {
        format!(
            "failed to write analysis markdown {}: {err}",
            output_markdown.display()
        )
    })?;

    let endpoint = resolve_endpoint(config)?;
    let record = AnalysisRecord {
        provider: config.provider,
        endpoint,
        model: response.model.clone(),
        created_unix_ms: current_timestamp_millis(),
        prompt: PromptRecord {
            system: request.system_prompt,
            user: request.user_prompt,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        },
        response_text: response.text,
        raw_response: response.raw_json,
    };
    write_json_pretty(&output_json, &record)?;

    Ok(AnalyzeSummary {
        provider: config.provider,
        model: response.model,
        output_markdown,
        output_json,
    })
}

fn build_adapter(config: &AnalyzeConfig) -> Result<Box<dyn AnalysisModel>, String> {
    let endpoint = resolve_endpoint(config)?;
    let model = resolve_model(config)?;

    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|err| format!("failed to build model client: {err}"))?;

    Ok(Box::new(OpenAiCompatibleAdapter {
        provider: config.provider,
        endpoint,
        model,
        api_key: config.api_key.clone(),
        client,
    }))
}

fn build_model_request(config: &AnalyzeConfig) -> Result<ModelRequest, String> {
    let run_json = read_required(config.run_dir.join("run.json"))?;
    let features_json = read_required(config.run_dir.join("features.json"))?;
    let processes_json =
        trim_for_prompt(&read_required(config.run_dir.join("processes.json"))?, 6000);
    let aggregates_json = trim_for_prompt(
        &read_required(config.run_dir.join("aggregates.json"))?,
        3000,
    );
    let event_samples = sample_event_lines(
        &config.run_dir.join("events.jsonl"),
        DEFAULT_EVENT_SAMPLE_COUNT,
        DEFAULT_EVENT_SAMPLE_COUNT,
    )?;
    let extra_instructions = match &config.instructions_path {
        Some(path) => {
            let text = fs::read_to_string(path).map_err(|err| {
                format!("failed to read instructions file {}: {err}", path.display())
            })?;
            format!("\nAdditional instructions:\n{text}\n")
        }
        None => String::new(),
    };

    let user_prompt = format!(
        "Dataset run directory: {}\n\nRun metadata:\n```json\n{}\n```\n\nDerived features:\n```json\n{}\n```\n\nProcesses:\n```json\n{}\n```\n\nAggregates:\n```json\n{}\n```\n\nEvent samples:\n```jsonl\n{}\n```\n{}",
        config.run_dir.display(),
        run_json.trim(),
        features_json.trim(),
        processes_json.trim(),
        aggregates_json.trim(),
        event_samples.trim(),
        extra_instructions.trim(),
    );

    Ok(ModelRequest {
        system_prompt: DEFAULT_PROMPT_INSTRUCTIONS.to_string(),
        user_prompt,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
    })
}

fn resolve_endpoint(config: &AnalyzeConfig) -> Result<String, String> {
    match config.provider {
        ModelProvider::LmStudio => Ok(config
            .endpoint
            .clone()
            .unwrap_or_else(|| DEFAULT_LM_STUDIO_ENDPOINT.to_string())),
        ModelProvider::OpenAiCompatible => config
            .endpoint
            .clone()
            .ok_or_else(|| "openai-compatible provider requires --endpoint".to_string()),
    }
}

fn resolve_model(config: &AnalyzeConfig) -> Result<String, String> {
    match config.provider {
        ModelProvider::LmStudio => Ok(config
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_LM_STUDIO_MODEL.to_string())),
        ModelProvider::OpenAiCompatible => config
            .model
            .clone()
            .ok_or_else(|| "openai-compatible provider requires --model".to_string()),
    }
}

fn read_required(path: PathBuf) -> Result<String, String> {
    fs::read_to_string(&path)
        .map_err(|err| format!("failed to read dataset file {}: {err}", path.display()))
}

fn trim_for_prompt(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let trimmed: String = text.chars().take(max_chars).collect();
    format!("{trimmed}\n... truncated ...")
}

fn sample_event_lines(path: &Path, head: usize, tail: usize) -> Result<String, String> {
    let file = File::open(path)
        .map_err(|err| format!("failed to open dataset events {}: {err}", path.display()))?;
    let lines: Vec<String> = BufReader::new(file)
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read dataset events {}: {err}", path.display()))?;

    if lines.is_empty() {
        return Ok(String::new());
    }

    let mut samples = Vec::new();
    let head_end = head.min(lines.len());
    samples.extend(lines.iter().take(head_end).cloned());

    let tail_start = lines.len().saturating_sub(tail);
    for (index, line) in lines.iter().enumerate().skip(tail_start) {
        if index >= head_end {
            samples.push(line.clone());
        }
    }

    Ok(samples.join("\n"))
}

fn sanitize_name(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }

    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }

    sanitized.trim_matches('-').to_string()
}

fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn chat_completions_url(endpoint: &str) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    }
}

fn extract_chat_content(response: &Value) -> Result<String, String> {
    let Some(choice) = response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
    else {
        return Err("model response did not include choices[0]".to_string());
    };

    let Some(content) = choice
        .get("message")
        .and_then(|message| message.get("content"))
    else {
        return Err("model response did not include message.content".to_string());
    };

    if let Some(text) = content.as_str() {
        return Ok(text.to_string());
    }

    if let Some(parts) = content.as_array() {
        let mut text = String::new();
        for part in parts {
            if let Some(fragment) = part.get("text").and_then(Value::as_str) {
                text.push_str(fragment);
            }
        }
        if !text.is_empty() {
            return Ok(text);
        }
    }

    Err("model response content was not a supported text format".to_string())
}

fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let file = File::create(path)
        .map_err(|err| format!("failed to create analysis file {}: {err}", path.display()))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value).map_err(|err| {
        format!(
            "failed to serialize analysis file {}: {err}",
            path.display()
        )
    })?;
    writer
        .write_all(b"\n")
        .map_err(|err| format!("failed to finalize analysis file {}: {err}", path.display()))?;
    writer
        .flush()
        .map_err(|err| format!("failed to flush analysis file {}: {err}", path.display()))?;
    Ok(())
}

impl AnalysisModel for OpenAiCompatibleAdapter {
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, String> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(api_key) = &self.api_key {
            let bearer = format!("Bearer {}", api_key.trim());
            let header_value = HeaderValue::from_str(&bearer)
                .map_err(|err| format!("invalid API key header: {err}"))?;
            headers.insert(AUTHORIZATION, header_value);
        }

        let body = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": request.system_prompt,
                },
                {
                    "role": "user",
                    "content": request.user_prompt,
                }
            ],
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        });

        let response = self
            .client
            .post(chat_completions_url(&self.endpoint))
            .headers(headers)
            .json(&body)
            .send()
            .map_err(|err| {
                format!(
                    "failed to call {} model endpoint {}: {err}",
                    self.provider.as_str(),
                    self.endpoint
                )
            })?;

        let status = response.status();
        let raw_json: Value = response.json().map_err(|err| {
            format!(
                "model endpoint {} returned unreadable JSON: {err}",
                self.endpoint
            )
        })?;

        if !status.is_success() {
            return Err(format!(
                "model endpoint {} returned {}: {}",
                self.endpoint, status, raw_json
            ));
        }

        let text = extract_chat_content(&raw_json)?;
        Ok(ModelResponse {
            model: self.model.clone(),
            text,
            raw_json,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        chat_completions_url, extract_chat_content, resolve_endpoint, resolve_model,
        sample_event_lines, sanitize_name, AnalyzeConfig, ModelProvider,
    };
    use serde_json::json;
    use std::env;
    use std::fs;

    #[test]
    fn lm_studio_defaults_are_resolved() {
        let config = AnalyzeConfig::default();
        assert_eq!(
            resolve_endpoint(&config).expect("lm studio endpoint should default"),
            "http://127.0.0.1:1234/v1"
        );
        assert_eq!(
            resolve_model(&config).expect("lm studio model should default"),
            "qwen/qwen3.5-9b"
        );
    }

    #[test]
    fn openai_compatible_requires_explicit_endpoint_and_model() {
        let config = AnalyzeConfig {
            provider: ModelProvider::OpenAiCompatible,
            ..AnalyzeConfig::default()
        };
        assert!(resolve_endpoint(&config).is_err());
        assert!(resolve_model(&config).is_err());
    }

    #[test]
    fn chat_completion_url_appends_path_once() {
        assert_eq!(
            chat_completions_url("http://127.0.0.1:1234/v1"),
            "http://127.0.0.1:1234/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("http://127.0.0.1:1234/v1/chat/completions"),
            "http://127.0.0.1:1234/v1/chat/completions"
        );
    }

    #[test]
    fn sanitize_name_keeps_model_file_friendly() {
        assert_eq!(sanitize_name("qwen/qwen3.5-9b"), "qwen-qwen3.5-9b");
    }

    #[test]
    fn extract_chat_content_reads_string_message() {
        let payload = json!({
            "choices": [
                {
                    "message": {
                        "content": "analysis text"
                    }
                }
            ]
        });
        assert_eq!(
            extract_chat_content(&payload).expect("content should be extracted"),
            "analysis text"
        );
    }

    #[test]
    fn sample_event_lines_keeps_head_and_tail_without_duplication() {
        let path = env::temp_dir().join(format!(
            "ebpf-tracker-analysis-sample-{}.jsonl",
            super::current_timestamp_millis()
        ));
        fs::write(&path, "a\nb\nc\nd\ne\n").expect("sample file should be written");

        let sample = sample_event_lines(&path, 2, 2).expect("samples should be collected");

        assert_eq!(sample, "a\nb\nd\ne");
        fs::remove_file(path).expect("sample file should be removed");
    }

    #[test]
    fn sample_event_lines_keeps_all_lines_when_small() {
        let path = env::temp_dir().join(format!(
            "ebpf-tracker-analysis-sample-small-{}.jsonl",
            super::current_timestamp_millis()
        ));
        fs::write(&path, "a\nb\n").expect("sample file should be written");

        let sample = sample_event_lines(&path, 4, 4).expect("samples should be collected");

        assert_eq!(sample, "a\nb");
        fs::remove_file(path).expect("sample file should be removed");
    }
}
