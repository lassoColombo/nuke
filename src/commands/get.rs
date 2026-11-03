//! `nuke get`: list or fetch any resource the cluster serves, by any name
//! kubectl accepts.
//!
//! The apiserver is asked for the Table representation with the objects
//! included, as kubectl asks: one response carries every object and the
//! cells the apiserver's printer produced for it. The columns of a row come
//! from [`crate::columns`]: nuke's own definitions for a built-in, the CRD's
//! printer columns for a custom resource, the Table's cells otherwise.

use anyhow::Result;
use futures::future::join_all;
use kube::{Client, Config, api::ListParams};
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
// nu-plugin marks the dynamic-completion API experimental (deprecated) as a
// whole; implementing it is the only way to offer completions.
#[allow(deprecated)]
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{
    Category, DynamicSuggestion, IntoValue, LabeledError, PipelineData, Record, Signature, Span,
    SyntaxShape, Type, Value,
};
use serde_json::Value as Json;

use crate::columns::table::{self, Table};
use crate::columns::{Columns, OutputFormat, PrinterColumn, builtin, crd, self_describing};
use crate::completions::{
    complete_common_flag, complete_labels, complete_resource_instances, complete_resource_names,
    flag_prefix, flag_str, label_filter, positional_str, selection,
};
use crate::decorators::{self, Decorator};
use crate::discovery::ResourceEntry;
use crate::kube_env::{self, KubeEnv, cluster_flags};
use crate::plugin::NukePlugin;

pub struct GetCommand;

impl PluginCommand for GetCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke get"
    }

    fn description(&self) -> &str {
        "Get Kubernetes resources"
    }

    fn signature(&self) -> Signature {
        let sig = cluster_flags(Signature::build("nuke get"))
            .required(
                "resource",
                SyntaxShape::String,
                "Resource type: name/short-name (pods, po), name.group (deploy.apps), category (all), or fully-qualified group/version/plural (metrics.k8s.io/v1beta1/pods, v1/pods)",
            )
            .optional("name", SyntaxShape::String, "Resource name (omit to list all)")
            .named("namespace", SyntaxShape::String, "Namespace to use", Some('n'))
            .named(
                "output",
                SyntaxShape::String,
                "Output format: compact | wide | full  (default: compact for lists, wide for single)",
                Some('o'),
            )
            .switch("all-namespaces", "List resources across all namespaces", Some('A'))
            .named(
                "labels",
                SyntaxShape::String,
                "Label selector: comma-separated key=value pairs (e.g. app=web,tier=frontend)",
                Some('l'),
            );
        decorators::switches(sig)
            // A list is a table; a get by name, and a category (one list per
            // type), are a record.
            .input_output_types(vec![
                (Type::Nothing, Type::Table(vec![].into())),
                (Type::Nothing, Type::Record(vec![].into())),
            ])
            .category(Category::Custom("kubernetes".to_string()))
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
            .block_on(run_get(plugin, &env, call))
            .map_err(crate::commands::to_labeled)
    }

    #[allow(deprecated)]
    fn get_dynamic_completion(
        &self,
        plugin: &NukePlugin,
        engine: &EngineInterface,
        call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<DynamicSuggestion>> {
        let env = KubeEnv::from_engine(engine);
        let sel = selection(&call.call);
        let namespace = flag_str(&call.call, "namespace");
        let result = match arg_type {
            ArgType::Positional(0) => plugin
                .rt
                .block_on(complete_resource_names(plugin, &env, &sel)),
            ArgType::Positional(1) => {
                let resource = positional_str(&call.call, 0)?;
                plugin.rt.block_on(complete_resource_instances(
                    plugin, &env, resource, namespace, &sel,
                ))
            }
            ArgType::Flag(ref name) if matches!(name.as_ref(), "labels" | "l") => {
                let resource = positional_str(&call.call, 0)?;
                let all_namespaces = call
                    .call
                    .named_iter()
                    .any(|(n, _, _)| n.item == "all-namespaces" || n.item == "A");
                let labels = plugin
                    .rt
                    .block_on(complete_labels(
                        plugin,
                        &env,
                        resource,
                        namespace,
                        all_namespaces,
                        &sel,
                    ))
                    .ok()?;
                return Some(label_filter(flag_prefix(&call, "labels"), &labels));
            }
            ArgType::Flag(ref name) => return complete_common_flag(plugin, &env, &call, name),
            ArgType::Positional(_) => return None,
        };
        Some(result.unwrap_or_default())
    }
}

/// What one `nuke get` call asked for, once the flags are parsed.
struct Query<'a> {
    env: &'a KubeEnv,
    config: &'a Config,
    name: Option<String>,
    /// The namespace to read, `None` for `--all-namespaces`.
    namespace: Option<String>,
    /// The label selector, passed to the apiserver verbatim.
    params: ListParams,
    format: Option<OutputFormat>,
    decorators: Vec<&'static Decorator>,
    span: Span,
}

async fn run_get(plugin: &NukePlugin, env: &KubeEnv, call: &EvaluatedCall) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: Option<String> = call.opt(1)?;
    let all_namespaces = call.has_flag("all-namespaces")?;
    if name.is_some() && all_namespaces {
        anyhow::bail!("a resource cannot be retrieved by name across all namespaces");
    }

    let selection = kube_env::selection(call)?;
    let (config, client) = env.connect(&selection).await?;
    let namespace = match all_namespaces {
        true => None,
        false => Some(
            call.get_flag("namespace")?
                .unwrap_or(config.default_namespace.clone()),
        ),
    };
    // Trim to tolerate stray whitespace; an empty selector selects everything.
    let params = match call.get_flag::<String>("labels")? {
        Some(sel) if !sel.trim().is_empty() => {
            if name.is_some() {
                anyhow::bail!("a label selector cannot be combined with a resource name");
            }
            ListParams::default().labels(sel.trim())
        }
        _ => ListParams::default(),
    };
    let query = Query {
        env,
        config: &config,
        name,
        namespace,
        params,
        format: OutputFormat::from_flag(call)?,
        decorators: decorators::active(call)?,
        span: call.head,
    };

    let cache = plugin
        .discovery_or_fetch(env, &client, &config, |cache| {
            cache.resolve(&resource).is_some() || !cache.find_by_category(&resource).is_empty()
        })
        .await?;
    // A category first, as kubectl's `replaceAliases` expands one before
    // resolving any resource name.
    let category: Vec<&ResourceEntry> = cache
        .find_by_category(&resource)
        .into_iter()
        .filter(|e| e.verbs.iter().any(|v| v == "list"))
        .collect();
    let value = match (category.is_empty(), cache.resolve(&resource)) {
        (false, _) if query.name.is_some() => {
            anyhow::bail!("cannot specify a resource name when querying by category '{resource}'")
        }
        (false, _) => get_category(client, &category, &query).await?,
        (true, Some(entry)) => get_one_type(client, entry, &query).await?,
        (true, None) => anyhow::bail!("unknown resource type or category: '{resource}'"),
    };
    Ok(PipelineData::Value(value, None))
}

/// The Table of `entry` for the query: the one named object, or the list.
async fn fetch(client: &Client, entry: &ResourceEntry, q: &Query<'_>) -> kube::Result<Table> {
    let url_path = entry.url_path(q.namespace.as_deref());
    let request = match &q.name {
        Some(name) => table::get_request(&url_path, name)?,
        None => table::list_request(&url_path, &q.params)?,
    };
    let json: Json = client.request(request).await?;
    Table::from_response(json).map_err(kube::Error::SerdeError)
}

/// The CRD's printer columns for `entry`, when it is a custom resource
/// nuke may read. A failure to read them is not a failure to get: the row
/// falls back to the apiserver's cells, with a note.
async fn printer_columns(
    client: &Client,
    entry: &ResourceEntry,
    q: &Query<'_>,
) -> Option<Vec<PrinterColumn>> {
    if builtin::find(&entry.group, &entry.version, &entry.plural).is_some() {
        return None;
    }
    match crd::printer_columns(
        q.env,
        client,
        &q.config.cluster_url,
        &entry.group,
        &entry.version,
        &entry.plural,
    )
    .await
    {
        Ok(columns) => columns,
        Err(e) => {
            eprintln!("nuke: {e:#}");
            None
        }
    }
}

/// One resource type: a list, or the record itself for a get by name.
async fn get_one_type(client: Client, entry: &ResourceEntry, q: &Query<'_>) -> Result<Value> {
    let format = q
        .format
        .unwrap_or(OutputFormat::default_for(q.name.is_some()));
    let (table, printer) =
        futures::join!(fetch(&client, entry, q), printer_columns(&client, entry, q));
    let table = table?;
    let rows = rows(entry, table, printer.as_deref(), format, q);
    match &q.name {
        Some(name) => rows.into_iter().next().ok_or_else(|| {
            anyhow::anyhow!(
                "the apiserver returned no object for {}/{name}",
                entry.plural
            )
        }),
        // A list stays a list, even when it happens to hold one row.
        None => Ok(Value::list(rows, q.span)),
    }
}

/// Every type of a category, listed concurrently, as a record of
/// `plural: rows` — a category's rows have different shapes, so it cannot be
/// one table.
async fn get_category(client: Client, entries: &[&ResourceEntry], q: &Query<'_>) -> Result<Value> {
    let format = q.format.unwrap_or(OutputFormat::Compact);
    let lists = join_all(entries.iter().map(|entry| {
        let client = client.clone();
        async move {
            let (table, printer) =
                futures::join!(fetch(&client, entry, q), printer_columns(&client, entry, q));
            table.map(|t| (t, printer))
        }
    }))
    .await;

    let mut record = Record::new();
    for (entry, result) in entries.iter().zip(lists) {
        let (table, printer) = match result {
            Ok(fetched) => fetched,
            // Gone since discovery, or not listable after all: nothing to show.
            Err(kube::Error::Api(status)) if matches!(status.code, 404 | 405) => continue,
            // kubectl prints the error and carries on with the other types.
            Err(kube::Error::Api(status)) if status.code == 403 => {
                eprintln!("nuke: skipping {}: {}", entry.plural, status.message);
                continue;
            }
            Err(e) => {
                return Err(anyhow::Error::new(e).context(format!("listing {}", entry.plural)));
            }
        };
        // Two groups may put the same plural in one category: those keys
        // carry their group, as kubectl's `plural.group`.
        let shared = entries.iter().filter(|e| e.plural == entry.plural).count() > 1;
        let key = match shared {
            true => format!("{}.{}", entry.plural, entry.group),
            false => entry.plural.clone(),
        };
        let rows = rows(entry, table, printer.as_deref(), format, q);
        record.push(key, Value::list(rows, q.span));
    }
    Ok(Value::record(record, q.span))
}

/// The rows of `table` in `format`: raw objects for `full`, otherwise the
/// resource's columns and the decorators.
fn rows(
    entry: &ResourceEntry,
    table: Table,
    printer: Option<&[PrinterColumn]>,
    format: OutputFormat,
    q: &Query<'_>,
) -> Vec<Value> {
    let span = q.span;
    if format == OutputFormat::Full {
        let api_version = entry.api_version();
        return table
            .rows
            .into_iter()
            .map(|row| self_describing(row.object, &api_version, &entry.kind).into_value(span))
            .collect();
    }
    let columns = Columns::resolve(
        &entry.group,
        &entry.version,
        &entry.plural,
        &table.columns,
        printer,
        entry.namespaced,
    );
    let wide = format == OutputFormat::Wide;
    table
        .rows
        .iter()
        .map(|row| {
            let cells = (!row.cells.is_empty()).then_some(row.cells.as_slice());
            let mut rec = columns.row(&row.object, cells, entry.namespaced, wide, span);
            decorators::apply(&mut rec, &row.object, &q.decorators, span);
            Value::record(rec, span)
        })
        .collect()
}
