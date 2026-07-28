use anyhow::Result;
use kube::Client;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{
    Category, IntoValue, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value,
};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::completions::{complete_clusters, complete_contexts, complete_users};
use crate::plugin::NukePlugin;

pub struct HttpGetCommand;

impl PluginCommand for HttpGetCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke http-get"
    }

    fn description(&self) -> &str {
        "Perform a raw HTTP GET against the Kubernetes API server (equivalent to kubectl get --raw)"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke http-get")
            .named("user", SyntaxShape::String, "Kubeconfig user to use", None)
            .named(
                "context",
                SyntaxShape::String,
                "Kubeconfig context to use",
                None,
            )
            .named(
                "cluster",
                SyntaxShape::String,
                "Kubeconfig cluster to use",
                None,
            )
            .required(
                "path",
                SyntaxShape::String,
                "API server path, e.g. /api/v1/nodes or /metrics",
            )
            .named(
                "headers",
                SyntaxShape::Record(vec![].into()),
                "Request headers as a record, e.g. {Accept: \"application/json\"}",
                Some('H'),
            )
            .named(
                "params",
                SyntaxShape::Record(vec![].into()),
                "Query parameters as a record; values can be strings or lists, e.g. {a: [\"one\", \"two\"], b: \"three\"}",
                Some('P'),
            )
            .switch(
                "raw",
                "Return the response body as a plain string instead of parsing JSON",
                Some('r'),
            )
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .category(Category::Custom("kubernetes".to_string()))
    }
    fn get_dynamic_completion(
        &self,
        _plugin: &NukePlugin,
        _engine: &EngineInterface,
        _call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        match arg_type {
            ArgType::Flag(ref name) => match name.as_ref() {
                "context" => Some(complete_contexts()),
                "cluster" => Some(complete_clusters()),
                "user" => Some(complete_users()),
                _ => None,
            },
            _ => None,
        }
    }

    fn run(
        &self,
        plugin: &NukePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        plugin
            .rt
            .block_on(run_http_get(plugin, call))
            .map_err(|e| LabeledError::new(e.to_string()))
    }
}

async fn run_http_get(_plugin: &NukePlugin, call: &EvaluatedCall) -> Result<PipelineData> {
    let path: String = call.req(0)?;
    let headers_flag: Option<Value> = call.get_flag("headers")?;
    let query_flag: Option<Value> = call.get_flag("query")?;
    let raw_flag: bool = call.has_flag("raw")?;
    let span = call.head;

    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context: call.get_flag("context")?,
        cluster: call.get_flag("cluster")?,
        user: call.get_flag("user")?,
    })
    .await?;

    let client = Client::try_from(config.clone())?;
    let uri = build_uri(&config.cluster_url, &path, query_flag.as_ref())?;
    let mut req_builder = http::Request::builder().method(http::Method::GET).uri(uri);
    if let Some(Value::Record { val, .. }) = &headers_flag {
        for (k, v) in val.iter() {
            req_builder = req_builder.header(k.as_str(), v.as_str().unwrap_or(""));
        }
    }
    let request = req_builder.body(vec![])?;
    let body: String = client.request_text(request).await?;
    if raw_flag {
        return Ok(PipelineData::Value(Value::string(body, span), None));
    }
    let nu_val = match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(json) => json.into_value(span),
        Err(_) => Value::string(body, span),
    };
    Ok(PipelineData::Value(nu_val, None))
}

fn build_uri(base_url: &http::Uri, path: &str, query: Option<&Value>) -> Result<http::Uri> {
    let base = base_url.to_string();
    let base = base.trim_end_matches('/');
    let path_clean = path.trim_start_matches('/');

    let uri_str = match query {
        Some(Value::Record { val, .. }) if !val.is_empty() => {
            let qs: String = val
                .iter()
                .flat_map(|(k, v)| {
                    let enc_key = utf8_percent_encode(k, NON_ALPHANUMERIC).to_string();
                    match v {
                        Value::List { vals, .. } => vals
                            .iter()
                            .map(|item| {
                                let enc_val = utf8_percent_encode(
                                    item.as_str().unwrap_or(""),
                                    NON_ALPHANUMERIC,
                                )
                                .to_string();
                                format!("{}={}", enc_key, enc_val)
                            })
                            .collect::<Vec<_>>(),
                        other => {
                            let enc_val =
                                utf8_percent_encode(other.as_str().unwrap_or(""), NON_ALPHANUMERIC)
                                    .to_string();
                            vec![format!("{}={}", enc_key, enc_val)]
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join("&");
            format!("{}/{}?{}", base, path_clean, qs)
        }
        _ => format!("{}/{}", base, path_clean),
    };

    Ok(uri_str.parse()?)
}
