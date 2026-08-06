<p align="center">
  <img src="public/kuberniva-mark.png" alt="Kuberniva logo" width="160">
</p>

<h1 align="center">Kuberniva</h1>

<p align="center"><strong>Kubernetes, in focus.</strong><br>Fast, native cluster operations for people who work across many Kubernetes environments.</p>

Kuberniva is an open-source, local-first desktop workspace for operating Kubernetes clusters. It is designed for people who move between many kubeconfig contexts and need a fast, readable surface for cluster health, workloads, resources, events, logs, and safe day-to-day operations.

The current desktop release is <code>0.2.4</code>.

## Features

### Cluster and kubeconfig management

- Add one kubeconfig file or a directory containing many kubeconfig files.
- Discover every valid context in a selected source and merge new sources into the existing workspace.
- Keep multiple kubeconfig sources tracked without replacing previously loaded contexts.
- Expand <code>~</code>, <code>~/…</code>, relative paths, and absolute paths consistently.
- View the context name, backing cluster, authentication method, source path, and connection status.
- Select a cluster from the top active-cluster selector, Cluster Manager, or a saved favorite shortcut.
- Connect lazily: reading kubeconfig metadata does not authenticate to every cluster. The Kubernetes client and OIDC flow start only when a cluster is selected.
- Support kubeconfig authentication based on OIDC <code>exec</code>, OIDC auth-provider configuration, bearer tokens, and client certificates.
- Validate missing <code>exec</code> helpers and return an actionable error when the configured command is not available.
- Cache Kubernetes clients by kubeconfig path and context for fast switching while the app is open.
- Invalidate a cached client and reconnect the current cluster on demand.
- Remove an outdated context from a single kubeconfig file with a two-step, name-confirmed action. Removal does not create a backup copy.
- Restore the local workspace snapshot at startup without rescanning sources. File and folder rescans happen only when the user explicitly adds or syncs a source.

### Favorites and shortcuts

- Pin up to 10 clusters from Available Clusters with the star control.
- Open a favorite directly from the Favorites section in the left sidebar.
- Open the full Favorites view when a larger shortcut list is useful.
- Right-click a favorite to rename its shortcut to a friendly operational name such as <code>Production · East</code> or <code>Payments staging</code>.
- Remove a shortcut without removing the underlying kubeconfig context.
- Persist favorite order and custom names locally across app restarts.

### Cluster overview

- See the active cluster, node count, ready-node count, and Metrics API availability at a glance.
- Open Workloads and Resources side by side from the overview entry cards.
- Refresh the overview on demand; while the Overview page is open, node data refreshes at most once per minute.
- Inspect every node in a selectable node workbench rather than a compressed table.
- View node readiness, roles, architecture, operating system, OS image, kernel, kubelet, container runtime, Pod CIDRs, provider ID, UID, and schedulability.
- Inspect node labels, network addresses, conditions, taints, capacity, and allocatable resources.
- View CPU and memory usage values with percentage meters, used-versus-capacity text, and remaining capacity.
- Continue to see capacity when the cluster does not expose the Kubernetes Metrics API.

### Cluster events

- Use the dedicated Events navigation item in the left Cluster workspace.
- Fetch the latest retained Kubernetes Events from the selected cluster only when the Events page is opened.
- Display event type, reason, message, involved object, namespace, action, source, occurrence count, and timestamps.
- Filter by Warning, Normal, or All events.
- Search event messages, reasons, objects, and namespaces.
- Refresh the event feed on demand. The backend requests up to 250 recent event objects per fetch.

### Kubernetes resource browser

- Discover listable Kubernetes APIs from server discovery instead of relying on a hardcoded resource list.
- Include built-in APIs, namespaced and cluster-scoped resources, and CRD-backed custom resources.
- Organize APIs into Workloads, Configuration, Access Control, Network, Storage, Cluster, and Custom Resources.
- Show API version, resource kind, plural name, scope, and CRD status.
- Filter resource kinds quickly and browse live objects only after selecting a kind.
- Select All namespaces or a specific namespace. Namespace selection is preserved per cluster.
- Search objects by name and namespace.
- Inspect live object properties in the resource workbench without navigating away from the workspace.
- Show resource metadata, labels, annotations, owners, network ports, workload references, and other available properties.
- View and edit YAML for live resources, then save the YAML back through the Kubernetes API.
- Delete resources only through a two-step confirmation that requires typing the resource name.

### ConfigMaps, Secrets, and certificates

- Open ConfigMaps and Secrets in a structured key/value editor.
- Add, edit, and remove data entries, then save changes directly to the selected cluster.
- Keep Secret values base64 encoded by default.
- Reveal decoded Secret values only on demand, edit them locally, and encode them again when saving.
- Keep decoded Secret values in the local editing flow; Kuberniva does not store them as workspace preferences.
- Detect certificate expiry from certificate resources and TLS Secret data.
- Show certificate expiration date, days remaining, and expired state.
- View YAML and delete supported configuration resources from the same inspector.

### Workloads

- Browse workload APIs such as Pods, Deployments, StatefulSets, DaemonSets, ReplicaSets, Jobs, CronJobs, and other discovered workload types.
- Load workload objects on demand for the selected namespace.
- Use a card-based workload inventory with name and namespace search.
- Open a live workload detail inspector instead of being sent straight to logs.
- See replica readiness, API version, labels, container images, and attached ConfigMaps and Secrets.
- Resolve a Deployment or other selector-based workload to its matching Pods.
- Keep the workload resource type and selected objects available while moving through the inspector.
- Delete workloads through the same name-confirmed two-step flow.

### Logs, terminals, and networking

- Open logs from a Pod or from a workload after Kuberniva resolves its live Pods.
- Keep sibling Pods in a left-side list so switching streams does not require leaving the log view.
- Select a container when a Pod has sidecars or multiple containers.
- Load timestamped log output with a 750-line initial tail and up to 5,000 lines retained in the UI.
- Refresh logs manually or automatically every 30 seconds.
- Preserve scroll position when new output arrives; if already at the bottom, follow the stream.
- Show declared container ports and identify the owning container.
- Start and stop native Kubernetes Pod port forwards with separate local and remote ports.
- Keep active forwards in a top-bar tray and stop them explicitly or when Kuberniva exits.
- Open a Pod terminal through Kubernetes <code>exec</code> using <code>/bin/sh -lc</code>.
- Choose a target Pod and container before running a command.
- Enforce a 30-second terminal command timeout and display stdout/stderr together.

### Workspace and interaction design

- Native Tauri desktop window with a resizable, collapsible left sidebar.
- Persistent top-bar active-cluster selector and workspace breadcrumbs.
- Full-height desktop layout that adapts to the available window size.
- Light ivory/golden workspace palette with lavender, cyan, and teal operational accents.
- Keyboard-friendly controls, clear loading states, inline errors, and transient action feedback.
- Command/search surface for jumping to resources and workspaces.
- Notification bell appears only when actionable notifications exist.
- Separate focused workspaces for Overview, Events, Workloads, Resources, Logs, Favorites, Cluster Manager, and Settings.

## Architecture

Kuberniva is deliberately split into a small Svelte frontend and a native Rust/Tauri backend. The frontend owns interaction state and rendering; all Kubernetes API access, kubeconfig parsing, authentication setup, streaming, and port forwarding stay in the Rust process.

~~~mermaid
flowchart LR
    UI["Svelte 5 + TypeScript UI"] -->|Tauri invoke| HOST["Tauri 2 native host"]
    HOST --> KC["Kubeconfig parser and context resolver"]
    HOST --> AUTH["kube Config + OIDC exec/auth-provider"]
    AUTH --> CLIENT["Cached kube::Client by source + context"]
    CLIENT --> DISC["Kubernetes discovery"]
    CLIENT --> API["Kubernetes APIs"]
    API --> OVERVIEW["Nodes and Metrics API"]
    API --> EVENTS["Core Events API"]
    API --> RES["Dynamic resources and CRDs"]
    API --> WORK["Workloads, Pods, logs, exec, port-forward"]
    UI --> STORE["Local workspace snapshot"]
~~~

### Frontend

- <code>src/App.svelte</code> contains the Svelte workspace, navigation, cluster/session state, resource workbench, workload inspector, event feed, node inspector, log stream, terminal surface, and modal confirmations.
- <code>src/app.css</code> contains the responsive desktop design system, light palette, operational cards, node metrics, event feed, resource workbench, workload inspector, and full-window layout rules.
- Svelte state is demand-driven: selecting a cluster, namespace, resource kind, workload, node, event page, or log target starts only the request required for that surface.
- In-memory caches keep cluster catalogs, object lists, sessions, selected namespaces, and active UI state available while switching clusters.
- <code>localStorage</code> stores only workspace preferences and the cached kubeconfig context snapshot: tracked sources, cached contexts, sidebar state, selected namespaces, and favorite shortcuts.

### Native backend

- <code>src-tauri/src/lib.rs</code> is the Rust command layer exposed to the frontend through Tauri.
- <code>kube</code> provides Kubernetes configuration, discovery, dynamic resources, typed core resources, logs, exec, and port forwarding.
- <code>k8s-openapi</code> provides typed Kubernetes objects for Nodes, Namespaces, Pods, Events, and node status details.
- <code>rustls-tls</code> provides the Kubernetes client transport without requiring a separate proxy service.
- <code>serde</code>, <code>serde_json</code>, and <code>serde_yaml</code> convert kubeconfig, dynamic objects, manifests, and editor payloads.
- <code>x509-parser</code> inspects TLS certificates stored in Secret data.
- A bounded client cache holds up to 64 source/context clients before clearing and rebuilding the cache.

### Tauri command surface

| Command | Responsibility |
| --- | --- |
| <code>read_kubeconfig_contexts</code> | Read one kubeconfig or every valid kubeconfig file in a directory. |
| <code>forget_kubeconfig_context</code> | Remove one context and its unused cluster/user entries from a source file. |
| <code>invalidate_cluster_client</code> | Drop one cached Kubernetes client before reconnecting. |
| <code>discover_cluster_catalog</code> | Run Kubernetes discovery, enumerate namespaces, and classify listable resources/CRDs. |
| <code>read_cluster_overview</code> | Fetch Nodes and optional Metrics API data, including detailed node metadata. |
| <code>read_cluster_events</code> | Fetch and sort recent cluster Events. |
| <code>list_resource_objects</code> | List dynamic objects for a namespaced or cluster-scoped resource. |
| <code>get_resource_detail</code> | Fetch a live object, serialize its manifest/YAML, and calculate certificate information. |
| <code>save_resource_detail</code> | Replace a live dynamic object from structured editor data. |
| <code>save_resource_yaml</code> | Parse YAML and replace a live dynamic object. |
| <code>delete_resource_object</code> | Delete a namespaced or cluster-scoped dynamic object. |
| <code>list_workload_pods</code> | Resolve a workload selector to matching Pods. |
| <code>read_pod_logs</code> | Fetch timestamped logs, containers, and declared ports. |
| <code>get_pod_runtime</code> | Read Pod containers and ports for terminal/sidecar selection. |
| <code>exec_pod_command</code> | Run a bounded <code>/bin/sh -lc</code> command through Kubernetes exec. |
| <code>start_port_forward</code> / <code>stop_port_forward</code> | Manage local TCP listeners connected to Pod ports. |

### Authentication and data flow

1. Kuberniva reads context metadata locally and shows available clusters without connecting to every server.
2. Selecting a cluster resolves its source path and context and calls <code>kube::Config</code> for that context.
3. If the context has an OIDC <code>exec</code> helper, the helper is resolved through the desktop process PATH and invoked only for that selected cluster.
4. The resulting <code>kube::Client</code> is cached by normalized kubeconfig path and context.
5. Frontend workspaces invoke narrowly scoped commands for discovery, resources, events, nodes, logs, exec, or forwarding.
6. Switching clusters changes the client key and restores the previous in-memory session and persisted namespace for that cluster.

## Repository layout

~~~text
Kuberniva/
├── public/                 # App logo and static assets
├── src/
│   ├── App.svelte          # Main Svelte workspace
│   └── app.css             # UI and responsive layout styles
├── src-tauri/
│   ├── src/lib.rs          # Rust/Tauri commands and Kubernetes integration
│   ├── capabilities/       # Tauri permissions
│   ├── icons/               # Native app icons
│   ├── Cargo.toml           # Rust dependencies and package metadata
│   └── tauri.conf.json     # Desktop window and bundle configuration
├── index.html
├── package.json
├── LICENSE                  # MIT license
└── README.md
~~~

## Run from source

Requirements: Node.js, npm, Rust, and a macOS development environment for the native app.

~~~bash
npm install
npm run tauri dev
~~~

For a browser-only frontend preview:

~~~bash
npm run dev
~~~

The browser preview renders the UI, but live kubeconfig connections, OIDC execution, Kubernetes discovery, logs, terminal access, and port forwarding require the Tauri desktop host.

## Development checks

~~~bash
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
~~~

## Build and share the macOS app

Build the native Apple Silicon app bundle:

~~~bash
npm run tauri build -- --bundles app
~~~

The app bundle is generated at:

~~~text
src-tauri/target/release/bundle/macos/Kuberniva.app
~~~

To create a shareable ZIP:

~~~bash
mkdir -p delivery
ditto -c -k --sequesterRsrc --keepParent src-tauri/target/release/bundle/macos/Kuberniva.app delivery/Kuberniva_VERSION_aarch64.zip
~~~

On another Apple Silicon Mac:

1. Extract the ZIP.
2. Move <code>Kuberniva.app</code> to Applications.
3. Right-click the app and choose **Open** the first time.
4. If Gatekeeper blocks it, use **System Settings → Privacy & Security → Open Anyway**.

The local development package is ad-hoc signed. Developer ID signing and notarization are needed for a public distribution with no first-launch Gatekeeper confirmation. The app does not bundle kubeconfig files; each user adds their own sources after installation.

## License

Kuberniva is released under the [MIT License](LICENSE).
