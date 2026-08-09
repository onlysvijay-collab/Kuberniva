use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use k8s_openapi::api::core::v1::{Event, Namespace, Node, Pod};
use kube::{
    api::{Api, AttachParams, DeleteParams, DynamicObject, ListParams, LogParams},
    config::{KubeConfigOptions, Kubeconfig},
    core::ApiResource,
    discovery::{verbs, Discovery, Scope},
    Client, Config,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
};
use tokio::{
    io::{copy_bidirectional, AsyncReadExt},
    net::TcpListener,
    sync::oneshot,
};
use x509_parser::{parse_x509_certificate, pem::parse_x509_pem};

static KUBE_CLIENT_CACHE: OnceLock<Mutex<HashMap<String, Client>>> = OnceLock::new();
static PORT_FORWARD_REGISTRY: OnceLock<Mutex<HashMap<String, PortForwardRuntime>>> =
    OnceLock::new();
static NEXT_PORT_FORWARD_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KubeContext {
    name: String,
    cluster: String,
    namespace: String,
    auth_method: String,
    current: bool,
    source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KubeconfigSummary {
    contexts: Vec<KubeContext>,
    current_context: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForgetKubeconfigContextRequest {
    kubeconfig_path: String,
    context: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceDescriptor {
    group: String,
    version: String,
    api_version: String,
    kind: String,
    plural: String,
    namespaced: bool,
    category: String,
    custom: bool,
    crd: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClusterCatalog {
    context: String,
    namespaces: Vec<String>,
    resources: Vec<ResourceDescriptor>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceRequest {
    kubeconfig_path: Option<String>,
    context: Option<String>,
    group: String,
    version: String,
    kind: String,
    plural: String,
    namespaced: bool,
    namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceObject {
    name: String,
    namespace: Option<String>,
    created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ready_containers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_containers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    restarts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    node_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceObjectRequest {
    kubeconfig_path: Option<String>,
    context: Option<String>,
    group: String,
    version: String,
    kind: String,
    plural: String,
    namespaced: bool,
    namespace: Option<String>,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveResourceRequest {
    kubeconfig_path: Option<String>,
    context: Option<String>,
    group: String,
    version: String,
    kind: String,
    plural: String,
    namespaced: bool,
    namespace: Option<String>,
    name: String,
    manifest: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveResourceYamlRequest {
    kubeconfig_path: Option<String>,
    context: Option<String>,
    group: String,
    version: String,
    kind: String,
    plural: String,
    namespaced: bool,
    namespace: Option<String>,
    name: String,
    yaml: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CertificateInfo {
    expires_at: String,
    days_remaining: i64,
    expired: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceDetail {
    manifest: Value,
    yaml: String,
    certificate: Option<CertificateInfo>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkloadPodRequest {
    kubeconfig_path: Option<String>,
    context: Option<String>,
    group: String,
    version: String,
    kind: String,
    plural: String,
    namespace: String,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PodLogRequest {
    kubeconfig_path: Option<String>,
    context: Option<String>,
    namespace: String,
    pod: String,
    container: Option<String>,
    tail_lines: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PodLogResponse {
    lines: Vec<String>,
    containers: Vec<String>,
    selected_container: Option<String>,
    ports: Vec<PodPort>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PodPort {
    container: String,
    name: Option<String>,
    port: i32,
    protocol: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PodRuntimeRequest {
    kubeconfig_path: Option<String>,
    context: Option<String>,
    namespace: String,
    pod: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PodRuntimeInfo {
    containers: Vec<String>,
    ports: Vec<PodPort>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PodExecRequest {
    kubeconfig_path: Option<String>,
    context: Option<String>,
    namespace: String,
    pod: String,
    container: Option<String>,
    command: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PodExecResponse {
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartPortForwardRequest {
    kubeconfig_path: Option<String>,
    context: Option<String>,
    namespace: String,
    pod: String,
    remote_port: u16,
    local_port: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StopPortForwardRequest {
    id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortForwardInfo {
    id: String,
    context: Option<String>,
    local_address: String,
    local_port: u16,
    remote_port: u16,
    namespace: String,
    pod: String,
}

struct PortForwardRuntime {
    info: PortForwardInfo,
    shutdown: oneshot::Sender<()>,
    stopped: oneshot::Receiver<()>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeOverview {
    name: String,
    ready: bool,
    roles: Vec<String>,
    labels: Vec<NodeProperty>,
    annotations: Vec<NodeProperty>,
    addresses: Vec<NodeAddressOverview>,
    conditions: Vec<NodeConditionOverview>,
    taints: Vec<NodeTaintOverview>,
    architecture: Option<String>,
    operating_system: Option<String>,
    os_image: Option<String>,
    kernel_version: Option<String>,
    kubelet_version: Option<String>,
    container_runtime_version: Option<String>,
    pod_cidrs: Vec<String>,
    provider_id: Option<String>,
    unschedulable: bool,
    uid: Option<String>,
    creation_timestamp: Option<String>,
    capacity: Vec<NodeProperty>,
    allocatable: Vec<NodeProperty>,
    cpu_capacity: Option<String>,
    memory_capacity: Option<String>,
    cpu_usage: Option<String>,
    memory_usage: Option<String>,
    cpu_usage_percent: Option<f64>,
    memory_usage_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeProperty {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeAddressOverview {
    type_: String,
    address: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeConditionOverview {
    type_: String,
    status: String,
    reason: Option<String>,
    message: Option<String>,
    last_heartbeat_time: Option<String>,
    last_transition_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeTaintOverview {
    key: String,
    value: Option<String>,
    effect: String,
    time_added: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClusterOverview {
    nodes: Vec<NodeOverview>,
    metrics_available: bool,
    observed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClusterEvent {
    name: String,
    namespace: Option<String>,
    event_type: String,
    reason: Option<String>,
    message: Option<String>,
    involved_kind: Option<String>,
    involved_name: Option<String>,
    action: Option<String>,
    count: Option<i32>,
    source: Option<String>,
    first_observed: Option<String>,
    last_observed: Option<String>,
}

fn node_time_string(
    time: Option<&k8s_openapi::apimachinery::pkg::apis::meta::v1::Time>,
) -> Option<String> {
    time.map(|value| value.0.to_string())
}

fn resolve_local_path(input: &str) -> PathBuf {
    let path = input.trim();
    let expanded = if path == "~" {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(path))
    } else if let Some(relative_path) = path.strip_prefix("~/") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(relative_path)
    } else {
        PathBuf::from(path)
    };
    if expanded.is_relative() {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join(expanded)
    } else {
        expanded
    }
}

fn load_kubeconfig(path: Option<&str>) -> Result<Kubeconfig, String> {
    match path {
        Some(path) if !path.trim().is_empty() => {
            let resolved_path = resolve_local_path(path);
            Kubeconfig::read_from(&resolved_path)
                .map_err(|error| format!("Could not read {}: {error}", resolved_path.display()))
        }
        _ => Kubeconfig::read().map_err(|error| error.to_string()),
    }
}

fn auth_method_for(kubeconfig: &Kubeconfig, user: Option<&str>) -> String {
    let auth_info = user.and_then(|user| {
        kubeconfig
            .auth_infos
            .iter()
            .find(|candidate| candidate.name == user)
            .and_then(|candidate| candidate.auth_info.as_ref())
    });
    match auth_info {
        Some(auth) if auth.exec.is_some() => "OIDC / exec".to_string(),
        Some(auth) if auth.auth_provider.is_some() => "OIDC provider".to_string(),
        Some(auth) if auth.token.is_some() || auth.token_file.is_some() => {
            "Bearer token".to_string()
        }
        Some(auth)
            if auth.client_certificate.is_some() || auth.client_certificate_data.is_some() =>
        {
            "Client certificate".to_string()
        }
        _ => "Credentials unavailable".to_string(),
    }
}

fn contexts_from_kubeconfig(
    kubeconfig: &Kubeconfig,
    source_path: Option<String>,
) -> Vec<KubeContext> {
    let current_context = kubeconfig.current_context.as_deref();
    kubeconfig
        .contexts
        .iter()
        .map(|named| {
            let context = named.context.clone().unwrap_or_default();
            KubeContext {
                current: current_context == Some(named.name.as_str()),
                name: named.name.clone(),
                cluster: context.cluster,
                namespace: context.namespace.unwrap_or_else(|| "default".to_string()),
                auth_method: auth_method_for(kubeconfig, context.user.as_deref()),
                source_path: source_path.clone(),
            }
        })
        .collect()
}

fn category_for(group: &str, plural: &str) -> (String, bool, bool) {
    let plural = plural.to_ascii_lowercase();
    let builtin_groups = [
        "",
        "apps",
        "batch",
        "autoscaling",
        "policy",
        "networking.k8s.io",
        "discovery.k8s.io",
        "storage.k8s.io",
        "rbac.authorization.k8s.io",
        "authentication.k8s.io",
        "authorization.k8s.io",
        "admissionregistration.k8s.io",
        "apiextensions.k8s.io",
        "apiregistration.k8s.io",
        "scheduling.k8s.io",
        "coordination.k8s.io",
        "events.k8s.io",
        "flowcontrol.apiserver.k8s.io",
        "node.k8s.io",
        "certificates.k8s.io",
    ];
    let custom = !builtin_groups.contains(&group);
    let crd = custom || plural == "customresourcedefinitions";
    // Gateway API resources are CRD-backed in most clusters, but they form a
    // first-class operational surface. Keep them out of the generic custom
    // resource bucket so Gateways and Routes are immediately discoverable.
    let category = if group == "gateway.networking.k8s.io" {
        "Gateway APIs"
    } else if crd {
        "Custom Resources"
    } else if matches!(
        plural.as_str(),
        "pods"
            | "deployments"
            | "daemonsets"
            | "statefulsets"
            | "replicasets"
            | "replicationcontrollers"
            | "jobs"
            | "cronjobs"
            | "horizontalpodautoscalers"
    ) {
        "Workloads"
    } else if matches!(
        plural.as_str(),
        "configmaps"
            | "secrets"
            | "resourcequotas"
            | "limitranges"
            | "priorityclasses"
            | "runtimeclasses"
    ) {
        "Configuration"
    } else if matches!(
        plural.as_str(),
        "serviceaccounts"
            | "roles"
            | "rolebindings"
            | "clusterroles"
            | "clusterrolebindings"
            | "leases"
    ) {
        "Access Control"
    } else if matches!(
        plural.as_str(),
        "services"
            | "ingresses"
            | "networkpolicies"
            | "endpoints"
            | "endpointslices"
            | "gateways"
            | "httproutes"
            | "grpcroutes"
            | "tcproutes"
            | "udproutes"
            | "referencegrants"
    ) {
        "Network"
    } else if matches!(
        plural.as_str(),
        "persistentvolumes"
            | "persistentvolumeclaims"
            | "storageclasses"
            | "volumeattachments"
            | "csidrivers"
            | "csinodes"
            | "csistoragecapacities"
    ) {
        "Storage"
    } else {
        "Cluster"
    };
    (category.to_string(), custom, crd)
}

fn api_resource(group: String, version: String, kind: String, plural: String) -> ApiResource {
    let api_version = if group.is_empty() {
        version.clone()
    } else {
        format!("{group}/{version}")
    };
    ApiResource {
        group,
        version,
        api_version,
        kind,
        plural,
    }
}

fn workload_label_selector(manifest: &Value) -> Result<String, String> {
    let selector = manifest
        .pointer("/spec/selector")
        .and_then(Value::as_object)
        .ok_or_else(|| "This resource does not expose a pod label selector".to_string())?;
    let mut parts = Vec::new();
    if let Some(labels) = selector.get("matchLabels").and_then(Value::as_object) {
        for (key, value) in labels {
            let value = value.as_str().ok_or_else(|| {
                "The workload pod selector is not a set of string labels".to_string()
            })?;
            parts.push(format!("{key}={value}"));
        }
    }
    if let Some(expressions) = selector.get("matchExpressions").and_then(Value::as_array) {
        for expression in expressions {
            let key = expression
                .get("key")
                .and_then(Value::as_str)
                .filter(|key| !key.is_empty())
                .ok_or_else(|| {
                    "A workload label selector expression is missing its key".to_string()
                })?;
            let operator = expression
                .get("operator")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    "A workload label selector expression is missing its operator".to_string()
                })?;
            let values = match expression.get("values").and_then(Value::as_array) {
                Some(values) => values
                    .iter()
                    .map(|value| value.as_str().map(str::to_string))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        "A workload label selector expression contains a non-string value"
                            .to_string()
                    })?,
                None => Vec::new(),
            };
            match operator {
                "In" if !values.is_empty() => {
                    parts.push(format!("{key} in ({})", values.join(",")))
                }
                "NotIn" if !values.is_empty() => {
                    parts.push(format!("{key} notin ({})", values.join(",")))
                }
                "Exists" => parts.push(key.to_string()),
                "DoesNotExist" => parts.push(format!("!{key}")),
                _ => {
                    return Err(format!(
                        "Unsupported or empty label selector expression for {key}"
                    ))
                }
            }
        }
    }
    if parts.is_empty() {
        return Err("This workload has an empty pod label selector".to_string());
    }
    Ok(parts.join(","))
}

fn certificate_info(manifest: &Value) -> Option<CertificateInfo> {
    if let Some(expires_at) = manifest.pointer("/status/notAfter").and_then(Value::as_str) {
        let expiry = chrono::DateTime::parse_from_rfc3339(expires_at)
            .ok()?
            .with_timezone(&Utc);
        let days_remaining = (expiry.timestamp() - Utc::now().timestamp()) / 86_400;
        return Some(CertificateInfo {
            expires_at: expiry.to_rfc3339(),
            days_remaining,
            expired: days_remaining < 0,
        });
    }

    let certificate = manifest.pointer("/data/tls.crt").and_then(Value::as_str)?;
    let bytes = STANDARD.decode(certificate).ok()?;
    let expiry_timestamp = if let Ok((_, pem)) = parse_x509_pem(&bytes) {
        pem.parse_x509().ok()?.validity().not_after.timestamp()
    } else {
        parse_x509_certificate(&bytes)
            .ok()?
            .1
            .validity()
            .not_after
            .timestamp()
    };
    let days_remaining = (expiry_timestamp - Utc::now().timestamp()) / 86_400;
    let expires_at = chrono::DateTime::from_timestamp(expiry_timestamp, 0)?.to_rfc3339();
    Some(CertificateInfo {
        expires_at,
        days_remaining,
        expired: days_remaining < 0,
    })
}

fn quantity_as_number(quantity: &str, cpu: bool) -> Option<f64> {
    let quantity = quantity.trim();
    if quantity.is_empty() {
        return None;
    }
    // Plain and exponent values are valid Kubernetes quantities, and are the
    // common representation for CPU cores and byte counts.
    if let Ok(number) = quantity.parse::<f64>() {
        return Some(number);
    }
    let units: &[(&str, f64)] = if cpu {
        &[
            ("n", 1e-9),
            ("u", 1e-6),
            ("m", 1e-3),
            ("k", 1e3),
            ("M", 1e6),
            ("G", 1e9),
            ("T", 1e12),
            ("P", 1e15),
            ("E", 1e18),
        ]
    } else {
        &[
            ("Ki", 1024_f64.powi(1)),
            ("Mi", 1024_f64.powi(2)),
            ("Gi", 1024_f64.powi(3)),
            ("Ti", 1024_f64.powi(4)),
            ("Pi", 1024_f64.powi(5)),
            ("Ei", 1024_f64.powi(6)),
            ("k", 1e3),
            ("M", 1e6),
            ("G", 1e9),
            ("T", 1e12),
            ("P", 1e15),
            ("E", 1e18),
            ("m", 1e-3),
        ]
    };
    units.iter().find_map(|(suffix, multiplier)| {
        quantity
            .strip_suffix(suffix)
            .and_then(|number| number.parse::<f64>().ok())
            .map(|number| number * multiplier)
    })
}

fn usage_percent(usage: Option<&str>, capacity: Option<&str>, cpu: bool) -> Option<f64> {
    let usage = quantity_as_number(usage?, cpu)?;
    let capacity = quantity_as_number(capacity?, cpu)?;
    (capacity > 0.0).then(|| (usage / capacity * 100.0).clamp(0.0, 100.0))
}

fn format_cpu_usage(cores: f64) -> String {
    if cores >= 1.0 {
        format!("{cores:.2} cores")
    } else {
        format!("{}m", (cores * 1_000.0).round() as i64)
    }
}

fn format_memory_usage(bytes: f64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    if bytes >= GIB {
        format!("{:.1}Gi", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.0}Mi", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.0}Ki", bytes / KIB)
    } else {
        format!("{}B", bytes.max(0.0).round() as i64)
    }
}

async fn pod_usage_map(
    client: Client,
    namespace: Option<&str>,
) -> HashMap<(String, String), (String, String)> {
    let resource = ApiResource {
        group: "metrics.k8s.io".to_string(),
        version: "v1beta1".to_string(),
        api_version: "metrics.k8s.io/v1beta1".to_string(),
        kind: "PodMetrics".to_string(),
        plural: "pods".to_string(),
    };
    let api = match namespace.filter(|value| *value != "all namespaces") {
        Some(namespace) => Api::<DynamicObject>::namespaced_with(client, namespace, &resource),
        None => Api::<DynamicObject>::all_with(client, &resource),
    };
    let response = match api.list(&ListParams::default()).await {
        Ok(response) => response,
        Err(_) => return HashMap::new(),
    };
    response
        .items
        .into_iter()
        .filter_map(|metric| {
            let name = metric.metadata.name?;
            let namespace = metric.metadata.namespace.unwrap_or_default();
            let containers = metric.data.get("containers")?.as_array()?;
            let mut cpu_values = Vec::new();
            let mut memory_values = Vec::new();
            for container in containers {
                let usage = container.get("usage").and_then(Value::as_object);
                if let Some(cpu) = usage
                    .and_then(|usage| usage.get("cpu"))
                    .and_then(Value::as_str)
                    .and_then(|value| quantity_as_number(value, true))
                {
                    cpu_values.push(cpu);
                }
                if let Some(memory) = usage
                    .and_then(|usage| usage.get("memory"))
                    .and_then(Value::as_str)
                    .and_then(|value| quantity_as_number(value, false))
                {
                    memory_values.push(memory);
                }
            }
            if cpu_values.is_empty() && memory_values.is_empty() {
                return None;
            }
            let cpu = if cpu_values.is_empty() {
                String::new()
            } else {
                format_cpu_usage(cpu_values.into_iter().sum())
            };
            let memory = if memory_values.is_empty() {
                String::new()
            } else {
                format_memory_usage(memory_values.into_iter().sum())
            };
            Some(((namespace, name), (cpu, memory)))
        })
        .collect()
}

fn pod_resource_object(pod: Pod, usage: Option<&(String, String)>) -> ResourceObject {
    let name = pod.metadata.name.unwrap_or_default();
    let namespace = pod.metadata.namespace;
    let created_at = pod
        .metadata
        .creation_timestamp
        .map(|timestamp| timestamp.0.to_string());
    let total_containers = pod.spec.as_ref().map(|spec| spec.containers.len() as u32);
    let node_name = pod.spec.as_ref().and_then(|spec| spec.node_name.clone());
    let (ready_containers, restarts) = pod
        .status
        .as_ref()
        .and_then(|status| status.container_statuses.as_ref())
        .map(|statuses| {
            (
                statuses.iter().filter(|status| status.ready).count() as u32,
                statuses
                    .iter()
                    .map(|status| status.restart_count.max(0) as u32)
                    .sum::<u32>(),
            )
        })
        .unwrap_or((0, 0));
    ResourceObject {
        name,
        namespace,
        created_at,
        status: pod.status.as_ref().and_then(|status| status.phase.clone()),
        ready_containers: total_containers.map(|_| ready_containers),
        total_containers,
        restarts: total_containers.map(|_| restarts),
        cpu_usage: usage
            .map(|usage| usage.0.clone())
            .filter(|value| !value.is_empty()),
        memory_usage: usage
            .map(|usage| usage.1.clone())
            .filter(|value| !value.is_empty()),
        node_name,
    }
}

fn configure_desktop_exec_path() {
    static PATH_CONFIGURED: OnceLock<()> = OnceLock::new();
    PATH_CONFIGURED.get_or_init(|| {
        let mut paths = env::var_os("PATH")
            .map(|path| env::split_paths(&path).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut fallback_paths = vec![
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/opt/homebrew/sbin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/local/sbin"),
        ];
        if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
            fallback_paths.extend([
                home.join(".local/bin"),
                home.join(".krew/bin"),
                home.join(".asdf/shims"),
                home.join("go/bin"),
                home.join("bin"),
            ]);
        }
        for fallback_path in fallback_paths {
            if fallback_path.is_dir() && !paths.contains(&fallback_path) {
                paths.push(fallback_path);
            }
        }
        if let Ok(path) = env::join_paths(paths) {
            env::set_var("PATH", path);
        }
    });
}

fn exec_command_for(
    kubeconfig: &Kubeconfig,
    requested_context: Option<&str>,
) -> Option<(String, Option<String>)> {
    let context_name = requested_context.or(kubeconfig.current_context.as_deref())?;
    let context = kubeconfig
        .contexts
        .iter()
        .find(|candidate| candidate.name == context_name)
        .and_then(|candidate| candidate.context.as_ref())?;
    let auth_info = context.user.as_deref().and_then(|user| {
        kubeconfig
            .auth_infos
            .iter()
            .find(|candidate| candidate.name == user)
            .and_then(|candidate| candidate.auth_info.as_ref())
    })?;
    let exec = auth_info.exec.as_ref()?;
    Some((exec.command.clone()?, exec.install_hint.clone()))
}

fn command_is_available(command: &str) -> bool {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 {
        return command_path.is_file();
    }
    env::var_os("PATH")
        .map(|path| env::split_paths(&path).any(|directory| directory.join(command).is_file()))
        .unwrap_or(false)
}

fn validate_exec_command(
    kubeconfig: &Kubeconfig,
    requested_context: Option<&str>,
) -> Result<(), String> {
    configure_desktop_exec_path();
    let Some((command, install_hint)) = exec_command_for(kubeconfig, requested_context) else {
        return Ok(());
    };
    if command_is_available(&command) {
        return Ok(());
    }
    let hint = install_hint
        .map(|hint| format!(" {hint}"))
        .unwrap_or_else(|| " Install the credential helper, or set user.exec.command in this kubeconfig to its absolute path.".to_string());
    Err(format!(
        "OIDC credential helper `{command}` was not found. Kuberniva checked the app PATH plus Homebrew, Krew, ~/.local/bin, ~/.asdf/shims, ~/go/bin, and ~/bin.{hint}"
    ))
}

fn client_cache_key(path: Option<&str>, context: Option<&str>) -> String {
    let source = path
        .filter(|path| !path.trim().is_empty())
        .map(resolve_local_path)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| "default-kubeconfig".to_string());
    format!("{source}\u{0}{}", context.unwrap_or("current-context"))
}

async fn client_for(path: Option<String>, context: Option<String>) -> Result<Client, String> {
    let cache_key = client_cache_key(path.as_deref(), context.as_deref());
    let client_cache = KUBE_CLIENT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(client) = client_cache
        .lock()
        .map_err(|_| "Kuberniva's Kubernetes client cache is unavailable".to_string())?
        .get(&cache_key)
        .cloned()
    {
        return Ok(client);
    }
    let options = KubeConfigOptions {
        context,
        ..Default::default()
    };
    let kubeconfig = load_kubeconfig(path.as_deref())?;
    validate_exec_command(&kubeconfig, options.context.as_deref())?;
    let config = Config::from_custom_kubeconfig(kubeconfig, &options)
        .await
        .map_err(|error| error.to_string())?;
    let client = Client::try_from(config).map_err(|error| error.to_string())?;
    let mut cache = client_cache
        .lock()
        .map_err(|_| "Kuberniva's Kubernetes client cache is unavailable".to_string())?;
    if cache.len() >= 64 {
        cache.clear();
    }
    cache.insert(cache_key, client.clone());
    Ok(client)
}

#[tauri::command]
fn invalidate_cluster_client(
    kubeconfig_path: Option<String>,
    context: Option<String>,
) -> Result<(), String> {
    let cache_key = client_cache_key(kubeconfig_path.as_deref(), context.as_deref());
    let cache = KUBE_CLIENT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    cache
        .lock()
        .map_err(|_| "Kuberniva's Kubernetes client cache is unavailable".to_string())?
        .remove(&cache_key);
    Ok(())
}

#[tauri::command]
fn read_kubeconfig_contexts(kubeconfig_path: Option<String>) -> Result<KubeconfigSummary, String> {
    if let Some(path) = kubeconfig_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    {
        let resolved_path = resolve_local_path(path);
        if resolved_path.is_dir() {
            let mut contexts = Vec::new();
            let mut files = fs::read_dir(&resolved_path)
                .map_err(|error| format!("Could not read kubeconfig directory: {error}"))?
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    entry
                        .file_type()
                        .ok()
                        .filter(|file_type| file_type.is_file())
                        .map(|_| entry.path())
                })
                .collect::<Vec<_>>();
            files.sort();

            for file in files {
                let source_path = file
                    .canonicalize()
                    .unwrap_or(file)
                    .to_string_lossy()
                    .into_owned();
                if let Ok(kubeconfig) = Kubeconfig::read_from(&source_path) {
                    contexts.extend(contexts_from_kubeconfig(&kubeconfig, Some(source_path)));
                }
            }

            if contexts.is_empty() {
                return Err("No valid kubeconfig contexts found in this directory".to_string());
            }
            let current_context = contexts
                .iter()
                .find(|context| context.current)
                .map(|context| context.name.clone());
            return Ok(KubeconfigSummary {
                contexts,
                current_context,
            });
        }
    }

    let kubeconfig = load_kubeconfig(kubeconfig_path.as_deref())?;
    let current_context = kubeconfig.current_context.clone();
    let source_path = kubeconfig_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .map(resolve_local_path)
        .map(|path| path.to_string_lossy().into_owned());
    let contexts = contexts_from_kubeconfig(&kubeconfig, source_path);
    Ok(KubeconfigSummary {
        contexts,
        current_context,
    })
}

#[tauri::command]
fn forget_kubeconfig_context(request: ForgetKubeconfigContextRequest) -> Result<(), String> {
    let kubeconfig_path = resolve_local_path(&request.kubeconfig_path)
        .canonicalize()
        .map_err(|error| format!("Could not resolve kubeconfig source: {error}"))?;
    if !kubeconfig_path.is_file() {
        return Err("A single kubeconfig file is required to remove a context".to_string());
    }

    // Parse the source directly instead of Kubeconfig::read_from: that helper resolves
    // certificate and credential paths, which would rewrite relative paths on save.
    let source = fs::read_to_string(&kubeconfig_path)
        .map_err(|error| format!("Could not read kubeconfig source: {error}"))?;
    let mut kubeconfig: Kubeconfig = serde_yaml::from_str(&source)
        .map_err(|error| format!("Could not parse kubeconfig source: {error}"))?;
    let removed_context = kubeconfig
        .contexts
        .iter()
        .find(|candidate| candidate.name == request.context)
        .cloned()
        .ok_or_else(|| {
            format!(
                "Context `{}` was not found in this kubeconfig",
                request.context
            )
        })?;
    let removed_cluster = removed_context
        .context
        .as_ref()
        .map(|context| context.cluster.clone());
    let removed_user = removed_context.context.and_then(|context| context.user);

    kubeconfig
        .contexts
        .retain(|candidate| candidate.name != request.context);
    let referenced_clusters = kubeconfig
        .contexts
        .iter()
        .filter_map(|candidate| {
            candidate
                .context
                .as_ref()
                .map(|context| context.cluster.clone())
        })
        .collect::<HashSet<_>>();
    let referenced_users = kubeconfig
        .contexts
        .iter()
        .filter_map(|candidate| {
            candidate
                .context
                .as_ref()
                .and_then(|context| context.user.clone())
        })
        .collect::<HashSet<_>>();
    if let Some(cluster) = removed_cluster {
        if !referenced_clusters.contains(&cluster) {
            kubeconfig
                .clusters
                .retain(|candidate| candidate.name != cluster);
        }
    }
    if let Some(user) = removed_user {
        if !referenced_users.contains(&user) {
            kubeconfig
                .auth_infos
                .retain(|candidate| candidate.name != user);
        }
    }
    if kubeconfig.current_context.as_deref() == Some(request.context.as_str()) {
        kubeconfig.current_context = kubeconfig
            .contexts
            .first()
            .map(|context| context.name.clone());
    }

    let updated = serde_yaml::to_string(&kubeconfig)
        .map_err(|error| format!("Could not write kubeconfig: {error}"))?;
    let temporary_path = kubeconfig_path.with_file_name(format!(
        ".{}.kuberniva-write-{}",
        kubeconfig_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("config"),
        Utc::now().timestamp_nanos_opt().unwrap_or_default(),
    ));
    fs::write(&temporary_path, updated)
        .map_err(|error| format!("Could not stage kubeconfig update: {error}"))?;
    fs::rename(&temporary_path, &kubeconfig_path)
        .map_err(|error| format!("Could not replace kubeconfig: {error}"))?;

    Ok(())
}

#[tauri::command]
async fn discover_cluster_catalog(
    kubeconfig_path: Option<String>,
    context: Option<String>,
) -> Result<ClusterCatalog, String> {
    let selected_context = context
        .clone()
        .unwrap_or_else(|| "current context".to_string());
    let client = client_for(kubeconfig_path, context).await?;

    // Aggregated discovery is two API calls on modern clusters. Older servers retain the
    // conventional discovery fallback, so the catalog remains complete across versions.
    let discovery = match Discovery::new(client.clone()).run_aggregated().await {
        Ok(discovery) => discovery,
        Err(_) => Discovery::new(client.clone())
            .run()
            .await
            .map_err(|error| error.to_string())?,
    };

    let mut resources = discovery
        .groups_alphabetical()
        .into_iter()
        .flat_map(|group| group.resources_by_stability())
        .filter(|(_, capabilities)| capabilities.supports_operation(verbs::LIST))
        .map(|(resource, capabilities)| {
            let (category, custom, crd) = category_for(&resource.group, &resource.plural);
            ResourceDescriptor {
                group: resource.group,
                version: resource.version,
                api_version: resource.api_version,
                kind: resource.kind,
                plural: resource.plural,
                namespaced: capabilities.scope == Scope::Namespaced,
                category,
                custom,
                crd,
            }
        })
        .collect::<Vec<_>>();
    resources.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then(left.kind.cmp(&right.kind))
    });

    let namespaces: BTreeSet<String> = Api::<Namespace>::all(client)
        .list(&ListParams::default())
        .await
        .map_err(|error| error.to_string())?
        .items
        .into_iter()
        .filter_map(|namespace| namespace.metadata.name)
        .collect();

    Ok(ClusterCatalog {
        context: selected_context,
        namespaces: namespaces.into_iter().collect(),
        resources,
    })
}

#[tauri::command]
async fn read_cluster_overview(
    kubeconfig_path: Option<String>,
    context: Option<String>,
) -> Result<ClusterOverview, String> {
    let client = client_for(kubeconfig_path, context).await?;
    let node_metrics_resource = ApiResource {
        group: "metrics.k8s.io".to_string(),
        version: "v1beta1".to_string(),
        api_version: "metrics.k8s.io/v1beta1".to_string(),
        kind: "NodeMetrics".to_string(),
        plural: "nodes".to_string(),
    };
    // Node metrics are optional (the metrics-server may not be installed). Fetch them in
    // parallel with the authoritative Node list so an unavailable Metrics API never delays
    // the cluster overview itself.
    let metrics_api = Api::<DynamicObject>::all_with(client.clone(), &node_metrics_resource);
    let nodes_api = Api::<Node>::all(client);
    let metrics_params = ListParams::default();
    let nodes_params = ListParams::default();
    let metrics_request = metrics_api.list(&metrics_params);
    let nodes_request = nodes_api.list(&nodes_params);
    let (metrics_response, nodes_response) = tokio::join!(metrics_request, nodes_request);
    let metrics = metrics_response.ok().map(|response| {
        response
            .items
            .into_iter()
            .filter_map(|metric| {
                let name = metric.metadata.name?;
                Some((
                    name,
                    (
                        metric
                            .data
                            .pointer("/usage/cpu")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        metric
                            .data
                            .pointer("/usage/memory")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    ),
                ))
            })
            .collect::<HashMap<_, _>>()
    });
    let metrics_available = metrics.is_some();
    let metrics = metrics.unwrap_or_default();
    let mut nodes = nodes_response
        .map_err(|error| error.to_string())?
        .items
        .into_iter()
        .filter_map(|node| {
            let name = node.metadata.name.clone()?;
            let label_map = node.metadata.labels.clone().unwrap_or_default();
            let roles = {
                let mut roles = label_map
                    .iter()
                    .filter_map(|(key, value)| {
                        if key == "kubernetes.io/role" {
                            Some(value.clone())
                        } else {
                            key.strip_prefix("node-role.kubernetes.io/").map(|role| {
                                if role.is_empty() {
                                    "worker".to_string()
                                } else {
                                    role.to_string()
                                }
                            })
                        }
                    })
                    .collect::<Vec<_>>();
                roles.sort();
                roles.dedup();
                roles
            };
            let labels = label_map
                .iter()
                .map(|(key, value)| NodeProperty {
                    key: key.clone(),
                    value: value.clone(),
                })
                .collect::<Vec<_>>();
            let annotations = node
                .metadata
                .annotations
                .as_ref()
                .map(|values| {
                    values
                        .iter()
                        .map(|(key, value)| NodeProperty {
                            key: key.clone(),
                            value: value.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let ready = node
                .status
                .as_ref()
                .and_then(|status| status.conditions.as_ref())
                .map(|conditions| {
                    conditions
                        .iter()
                        .any(|condition| condition.type_ == "Ready" && condition.status == "True")
                })
                .unwrap_or(false);
            let capacity = node
                .status
                .as_ref()
                .and_then(|status| status.capacity.as_ref());
            let allocatable = node
                .status
                .as_ref()
                .and_then(|status| status.allocatable.as_ref());
            let (cpu_usage, memory_usage) = metrics.get(&name).cloned().unwrap_or((None, None));
            let cpu_capacity = capacity
                .and_then(|capacity| capacity.get("cpu"))
                .map(|quantity| quantity.0.clone());
            let memory_capacity = capacity
                .and_then(|capacity| capacity.get("memory"))
                .map(|quantity| quantity.0.clone());
            let capacity = capacity
                .map(|values| {
                    values
                        .iter()
                        .map(|(key, value)| NodeProperty {
                            key: key.clone(),
                            value: value.0.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let allocatable = allocatable
                .map(|values| {
                    values
                        .iter()
                        .map(|(key, value)| NodeProperty {
                            key: key.clone(),
                            value: value.0.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let status = node.status.as_ref();
            let node_info = status.and_then(|status| status.node_info.as_ref());
            let conditions = status
                .and_then(|status| status.conditions.as_ref())
                .map(|values| {
                    values
                        .iter()
                        .map(|condition| NodeConditionOverview {
                            type_: condition.type_.clone(),
                            status: condition.status.clone(),
                            reason: condition.reason.clone(),
                            message: condition.message.clone(),
                            last_heartbeat_time: node_time_string(
                                condition.last_heartbeat_time.as_ref(),
                            ),
                            last_transition_time: node_time_string(
                                condition.last_transition_time.as_ref(),
                            ),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let addresses = status
                .and_then(|status| status.addresses.as_ref())
                .map(|values| {
                    values
                        .iter()
                        .map(|address| NodeAddressOverview {
                            type_: address.type_.clone(),
                            address: address.address.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let taints = node
                .spec
                .as_ref()
                .and_then(|spec| spec.taints.as_ref())
                .map(|values| {
                    values
                        .iter()
                        .map(|taint| NodeTaintOverview {
                            key: taint.key.clone(),
                            value: taint.value.clone(),
                            effect: taint.effect.clone(),
                            time_added: node_time_string(taint.time_added.as_ref()),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let pod_cidrs = node
                .spec
                .as_ref()
                .map(|spec| {
                    spec.pod_cidrs
                        .clone()
                        .unwrap_or_else(|| spec.pod_cidr.clone().into_iter().collect())
                })
                .unwrap_or_default();
            Some(NodeOverview {
                name,
                ready,
                roles,
                labels,
                annotations,
                addresses,
                conditions,
                taints,
                architecture: node_info.map(|info| info.architecture.clone()),
                operating_system: node_info.map(|info| info.operating_system.clone()),
                os_image: node_info.map(|info| info.os_image.clone()),
                kernel_version: node_info.map(|info| info.kernel_version.clone()),
                kubelet_version: node_info.map(|info| info.kubelet_version.clone()),
                container_runtime_version: node_info
                    .map(|info| info.container_runtime_version.clone()),
                pod_cidrs,
                provider_id: node.spec.as_ref().and_then(|spec| spec.provider_id.clone()),
                unschedulable: node
                    .spec
                    .as_ref()
                    .and_then(|spec| spec.unschedulable)
                    .unwrap_or(false),
                uid: node.metadata.uid.clone(),
                creation_timestamp: node_time_string(node.metadata.creation_timestamp.as_ref()),
                capacity,
                allocatable,
                cpu_usage_percent: usage_percent(
                    cpu_usage.as_deref(),
                    cpu_capacity.as_deref(),
                    true,
                ),
                memory_usage_percent: usage_percent(
                    memory_usage.as_deref(),
                    memory_capacity.as_deref(),
                    false,
                ),
                cpu_capacity,
                memory_capacity,
                cpu_usage,
                memory_usage,
            })
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(ClusterOverview {
        nodes,
        metrics_available,
        observed_at: Utc::now().to_rfc3339(),
    })
}

#[tauri::command]
async fn read_cluster_events(
    kubeconfig_path: Option<String>,
    context: Option<String>,
) -> Result<Vec<ClusterEvent>, String> {
    let client = client_for(kubeconfig_path, context).await?;
    let params = ListParams {
        limit: Some(250),
        ..Default::default()
    };
    let mut events = Api::<Event>::all(client)
        .list(&params)
        .await
        .map_err(|error| error.to_string())?
        .items
        .into_iter()
        .filter_map(|event| {
            let name = event.metadata.name.clone()?;
            let last_observed = event
                .event_time
                .as_ref()
                .map(|time| time.0.to_string())
                .or_else(|| node_time_string(event.last_timestamp.as_ref()))
                .or_else(|| node_time_string(event.first_timestamp.as_ref()))
                .or_else(|| node_time_string(event.metadata.creation_timestamp.as_ref()));
            let first_observed = event
                .first_timestamp
                .as_ref()
                .and_then(|time| node_time_string(Some(time)))
                .or_else(|| event.event_time.as_ref().map(|time| time.0.to_string()));
            let source = event.reporting_component.clone().or_else(|| {
                event
                    .source
                    .as_ref()
                    .and_then(|source| source.component.clone())
            });
            Some(ClusterEvent {
                name,
                namespace: event
                    .metadata
                    .namespace
                    .clone()
                    .or(event.involved_object.namespace.clone()),
                event_type: event.type_.unwrap_or_else(|| "Normal".to_string()),
                reason: event.reason,
                message: event.message,
                involved_kind: event.involved_object.kind,
                involved_name: event.involved_object.name,
                action: event.action,
                count: event
                    .count
                    .or_else(|| event.series.as_ref().and_then(|series| series.count)),
                source,
                first_observed,
                last_observed,
            })
        })
        .collect::<Vec<_>>();
    events.sort_by(|left, right| right.last_observed.cmp(&left.last_observed));
    Ok(events)
}

#[tauri::command]
async fn list_resource_objects(request: ResourceRequest) -> Result<Vec<ResourceObject>, String> {
    let client = client_for(request.kubeconfig_path, request.context).await?;
    if request.kind == "Pod" {
        let namespace = request
            .namespace
            .as_deref()
            .filter(|value| *value != "all namespaces");
        let pods_api = match namespace {
            Some(namespace) => Api::<Pod>::namespaced(client.clone(), namespace),
            None => Api::<Pod>::all(client.clone()),
        };
        let list_params = ListParams::default();
        let (usage, pods_result) = tokio::join!(
            pod_usage_map(client, namespace),
            pods_api.list(&list_params),
        );
        let mut pods = pods_result
            .map_err(|error| error.to_string())?
            .items
            .into_iter()
            .map(|pod| {
                let key = (
                    pod.metadata.namespace.clone().unwrap_or_default(),
                    pod.metadata.name.clone().unwrap_or_default(),
                );
                pod_resource_object(pod, usage.get(&key))
            })
            .collect::<Vec<_>>();
        pods.sort_by(|left, right| {
            left.namespace
                .cmp(&right.namespace)
                .then(left.name.cmp(&right.name))
        });
        return Ok(pods);
    }
    let api_version = if request.group.is_empty() {
        request.version.clone()
    } else {
        format!("{}/{}", request.group, request.version)
    };
    let resource = ApiResource {
        group: request.group,
        version: request.version,
        api_version,
        kind: request.kind,
        plural: request.plural,
    };
    let api: Api<DynamicObject> = if request.namespaced {
        match request.namespace.as_deref() {
            Some(namespace) if namespace != "all namespaces" => {
                Api::namespaced_with(client, namespace, &resource)
            }
            _ => Api::all_with(client, &resource),
        }
    } else {
        Api::all_with(client, &resource)
    };
    let mut objects = api
        .list(&ListParams::default())
        .await
        .map_err(|error| error.to_string())?
        .items
        .into_iter()
        .map(|object| ResourceObject {
            name: object.metadata.name.unwrap_or_default(),
            namespace: object.metadata.namespace,
            created_at: object
                .metadata
                .creation_timestamp
                .map(|timestamp| timestamp.0.to_string()),
            status: None,
            ready_containers: None,
            total_containers: None,
            restarts: None,
            cpu_usage: None,
            memory_usage: None,
            node_name: None,
        })
        .collect::<Vec<_>>();
    objects.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then(left.name.cmp(&right.name))
    });
    Ok(objects)
}

#[tauri::command]
async fn get_resource_detail(request: ResourceObjectRequest) -> Result<ResourceDetail, String> {
    let client = client_for(request.kubeconfig_path, request.context).await?;
    let resource = api_resource(request.group, request.version, request.kind, request.plural);
    let object: DynamicObject = if request.namespaced {
        let namespace = request
            .namespace
            .ok_or_else(|| "A namespace is required for this resource".to_string())?;
        Api::<DynamicObject>::namespaced_with(client, &namespace, &resource)
            .get(&request.name)
            .await
            .map_err(|error| error.to_string())?
    } else {
        Api::<DynamicObject>::all_with(client, &resource)
            .get(&request.name)
            .await
            .map_err(|error| error.to_string())?
    };
    let manifest = serde_json::to_value(object).map_err(|error| error.to_string())?;
    let certificate = certificate_info(&manifest);
    let yaml = serde_yaml::to_string(&manifest).map_err(|error| error.to_string())?;
    Ok(ResourceDetail {
        manifest,
        yaml,
        certificate,
    })
}

#[tauri::command]
async fn delete_resource_object(request: ResourceObjectRequest) -> Result<(), String> {
    let client = client_for(request.kubeconfig_path, request.context).await?;
    let resource = api_resource(request.group, request.version, request.kind, request.plural);
    if request.namespaced {
        let namespace = request
            .namespace
            .ok_or_else(|| "A namespace is required for this resource".to_string())?;
        Api::<DynamicObject>::namespaced_with(client, &namespace, &resource)
            .delete(&request.name, &DeleteParams::default())
            .await
            .map_err(|error| error.to_string())?;
    } else {
        Api::<DynamicObject>::all_with(client, &resource)
            .delete(&request.name, &DeleteParams::default())
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn save_resource_detail(request: SaveResourceRequest) -> Result<(), String> {
    let client = client_for(request.kubeconfig_path, request.context).await?;
    let resource = api_resource(request.group, request.version, request.kind, request.plural);
    let object: DynamicObject = serde_json::from_value(request.manifest)
        .map_err(|error| format!("Invalid resource data: {error}"))?;
    if request.namespaced {
        let namespace = request
            .namespace
            .ok_or_else(|| "A namespace is required for this resource".to_string())?;
        Api::namespaced_with(client, &namespace, &resource)
            .replace(&request.name, &kube::api::PostParams::default(), &object)
            .await
            .map_err(|error| error.to_string())?;
    } else {
        Api::all_with(client, &resource)
            .replace(&request.name, &kube::api::PostParams::default(), &object)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn save_resource_yaml(request: SaveResourceYamlRequest) -> Result<(), String> {
    let manifest: Value =
        serde_yaml::from_str(&request.yaml).map_err(|error| format!("Invalid YAML: {error}"))?;
    save_resource_detail(SaveResourceRequest {
        kubeconfig_path: request.kubeconfig_path,
        context: request.context,
        group: request.group,
        version: request.version,
        kind: request.kind,
        plural: request.plural,
        namespaced: request.namespaced,
        namespace: request.namespace,
        name: request.name,
        manifest,
    })
    .await
}

#[tauri::command]
async fn list_workload_pods(request: WorkloadPodRequest) -> Result<Vec<ResourceObject>, String> {
    let client = client_for(request.kubeconfig_path, request.context).await?;
    let api_version = if request.group.is_empty() {
        request.version.clone()
    } else {
        format!("{}/{}", request.group, request.version)
    };
    let resource = ApiResource {
        group: request.group,
        version: request.version,
        api_version,
        kind: request.kind,
        plural: request.plural,
    };
    let workload: DynamicObject =
        Api::namespaced_with(client.clone(), &request.namespace, &resource)
            .get(&request.name)
            .await
            .map_err(|error| error.to_string())?;
    let selector = workload_label_selector(&workload.data)?;

    let pods_api = Api::<Pod>::namespaced(client.clone(), &request.namespace);
    let list_params = ListParams::default().labels(&selector);
    let (usage, pods_result) = tokio::join!(
        pod_usage_map(client, Some(&request.namespace)),
        pods_api.list(&list_params),
    );
    let mut pods = pods_result
        .map_err(|error| error.to_string())?
        .items
        .into_iter()
        .filter_map(|pod| {
            pod.metadata.name.as_ref()?;
            let key = (
                pod.metadata.namespace.clone().unwrap_or_default(),
                pod.metadata.name.clone().unwrap_or_default(),
            );
            Some(pod_resource_object(pod, usage.get(&key)))
        })
        .collect::<Vec<_>>();
    pods.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(pods)
}

#[tauri::command]
async fn read_pod_logs(request: PodLogRequest) -> Result<PodLogResponse, String> {
    let client = client_for(request.kubeconfig_path, request.context).await?;
    let pods = Api::<Pod>::namespaced(client, &request.namespace);
    let pod = pods
        .get(&request.pod)
        .await
        .map_err(|error| error.to_string())?;
    let containers: Vec<String> = pod
        .spec
        .as_ref()
        .map(|spec| {
            spec.containers
                .iter()
                .map(|container| container.name.clone())
                .collect()
        })
        .unwrap_or_default();
    let ports = pod
        .spec
        .as_ref()
        .map(|spec| {
            spec.containers
                .iter()
                .flat_map(|container| {
                    container
                        .ports
                        .as_ref()
                        .into_iter()
                        .flatten()
                        .map(move |port| PodPort {
                            container: container.name.clone(),
                            name: port.name.clone(),
                            port: port.container_port,
                            protocol: port
                                .protocol
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_else(|| "TCP".to_string()),
                        })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let tail_lines = request.tail_lines.unwrap_or(500).clamp(1, 5_000);
    let selected_container = request.container.or_else(|| containers.first().cloned());
    let output = pods
        .logs(
            &request.pod,
            &LogParams {
                container: selected_container.clone(),
                tail_lines: Some(tail_lines),
                timestamps: true,
                ..Default::default()
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(PodLogResponse {
        lines: output.lines().map(str::to_string).collect(),
        containers,
        selected_container,
        ports,
    })
}

fn pod_runtime_info(pod: &Pod) -> PodRuntimeInfo {
    let containers = pod
        .spec
        .as_ref()
        .map(|spec| {
            spec.containers
                .iter()
                .map(|container| container.name.clone())
                .collect()
        })
        .unwrap_or_default();
    let ports = pod
        .spec
        .as_ref()
        .map(|spec| {
            spec.containers
                .iter()
                .flat_map(|container| {
                    container
                        .ports
                        .as_ref()
                        .into_iter()
                        .flatten()
                        .map(move |port| PodPort {
                            container: container.name.clone(),
                            name: port.name.clone(),
                            port: port.container_port,
                            protocol: port
                                .protocol
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_else(|| "TCP".to_string()),
                        })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    PodRuntimeInfo { containers, ports }
}

#[tauri::command]
async fn get_pod_runtime(request: PodRuntimeRequest) -> Result<PodRuntimeInfo, String> {
    let client = client_for(request.kubeconfig_path, request.context).await?;
    let pod = Api::<Pod>::namespaced(client, &request.namespace)
        .get(&request.pod)
        .await
        .map_err(|error| error.to_string())?;
    Ok(pod_runtime_info(&pod))
}

#[tauri::command]
async fn exec_pod_command(request: PodExecRequest) -> Result<PodExecResponse, String> {
    let command = request.command.trim();
    if command.is_empty() {
        return Err("Enter a command to run in the selected container".to_string());
    }
    let client = client_for(request.kubeconfig_path, request.context).await?;
    let pods = Api::<Pod>::namespaced(client, &request.namespace);
    let selected_container = request
        .container
        .filter(|container| !container.trim().is_empty());
    let attach = selected_container
        .clone()
        .map(|container| AttachParams::default().container(container))
        .unwrap_or_default()
        .stdout(true)
        .stderr(true);
    let shell_command = command.to_string();
    let response = tokio::time::timeout(std::time::Duration::from_secs(30), async move {
        let mut attached = pods
            .exec(
                &request.pod,
                vec!["/bin/sh", "-lc", shell_command.as_str()],
                &attach,
            )
            .await
            .map_err(|error| error.to_string())?;
        let mut stdout_reader = attached.stdout().ok_or_else(|| {
            "The Kubernetes API did not provide stdout for this exec session".to_string()
        })?;
        let mut stderr_reader = attached.stderr().ok_or_else(|| {
            "The Kubernetes API did not provide stderr for this exec session".to_string()
        })?;
        let stdout_task = async {
            let mut buffer = Vec::new();
            stdout_reader
                .read_to_end(&mut buffer)
                .await
                .map_err(|error| error.to_string())?;
            Ok::<Vec<u8>, String>(buffer)
        };
        let stderr_task = async {
            let mut buffer = Vec::new();
            stderr_reader
                .read_to_end(&mut buffer)
                .await
                .map_err(|error| error.to_string())?;
            Ok::<Vec<u8>, String>(buffer)
        };
        let (stdout, stderr) = tokio::join!(stdout_task, stderr_task);
        attached.join().await.map_err(|error| error.to_string())?;
        Ok::<PodExecResponse, String>(PodExecResponse {
            stdout: String::from_utf8_lossy(&stdout?).into_owned(),
            stderr: String::from_utf8_lossy(&stderr?).into_owned(),
        })
    })
    .await
    .map_err(|_| "The command did not finish within 30 seconds".to_string())??;
    Ok(response)
}

#[tauri::command]
async fn start_port_forward(request: StartPortForwardRequest) -> Result<PortForwardInfo, String> {
    if request.remote_port == 0 {
        return Err("Choose a remote port between 1 and 65535".to_string());
    }
    let requested_local_port = request.local_port.unwrap_or(request.remote_port);
    if requested_local_port == 0 {
        return Err("Choose a local port between 1 and 65535".to_string());
    }

    // Resolve credentials before binding a local port so an OIDC/configuration error does not
    // leave a misleading listener behind.
    let context = request.context.clone();
    let client = client_for(request.kubeconfig_path, request.context).await?;
    let listener = TcpListener::bind(("127.0.0.1", requested_local_port))
        .await
        .map_err(|error| {
            format!(
                "Could not listen on 127.0.0.1:{requested_local_port}: {error}. Choose another local port."
            )
        })?;
    let local_port = listener
        .local_addr()
        .map_err(|error| format!("Could not read the local listener address: {error}"))?
        .port();
    let id = format!(
        "pf-{}",
        NEXT_PORT_FORWARD_ID.fetch_add(1, Ordering::Relaxed)
    );
    let info = PortForwardInfo {
        id: id.clone(),
        context,
        local_address: format!("127.0.0.1:{local_port}"),
        local_port,
        remote_port: request.remote_port,
        namespace: request.namespace.clone(),
        pod: request.pod.clone(),
    };
    let (shutdown_sender, mut shutdown_receiver) = oneshot::channel();
    let (stopped_sender, stopped_receiver) = oneshot::channel();
    PORT_FORWARD_REGISTRY
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "The port-forward registry is unavailable".to_string())?
        .insert(
            id.clone(),
            PortForwardRuntime {
                info: info.clone(),
                shutdown: shutdown_sender,
                stopped: stopped_receiver,
            },
        );

    let task_id = id.clone();
    let namespace = request.namespace.clone();
    let pod = request.pod.clone();
    let remote_port = request.remote_port;
    tauri::async_runtime::spawn(async move {
        let mut connections = Vec::new();
        loop {
            tokio::select! {
                _ = &mut shutdown_receiver => break,
                accepted = listener.accept() => match accepted {
                    Ok((mut local_stream, _)) => {
                        let pod_client = client.clone();
                        let pod_namespace = namespace.clone();
                        let pod_name = pod.clone();
                        connections.retain(|connection: &tokio::task::JoinHandle<()>| !connection.is_finished());
                        connections.push(tokio::spawn(async move {
                            let pods = Api::<Pod>::namespaced(pod_client, &pod_namespace);
                            match pods.portforward(&pod_name, &[remote_port]).await {
                                Ok(mut forwarder) => {
                                    if let Some(mut remote_stream) = forwarder.take_stream(remote_port) {
                                        let _ = copy_bidirectional(&mut local_stream, &mut remote_stream).await;
                                    }
                                    forwarder.abort();
                                }
                                Err(error) => log::warn!("Kuberniva port forward to {pod_namespace}/{pod_name}:{remote_port} failed: {error}"),
                            }
                        }));
                    }
                    Err(error) => {
                        log::warn!("Kuberniva port-forward listener stopped: {error}");
                        break;
                    }
                }
            }
        }
        for connection in connections {
            connection.abort();
            let _ = connection.await;
        }
        if let Some(registry) = PORT_FORWARD_REGISTRY.get() {
            if let Ok(mut forwards) = registry.lock() {
                forwards.remove(&task_id);
            }
        }
        let _ = stopped_sender.send(());
    });

    Ok(info)
}

#[tauri::command]
fn list_port_forwards() -> Result<Vec<PortForwardInfo>, String> {
    let mut forwards = PORT_FORWARD_REGISTRY
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "The port-forward registry is unavailable".to_string())?
        .values()
        .map(|runtime| runtime.info.clone())
        .collect::<Vec<_>>();
    forwards.sort_by(|left, right| {
        left.context
            .cmp(&right.context)
            .then(left.namespace.cmp(&right.namespace))
            .then(left.pod.cmp(&right.pod))
            .then(left.local_port.cmp(&right.local_port))
    });
    Ok(forwards)
}

#[tauri::command]
async fn stop_port_forward(request: StopPortForwardRequest) -> Result<PortForwardInfo, String> {
    let runtime = PORT_FORWARD_REGISTRY
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "The port-forward registry is unavailable".to_string())?
        .remove(&request.id)
        .ok_or_else(|| "This port forward is no longer active".to_string())?;
    let PortForwardRuntime {
        info,
        shutdown,
        stopped,
    } = runtime;
    let _ = shutdown.send(());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), stopped).await;
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn kubeconfig(name: &str) -> String {
        format!(
            "apiVersion: v1\nkind: Config\nclusters:\n  - name: {name}\n    cluster:\n      server: https://{name}.example.invalid\nusers:\n  - name: {name}-user\n    user:\n      token: test-token\ncontexts:\n  - name: {name}\n    context:\n      cluster: {name}\n      user: {name}-user\ncurrent-context: {name}\n"
        )
    }

    fn exec_kubeconfig(command: &str) -> String {
        format!(
            "apiVersion: v1\nkind: Config\nclusters:\n  - name: oidc\n    cluster:\n      server: https://oidc.example.invalid\nusers:\n  - name: oidc-user\n    user:\n      exec:\n        apiVersion: client.authentication.k8s.io/v1beta1\n        command: {command}\n        installHint: Install the OIDC helper.\ncontexts:\n  - name: oidc\n    context:\n      cluster: oidc\n      user: oidc-user\ncurrent-context: oidc\n"
        )
    }

    #[test]
    fn reads_contexts_from_each_kubeconfig_in_a_directory() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after the epoch")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("kuberniva-kubeconfigs-{suffix}"));
        fs::create_dir_all(&directory).expect("create temporary kubeconfig directory");
        fs::write(directory.join("cluster-a.yaml"), kubeconfig("cluster-a"))
            .expect("write first kubeconfig");
        fs::write(directory.join("cluster-b.yaml"), kubeconfig("cluster-b"))
            .expect("write second kubeconfig");
        fs::write(directory.join("not-a-kubeconfig"), "not valid yaml")
            .expect("write ignored file");

        let summary = read_kubeconfig_contexts(Some(directory.to_string_lossy().into_owned()))
            .expect("read kubeconfig directory");
        let names = summary
            .contexts
            .iter()
            .map(|context| context.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["cluster-a", "cluster-b"]);
        assert!(summary
            .contexts
            .iter()
            .all(|context| context.source_path.is_some()));
        fs::remove_dir_all(directory).expect("remove temporary kubeconfig directory");
    }

    #[test]
    fn expands_home_shorthand_and_home_relative_paths() {
        let home = std::env::var_os("HOME").expect("home directory is set");
        assert_eq!(
            resolve_local_path("~/clusters/production"),
            PathBuf::from(&home).join("clusters/production")
        );
        assert_eq!(
            resolve_local_path("clusters/production"),
            PathBuf::from(home).join("clusters/production")
        );
    }

    #[test]
    fn labels_crd_definitions_and_crd_backed_resources_in_the_catalog() {
        let (definitions_category, definitions_custom, definitions_crd) =
            category_for("apiextensions.k8s.io", "customresourcedefinitions");
        assert_eq!(definitions_category, "Custom Resources");
        assert!(!definitions_custom);
        assert!(definitions_crd);

        let (resource_category, resource_custom, resource_crd) =
            category_for("example.platform.io", "widgets");
        assert_eq!(resource_category, "Custom Resources");
        assert!(resource_custom);
        assert!(resource_crd);
    }

    #[test]
    fn gives_gateway_api_resources_their_own_catalog_category() {
        let (category, custom, crd) = category_for("gateway.networking.k8s.io", "httproutes");
        assert_eq!(category, "Gateway APIs");
        assert!(custom);
        assert!(crd);

        let (gateway_category, _, _) = category_for("gateway.networking.k8s.io", "gateways");
        assert_eq!(gateway_category, "Gateway APIs");
    }

    #[test]
    fn builds_label_selectors_from_labels_and_match_expressions() {
        let manifest = serde_json::json!({
            "spec": {
                "selector": {
                    "matchLabels": { "app": "api" },
                    "matchExpressions": [
                        { "key": "track", "operator": "In", "values": ["stable", "canary"] },
                        { "key": "debug", "operator": "DoesNotExist" }
                    ]
                }
            }
        });
        assert_eq!(
            workload_label_selector(&manifest).expect("valid selector"),
            "app=api,track in (stable,canary),!debug"
        );
    }

    #[test]
    fn calculates_cpu_and_memory_utilization_from_kubernetes_quantities() {
        let cpu = usage_percent(Some("250m"), Some("4"), true).expect("CPU percentage");
        let memory = usage_percent(Some("512Mi"), Some("2Gi"), false).expect("memory percentage");

        assert!((cpu - 6.25).abs() < f64::EPSILON);
        assert!((memory - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reports_a_missing_oidc_exec_helper_with_an_actionable_hint() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after the epoch")
            .as_nanos();
        let kubeconfig_path = std::env::temp_dir().join(format!("kuberniva-oidc-{suffix}.yaml"));
        fs::write(
            &kubeconfig_path,
            exec_kubeconfig("kuberniva-helper-that-does-not-exist"),
        )
        .expect("write exec kubeconfig");
        let kubeconfig = Kubeconfig::read_from(&kubeconfig_path).expect("read exec kubeconfig");

        let error = validate_exec_command(&kubeconfig, Some("oidc"))
            .expect_err("missing helper should be reported");

        assert!(error.contains("kuberniva-helper-that-does-not-exist"));
        assert!(error.contains("Install the OIDC helper."));
        fs::remove_file(kubeconfig_path).expect("remove temporary kubeconfig");
    }

    #[test]
    fn removes_one_context_without_creating_a_backup() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after the epoch")
            .as_nanos();
        let kubeconfig_path = std::env::temp_dir().join(format!("kuberniva-remove-{suffix}.yaml"));
        let original = "apiVersion: v1\nkind: Config\nclusters:\n  - name: retired\n    cluster:\n      server: https://retired.example.invalid\n  - name: current\n    cluster:\n      server: https://current.example.invalid\nusers:\n  - name: retired-user\n    user:\n      token: retired\n  - name: current-user\n    user:\n      token: current\ncontexts:\n  - name: retired\n    context:\n      cluster: retired\n      user: retired-user\n  - name: current\n    context:\n      cluster: current\n      user: current-user\ncurrent-context: retired\n";
        fs::write(&kubeconfig_path, original).expect("write kubeconfig");

        forget_kubeconfig_context(ForgetKubeconfigContextRequest {
            kubeconfig_path: kubeconfig_path.to_string_lossy().into_owned(),
            context: "retired".to_string(),
        })
        .expect("remove retired context");
        let updated: Kubeconfig = serde_yaml::from_str(
            &fs::read_to_string(&kubeconfig_path).expect("read updated kubeconfig"),
        )
        .expect("parse updated kubeconfig");

        assert_eq!(
            updated
                .contexts
                .iter()
                .map(|context| context.name.as_str())
                .collect::<Vec<_>>(),
            vec!["current"]
        );
        assert_eq!(
            updated
                .clusters
                .iter()
                .map(|cluster| cluster.name.as_str())
                .collect::<Vec<_>>(),
            vec!["current"]
        );
        assert_eq!(
            updated
                .auth_infos
                .iter()
                .map(|user| user.name.as_str())
                .collect::<Vec<_>>(),
            vec!["current-user"]
        );
        assert_eq!(updated.current_context.as_deref(), Some("current"));
        fs::remove_file(kubeconfig_path).expect("remove temporary kubeconfig");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            read_kubeconfig_contexts,
            forget_kubeconfig_context,
            invalidate_cluster_client,
            discover_cluster_catalog,
            read_cluster_overview,
            read_cluster_events,
            list_resource_objects,
            get_resource_detail,
            delete_resource_object,
            save_resource_detail,
            save_resource_yaml,
            list_workload_pods,
            read_pod_logs,
            get_pod_runtime,
            exec_pod_command,
            start_port_forward,
            list_port_forwards,
            stop_port_forward
        ])
        .run(tauri::generate_context!())
        .expect("error while running Kuberniva");
}
