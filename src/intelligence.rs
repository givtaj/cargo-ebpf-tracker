use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use ebpf_tracker_dataset::{
    analyze_run, default_output_root, ingest_records, AnalyzeConfig, DatasetConfig, DatasetSource,
    ModelProvider,
};
use ebpf_tracker_events::StreamRecord;
use serde::Serialize;

use crate::{current_timestamp_millis, CliArgs};

const INTELLIGENCE_STATUS_PREFIX: &str = "intelligence-status ";
const INTELLIGENCE_STATUS_FILE_NAME: &str = "status.json";
const CAPTURE_STATUS_INTERVAL_RECORDS: usize = 32;
const SUMMARY_EXCERPT_CHARS: usize = 900;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct IntelligenceOptions {
    pub enabled: bool,
    pub output_root: PathBuf,
    pub provider: ModelProvider,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub instructions_path: Option<PathBuf>,
}

impl Default for IntelligenceOptions {
    fn default() -> Self {
        let analyze = AnalyzeConfig::default();
        Self {
            enabled: false,
            output_root: default_output_root(),
            provider: analyze.provider,
            endpoint: analyze.endpoint,
            model: analyze.model,
            api_key: analyze.api_key,
            temperature: analyze.temperature,
            max_tokens: analyze.max_tokens,
            instructions_path: analyze.instructions_path,
        }
    }
}

impl IntelligenceOptions {
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }
}

pub(crate) fn parse_intelligence_arg(
    args: &[String],
    index: &mut usize,
    options: &mut IntelligenceOptions,
) -> Result<bool, String> {
    let Some(arg) = args.get(*index) else {
        return Ok(false);
    };

    match arg.as_str() {
        "--intelligence" | "--intelligence-dataset" => {
            options.enabled = true;
            *index += 1;
            Ok(true)
        }
        "--intelligence-output" => {
            let value = args
                .get(*index + 1)
                .ok_or_else(|| "missing value for --intelligence-output".to_string())?;
            options.enabled = true;
            options.output_root = PathBuf::from(value);
            *index += 2;
            Ok(true)
        }
        "--intelligence-provider" => {
            let value = args
                .get(*index + 1)
                .ok_or_else(|| "missing value for --intelligence-provider".to_string())?;
            options.enabled = true;
            options.provider = ModelProvider::parse(value)?;
            *index += 2;
            Ok(true)
        }
        "--intelligence-endpoint" => {
            let value = args
                .get(*index + 1)
                .ok_or_else(|| "missing value for --intelligence-endpoint".to_string())?;
            options.enabled = true;
            options.endpoint = Some(value.clone());
            *index += 2;
            Ok(true)
        }
        "--intelligence-model" => {
            let value = args
                .get(*index + 1)
                .ok_or_else(|| "missing value for --intelligence-model".to_string())?;
            options.enabled = true;
            options.model = Some(value.clone());
            *index += 2;
            Ok(true)
        }
        "--intelligence-api-key" => {
            let value = args
                .get(*index + 1)
                .ok_or_else(|| "missing value for --intelligence-api-key".to_string())?;
            options.enabled = true;
            options.api_key = Some(value.clone());
            *index += 2;
            Ok(true)
        }
        "--intelligence-temperature" => {
            let value = args
                .get(*index + 1)
                .ok_or_else(|| "missing value for --intelligence-temperature".to_string())?;
            options.enabled = true;
            options.temperature = value
                .parse()
                .map_err(|_| format!("invalid intelligence temperature: {value}"))?;
            *index += 2;
            Ok(true)
        }
        "--intelligence-max-tokens" => {
            let value = args
                .get(*index + 1)
                .ok_or_else(|| "missing value for --intelligence-max-tokens".to_string())?;
            options.enabled = true;
            options.max_tokens = Some(
                value
                    .parse()
                    .map_err(|_| format!("invalid intelligence max tokens: {value}"))?,
            );
            *index += 2;
            Ok(true)
        }
        "--intelligence-instructions-file" => {
            let value = args
                .get(*index + 1)
                .ok_or_else(|| "missing value for --intelligence-instructions-file".to_string())?;
            options.enabled = true;
            options.instructions_path = Some(PathBuf::from(value));
            *index += 2;
            Ok(true)
        }
        _ if arg.starts_with("--intelligence-output=") => {
            options.enabled = true;
            options.output_root = PathBuf::from(arg.trim_start_matches("--intelligence-output="));
            *index += 1;
            Ok(true)
        }
        _ if arg.starts_with("--intelligence-provider=") => {
            options.enabled = true;
            options.provider =
                ModelProvider::parse(arg.trim_start_matches("--intelligence-provider="))?;
            *index += 1;
            Ok(true)
        }
        _ if arg.starts_with("--intelligence-endpoint=") => {
            options.enabled = true;
            options.endpoint = Some(
                arg.trim_start_matches("--intelligence-endpoint=")
                    .to_string(),
            );
            *index += 1;
            Ok(true)
        }
        _ if arg.starts_with("--intelligence-model=") => {
            options.enabled = true;
            options.model = Some(arg.trim_start_matches("--intelligence-model=").to_string());
            *index += 1;
            Ok(true)
        }
        _ if arg.starts_with("--intelligence-api-key=") => {
            options.enabled = true;
            options.api_key = Some(
                arg.trim_start_matches("--intelligence-api-key=")
                    .to_string(),
            );
            *index += 1;
            Ok(true)
        }
        _ if arg.starts_with("--intelligence-temperature=") => {
            let value = arg.trim_start_matches("--intelligence-temperature=");
            options.enabled = true;
            options.temperature = value
                .parse()
                .map_err(|_| format!("invalid intelligence temperature: {value}"))?;
            *index += 1;
            Ok(true)
        }
        _ if arg.starts_with("--intelligence-max-tokens=") => {
            let value = arg.trim_start_matches("--intelligence-max-tokens=");
            options.enabled = true;
            options.max_tokens = Some(
                value
                    .parse()
                    .map_err(|_| format!("invalid intelligence max tokens: {value}"))?,
            );
            *index += 1;
            Ok(true)
        }
        _ if arg.starts_with("--intelligence-instructions-file=") => {
            options.enabled = true;
            options.instructions_path = Some(PathBuf::from(
                arg.trim_start_matches("--intelligence-instructions-file="),
            ));
            *index += 1;
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub(crate) fn append_intelligence_args(args: &mut Vec<String>, options: &IntelligenceOptions) {
    if !options.enabled {
        return;
    }

    let defaults = IntelligenceOptions::default();
    args.push("--intelligence-dataset".to_string());

    if options.output_root != defaults.output_root {
        args.push("--intelligence-output".to_string());
        args.push(options.output_root.display().to_string());
    }

    if options.provider != defaults.provider {
        args.push("--intelligence-provider".to_string());
        args.push(options.provider.as_str().to_string());
    }

    if let Some(endpoint) = &options.endpoint {
        args.push("--intelligence-endpoint".to_string());
        args.push(endpoint.clone());
    }

    if let Some(model) = &options.model {
        args.push("--intelligence-model".to_string());
        args.push(model.clone());
    }

    if let Some(api_key) = &options.api_key {
        args.push("--intelligence-api-key".to_string());
        args.push(api_key.clone());
    }

    if (options.temperature - defaults.temperature).abs() > f32::EPSILON {
        args.push("--intelligence-temperature".to_string());
        args.push(options.temperature.to_string());
    }

    if options.max_tokens != defaults.max_tokens {
        if let Some(max_tokens) = options.max_tokens {
            args.push("--intelligence-max-tokens".to_string());
            args.push(max_tokens.to_string());
        }
    }

    if let Some(instructions_path) = &options.instructions_path {
        args.push("--intelligence-instructions-file".to_string());
        args.push(instructions_path.display().to_string());
    }
}

#[derive(Clone)]
pub(crate) struct IntelligenceReporter {
    sender: Sender<IntelligenceMessage>,
}

pub(crate) struct IntelligenceSupervisor {
    reporter: IntelligenceReporter,
    handle: thread::JoinHandle<Result<(), String>>,
}

impl IntelligenceSupervisor {
    pub(crate) fn start(
        options: &IntelligenceOptions,
        cli_args: &CliArgs,
    ) -> Result<Option<Self>, String> {
        if !options.is_enabled() {
            return Ok(None);
        }

        let run_id = format!("run-{}-live", current_timestamp_millis());
        let dataset_dir = options.output_root.join(&run_id);
        let analysis_dir = dataset_dir.join("analysis");
        let status_path = analysis_dir.join(INTELLIGENCE_STATUS_FILE_NAME);
        let command = cli_args.command.join(" ");
        let test_name = match cli_args.session_record.as_ref() {
            Some(StreamRecord::Session { demo_name, .. }) => {
                Some(format!("{demo_name}-intelligence"))
            }
            _ => None,
        };
        let (sender, receiver) = mpsc::channel();
        let actor_config = IntelligenceActorConfig {
            options: options.clone(),
            run_id,
            dataset_dir,
            analysis_dir,
            status_path,
            command,
            test_name,
            transport: cli_args.transport_mode.as_str().to_string(),
            runtime: cli_args.runtime_selection.as_str().to_string(),
            initial_records: cli_args.session_record.iter().cloned().collect(),
        };

        let handle = thread::spawn(move || intelligence_actor(receiver, actor_config));
        Ok(Some(Self {
            reporter: IntelligenceReporter { sender },
            handle,
        }))
    }

    pub(crate) fn reporter(&self) -> IntelligenceReporter {
        self.reporter.clone()
    }

    pub(crate) fn finish(self, exit_code: i32, exit_signal: Option<String>) -> Result<(), String> {
        let _ = self.reporter.sender.send(IntelligenceMessage::Finish {
            exit_code,
            exit_signal,
        });
        self.handle
            .join()
            .map_err(|_| "intelligence supervisor thread panicked".to_string())?
    }
}

impl IntelligenceReporter {
    pub(crate) fn observe(&self, record: &StreamRecord) {
        let _ = self
            .sender
            .send(IntelligenceMessage::Record(record.clone()));
    }
}

enum IntelligenceMessage {
    Record(StreamRecord),
    Finish {
        exit_code: i32,
        exit_signal: Option<String>,
    },
}

struct IntelligenceActorConfig {
    options: IntelligenceOptions,
    run_id: String,
    dataset_dir: PathBuf,
    analysis_dir: PathBuf,
    status_path: PathBuf,
    command: String,
    test_name: Option<String>,
    transport: String,
    runtime: String,
    initial_records: Vec<StreamRecord>,
}

#[derive(Clone, Debug, Serialize)]
struct IntelligenceStatus {
    phase: String,
    run_id: String,
    dataset_dir: String,
    analysis_dir: String,
    provider: String,
    model: String,
    buffered_records: usize,
    updated_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    analysis_markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    analysis_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn intelligence_actor(
    receiver: Receiver<IntelligenceMessage>,
    config: IntelligenceActorConfig,
) -> Result<(), String> {
    fs::create_dir_all(&config.analysis_dir).map_err(|err| {
        format!(
            "failed to create intelligence analysis dir {}: {err}",
            config.analysis_dir.display()
        )
    })?;

    let mut records = config.initial_records.clone();
    let mut last_capture_report = records.len();

    publish_status(
        &config,
        IntelligenceStatus {
            phase: "capturing".to_string(),
            run_id: config.run_id.clone(),
            dataset_dir: config.dataset_dir.display().to_string(),
            analysis_dir: config.analysis_dir.display().to_string(),
            provider: config.options.provider.as_str().to_string(),
            model: resolve_model_label(&config.options),
            buffered_records: records.len(),
            updated_unix_ms: current_timestamp_millis(),
            message: Some("buffering live trace records for the dataset worker".to_string()),
            analysis_markdown: None,
            analysis_json: None,
            summary_excerpt: None,
            error: None,
        },
    )?;

    while let Ok(message) = receiver.recv() {
        match message {
            IntelligenceMessage::Record(record) => {
                records.push(record);
                if records.len() <= 4
                    || records.len().saturating_sub(last_capture_report)
                        >= CAPTURE_STATUS_INTERVAL_RECORDS
                {
                    last_capture_report = records.len();
                    publish_status(
                        &config,
                        IntelligenceStatus {
                            phase: "capturing".to_string(),
                            run_id: config.run_id.clone(),
                            dataset_dir: config.dataset_dir.display().to_string(),
                            analysis_dir: config.analysis_dir.display().to_string(),
                            provider: config.options.provider.as_str().to_string(),
                            model: resolve_model_label(&config.options),
                            buffered_records: records.len(),
                            updated_unix_ms: current_timestamp_millis(),
                            message: Some(format!(
                                "captured {} records; waiting for the traced command to finish",
                                records.len()
                            )),
                            analysis_markdown: None,
                            analysis_json: None,
                            summary_excerpt: None,
                            error: None,
                        },
                    )?;
                }
            }
            IntelligenceMessage::Finish {
                exit_code,
                exit_signal,
            } => {
                publish_status(
                    &config,
                    IntelligenceStatus {
                        phase: "writing_dataset".to_string(),
                        run_id: config.run_id.clone(),
                        dataset_dir: config.dataset_dir.display().to_string(),
                        analysis_dir: config.analysis_dir.display().to_string(),
                        provider: config.options.provider.as_str().to_string(),
                        model: resolve_model_label(&config.options),
                        buffered_records: records.len(),
                        updated_unix_ms: current_timestamp_millis(),
                        message: Some("writing the buffered run into a dataset bundle".to_string()),
                        analysis_markdown: None,
                        analysis_json: None,
                        summary_excerpt: None,
                        error: None,
                    },
                )?;

                let dataset_summary = ingest_records(
                    &records,
                    &DatasetConfig {
                        output_root: config.options.output_root.clone(),
                        run_id: Some(config.run_id.clone()),
                        source: Some(DatasetSource::Live),
                        command: non_empty_string(&config.command),
                        test_name: config.test_name.clone(),
                        transport: Some(config.transport.clone()),
                        runtime: Some(config.runtime.clone()),
                        exit_code: Some(exit_code),
                        exit_signal,
                        ..DatasetConfig::default()
                    },
                )?;

                publish_status(
                    &config,
                    IntelligenceStatus {
                        phase: "analyzing".to_string(),
                        run_id: config.run_id.clone(),
                        dataset_dir: config.dataset_dir.display().to_string(),
                        analysis_dir: config.analysis_dir.display().to_string(),
                        provider: config.options.provider.as_str().to_string(),
                        model: resolve_model_label(&config.options),
                        buffered_records: dataset_summary.total_records,
                        updated_unix_ms: current_timestamp_millis(),
                        message: Some("dataset bundle ready; running model analysis".to_string()),
                        analysis_markdown: None,
                        analysis_json: None,
                        summary_excerpt: None,
                        error: None,
                    },
                )?;

                let analyze_summary = match analyze_run(&AnalyzeConfig {
                    run_dir: dataset_summary.output_dir.clone(),
                    provider: config.options.provider,
                    endpoint: config.options.endpoint.clone(),
                    model: config.options.model.clone(),
                    api_key: config.options.api_key.clone(),
                    temperature: config.options.temperature,
                    max_tokens: config.options.max_tokens,
                    instructions_path: config.options.instructions_path.clone(),
                    live_logs: true,
                }) {
                    Ok(summary) => summary,
                    Err(err) => {
                        publish_status(
                            &config,
                            IntelligenceStatus {
                                phase: "failed".to_string(),
                                run_id: config.run_id.clone(),
                                dataset_dir: config.dataset_dir.display().to_string(),
                                analysis_dir: config.analysis_dir.display().to_string(),
                                provider: config.options.provider.as_str().to_string(),
                                model: resolve_model_label(&config.options),
                                buffered_records: dataset_summary.total_records,
                                updated_unix_ms: current_timestamp_millis(),
                                message: Some("dataset analysis failed".to_string()),
                                analysis_markdown: None,
                                analysis_json: None,
                                summary_excerpt: None,
                                error: Some(err.clone()),
                            },
                        )?;
                        return Err(err);
                    }
                };

                publish_status(
                    &config,
                    IntelligenceStatus {
                        phase: "completed".to_string(),
                        run_id: config.run_id.clone(),
                        dataset_dir: config.dataset_dir.display().to_string(),
                        analysis_dir: config.analysis_dir.display().to_string(),
                        provider: config.options.provider.as_str().to_string(),
                        model: analyze_summary.model.clone(),
                        buffered_records: dataset_summary.total_records,
                        updated_unix_ms: current_timestamp_millis(),
                        message: Some("intelligence analysis finished".to_string()),
                        analysis_markdown: Some(
                            analyze_summary.output_markdown.display().to_string(),
                        ),
                        analysis_json: Some(analyze_summary.output_json.display().to_string()),
                        summary_excerpt: read_summary_excerpt(&analyze_summary.output_markdown),
                        error: None,
                    },
                )?;

                return Ok(());
            }
        }
    }

    let error = "intelligence channel closed before the run finished".to_string();
    publish_status(
        &config,
        IntelligenceStatus {
            phase: "failed".to_string(),
            run_id: config.run_id.clone(),
            dataset_dir: config.dataset_dir.display().to_string(),
            analysis_dir: config.analysis_dir.display().to_string(),
            provider: config.options.provider.as_str().to_string(),
            model: resolve_model_label(&config.options),
            buffered_records: records.len(),
            updated_unix_ms: current_timestamp_millis(),
            message: Some("intelligence supervisor stopped unexpectedly".to_string()),
            analysis_markdown: None,
            analysis_json: None,
            summary_excerpt: None,
            error: Some(error.clone()),
        },
    )?;
    Err(error)
}

fn publish_status(
    config: &IntelligenceActorConfig,
    status: IntelligenceStatus,
) -> Result<(), String> {
    let serialized = serde_json::to_string(&status)
        .map_err(|err| format!("failed to serialize intelligence status: {err}"))?;
    fs::write(
        &config.status_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&status)
                .map_err(|err| format!("failed to serialize intelligence status file: {err}"))?
        ),
    )
    .map_err(|err| {
        format!(
            "failed to write intelligence status file {}: {err}",
            config.status_path.display()
        )
    })?;

    let mut stderr = io::stderr();
    stderr
        .write_all(format!("{INTELLIGENCE_STATUS_PREFIX}{serialized}\n").as_bytes())
        .map_err(|err| format!("failed to emit intelligence status: {err}"))?;
    stderr
        .flush()
        .map_err(|err| format!("failed to flush intelligence status: {err}"))?;
    Ok(())
}

fn resolve_model_label(options: &IntelligenceOptions) -> String {
    options
        .model
        .clone()
        .unwrap_or_else(|| "qwen/qwen3.5-9b".to_string())
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn read_summary_excerpt(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut excerpt: String = trimmed.chars().take(SUMMARY_EXCERPT_CHARS).collect();
    if trimmed.chars().count() > SUMMARY_EXCERPT_CHARS {
        excerpt.push_str("...");
    }
    Some(excerpt)
}
