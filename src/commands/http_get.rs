use anyhow::Result;
use kube::Client;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, Record, Signature, SyntaxShape, Type, Value,
};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::{client::config_from_context, plugin::KubectlPlugin};

pub struct HttpGetCommand;

impl PluginCommand for HttpGetCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str {
        "nuke http-get"
    }

    fn description(&self) -> &str {
        "Perform a raw HTTP GET against the Kubernetes API server (equivalent to kubectl get --raw)"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke http-get")
            .required(
                "path",
                SyntaxShape::String,
                "API server path, e.g. /api/v1/nodes or /metrics",
            )
            .named(
                "context",
                SyntaxShape::String,
                "Kubeconfig context to use",
                None,
            )
            .named(
                "headers",
                SyntaxShape::Record(vec![]),
                "Request headers as a record, e.g. {Accept: \"application/json\"}",
                Some('H'),
            )
            .named(
                "query",
                SyntaxShape::Record(vec![]),
                "Query parameters as a record; values can be strings or lists, e.g. {a: [\"one\", \"two\"], b: \"three\"}",
                Some('q'),
            )
            .switch(
                "raw",
                "Return the response body as a plain string instead of parsing JSON",
                Some('r'),
            )
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        plugin: &KubectlPlugin,
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

async fn run_http_get(_plugin: &KubectlPlugin, call: &EvaluatedCall) -> Result<PipelineData> {
    let path: String = call.req(0)?;
    let context_flag: Option<String> = call.get_flag("context")?;
    let headers_flag: Option<Value> = call.get_flag("headers")?;
    let query_flag: Option<Value> = call.get_flag("query")?;
    let raw_flag: bool = call.has_flag("raw")?;
    let span = call.head;

    let config = config_from_context(context_flag).await?;
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
        Ok(json) => json_to_nu_value(&json, span),
        Err(_) => Value::string(body, span),
    };
    Ok(PipelineData::Value(nu_val, None))
}

fn json_to_nu_value(json: &serde_json::Value, span: nu_protocol::Span) -> Value {
    match json {
        serde_json::Value::Null => Value::nothing(span),
        serde_json::Value::Bool(b) => Value::bool(*b, span),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::int(i, span)
            } else if let Some(f) = n.as_f64() {
                Value::float(f, span)
            } else {
                Value::string(n.to_string(), span)
            }
        }
        serde_json::Value::String(s) => Value::string(s.clone(), span),
        serde_json::Value::Array(arr) => Value::list(
            arr.iter().map(|v| json_to_nu_value(v, span)).collect(),
            span,
        ),
        serde_json::Value::Object(map) => {
            let record = Record::from_iter(
                map.iter()
                    .map(|(k, v)| (k.clone(), json_to_nu_value(v, span))),
            );
            Value::record(record, span)
        }
    }
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
