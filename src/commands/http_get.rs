use anyhow::Result;
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
// nu-plugin marks the dynamic-completion API experimental (deprecated) as a
// whole; implementing it is the only way to offer completions.
#[allow(deprecated)]
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{
    Category, IntoValue, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value,
};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

use crate::completions::complete_flags_only;
use crate::kube_env::{KubeEnv, cluster_flags};
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
        cluster_flags(Signature::build("nuke http-get"))
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
    #[allow(deprecated)]
    fn get_dynamic_completion(
        &self,
        plugin: &NukePlugin,
        engine: &EngineInterface,
        call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        complete_flags_only(plugin, engine, &call, &arg_type)
    }

    fn run(
        &self,
        plugin: &NukePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let env = KubeEnv::from_engine(engine);
        plugin
            .rt
            .block_on(run_http_get(&env, call))
            .map_err(crate::commands::to_labeled)
    }
}

async fn run_http_get(env: &KubeEnv, call: &EvaluatedCall) -> Result<PipelineData> {
    let path: String = call.req(0)?;
    let headers_flag: Option<Value> = call.get_flag("headers")?;
    let query_flag: Option<Value> = call.get_flag("params")?;
    let raw_flag: bool = call.has_flag("raw")?;
    let span = call.head;

    let (_, client) = env.connect(&crate::kube_env::selection(call)?).await?;
    let uri = build_uri(&path, query_flag.as_ref())?;
    let mut req_builder = http::Request::builder().method(http::Method::GET).uri(uri);
    if let Some(Value::Record { val, .. }) = &headers_flag {
        for (k, v) in val.iter() {
            let v = v.coerce_str().map_err(|_| {
                anyhow::anyhow!("header '{k}' must be a string, got {}", v.get_type())
            })?;
            req_builder = req_builder.header(k.as_str(), v.as_ref());
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

/// The request's path and query only. kube's client joins it onto the
/// cluster URL itself, keeping any path the server URL carries (e.g. a Rancher
/// proxy's `/k8s/clusters/<id>`), so including the base here would double it.
fn build_uri(path: &str, query: Option<&Value>) -> Result<http::Uri> {
    let mut uri = format!("/{}", path.trim_start_matches('/'));

    let Some(Value::Record { val, .. }) = query else {
        return Ok(uri.parse()?);
    };
    let encode = |s: &str| utf8_percent_encode(s, NON_ALPHANUMERIC).to_string();
    let mut pairs = Vec::new();
    for (key, value) in val.iter() {
        // A list repeats the key: {a: [x, y]} → a=x&a=y.
        let values = match value {
            Value::List { vals, .. } => vals.as_slice(),
            single => std::slice::from_ref(single),
        };
        for v in values {
            let v = v.coerce_str().map_err(|_| {
                anyhow::anyhow!(
                    "query parameter '{key}' must be a string, number or list of them, got {}",
                    v.get_type()
                )
            })?;
            pairs.push(format!("{}={}", encode(key), encode(&v)));
        }
    }
    if !pairs.is_empty() {
        // The path may already carry a query (`/api/v1/pods?limit=1`).
        uri.push(if uri.contains('?') { '&' } else { '?' });
        uri.push_str(&pairs.join("&"));
    }
    Ok(uri.parse()?)
}

#[cfg(test)]
mod tests {
    use super::build_uri;
    use nu_protocol::{Record, Span, Value};

    #[test]
    fn query_parameters() {
        let span = Span::test_data();
        let mut params = Record::new();
        params.push(
            "a",
            Value::list(
                vec![Value::string("x y", span), Value::string("z", span)],
                span,
            ),
        );
        params.push("limit", Value::int(1, span));
        let params = Value::record(params, span);

        let uri = |path| build_uri(path, Some(&params)).unwrap().to_string();
        assert_eq!(uri("api/v1/pods"), "/api/v1/pods?a=x%20y&a=z&limit=1");
        assert_eq!(
            uri("/api/v1/pods?watch=0"),
            "/api/v1/pods?watch=0&a=x%20y&a=z&limit=1"
        );
        assert_eq!(build_uri("/metrics", None).unwrap().to_string(), "/metrics");
    }
}
