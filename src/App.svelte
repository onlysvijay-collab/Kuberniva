<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import type { Window as TauriWindow } from '@tauri-apps/api/window';
  import { Bell, Boxes, Cable, ChevronDown, ChevronRight, Command, Container, Database, LayoutDashboard, Maximize2, Menu, Minimize2, Minus, Moon, RefreshCw, Search, ScrollText, Settings2, Star, Sun, Terminal, Workflow } from '@lucide/svelte';

  type View = 'Clusters' | 'Favorites' | 'Overview' | 'Events' | 'Resources' | 'Workloads' | 'Explore' | 'Logs' | 'CLI' | 'Port forwards' | 'Settings';
  type ThemeMode = 'light' | 'dark';
  type ResourceCategory = 'Workloads' | 'Configuration' | 'Access Control' | 'Network' | 'Gateway APIs' | 'Storage' | 'Cluster' | 'Custom Resources';
  type ResourceDescriptor = { group: string; version: string; apiVersion: string; kind: string; plural: string; namespaced: boolean; category: ResourceCategory; custom: boolean; crd: boolean };
  type ClusterCatalog = { context: string; namespaces: string[]; resources: ResourceDescriptor[] };
  type ResourceObject = {
    name: string;
    namespace?: string;
    createdAt?: string;
    status?: string;
    readyContainers?: number;
    totalContainers?: number;
    restarts?: number;
    cpuUsage?: string;
    memoryUsage?: string;
    nodeName?: string;
  };
  type PodPort = { container: string; name?: string; port: number; protocol: string };
  type PodLogResponse = { lines: string[]; containers: string[]; selectedContainer?: string; ports: PodPort[] };
  type PodRuntime = { containers: string[]; ports: PodPort[] };
  type PodExecResponse = { stdout: string; stderr: string };
  type PortForward = { id: string; context?: string; localAddress: string; localPort: number; remotePort: number; namespace: string; pod: string };
  type OpeningLogsTarget = { key: string; label: string };
  type LogTarget = { pod: string; namespace: string };
  type CertificateInfo = { expiresAt: string; daysRemaining: number; expired: boolean };
  type ResourceDetail = { manifest: Record<string, unknown>; yaml: string; certificate?: CertificateInfo };
  type EditorEntry = { key: string; value: string };
  type NodeProperty = { key: string; value: string };
  type NodeAddress = { type: string; address: string };
  type NodeCondition = { type: string; status: string; reason?: string; message?: string; lastHeartbeatTime?: string; lastTransitionTime?: string };
  type NodeTaint = { key: string; value?: string; effect: string; timeAdded?: string };
  type NodeOverview = { name: string; ready: boolean; roles: string[]; labels: NodeProperty[]; annotations: NodeProperty[]; addresses: NodeAddress[]; conditions: NodeCondition[]; taints: NodeTaint[]; architecture?: string; operatingSystem?: string; osImage?: string; kernelVersion?: string; kubeletVersion?: string; containerRuntimeVersion?: string; podCidrs: string[]; providerId?: string; unschedulable: boolean; uid?: string; creationTimestamp?: string; capacity: NodeProperty[]; allocatable: NodeProperty[]; cpuCapacity?: string; memoryCapacity?: string; cpuUsage?: string; memoryUsage?: string; cpuUsagePercent?: number; memoryUsagePercent?: number };
  type ClusterTotals = { cpuCapacity?: string; memoryCapacity?: string; storageCapacity?: string; cpuUsage?: string; memoryUsage?: string; cpuUsagePercent?: number; memoryUsagePercent?: number; metricNodes: number };
  type NetworkFact = { label: string; value: string; tone: 'neutral' | 'primary' | 'external' };
  type ClusterOverview = { nodes: NodeOverview[]; totals: ClusterTotals; metricsAvailable: boolean; observedAt: string };
  type ClusterEvent = { name: string; namespace?: string; eventType: string; reason?: string; message?: string; involvedKind?: string; involvedName?: string; action?: string; count?: number; source?: string; firstObserved?: string; lastObserved?: string };
  type KubeContext = { name: string; cluster: string; namespace: string; authMethod: string; current: boolean; sourcePath?: string };
  type KubeconfigSummary = { contexts: KubeContext[]; currentContext?: string };
  type KubeconfigInputMode = 'file' | 'folder' | 'paste';
  type Cluster = { id: string; name: string; provider: string; status: string; tone: string; authMethod?: string; namespace?: string; kubeconfigPath?: string; sourceId?: string };
  type GlobalSearchResult = { type: 'resource' | 'object'; resource: ResourceDescriptor; object?: ResourceObject; title: string; detail: string };
  type KubeCliResponse = { stdout: string; stderr: string; exitCode?: number; success: boolean };
  type ResourceWatchSignal = { watchId: string; action: string; error?: string };
  type ResourceWatchStatus = 'idle' | 'connecting' | 'connected' | 'reconnecting' | 'error';
  type ClusterSession = { namespace: string; selectedCategory: ResourceCategory | 'All resources'; resourceSearch: string; workloadResource: ResourceDescriptor | null; workloadObjects: ResourceObject[]; workloadSearch: string; clusterOverview: ClusterOverview | null };
  type PersistedWorkspace = { version: 6; sourceConfigured: boolean; kubeconfigPath: string; kubeconfigPaths: string[]; clusters: Cluster[]; sidebarWidth?: number; sidebarHidden?: boolean; clusterNamespaces?: Record<string, string>; favoriteClusterIds?: string[]; favoriteClusterNames?: Record<string, string>; theme?: ThemeMode };
  type DeletionTarget =
    | { type: 'resource'; resource: ResourceDescriptor; object: ResourceObject }
    | { type: 'cluster'; cluster: Cluster };

  let activeView: View = 'Overview';
  let activeCluster = 'No cluster connected';
  let search = '';
  let namespace = 'all namespaces';
  let namespaceOpen = false;
  let clusterPickerOpen = false;
  let commandOpen = false;
  let commandQuery = '';
  let globalSearchResults: GlobalSearchResult[] = [];
  let kubeconfigOpen = false;
  let kubeconfigInputMode: KubeconfigInputMode = 'file';
  let kubeconfigPath = '';
  let pastedKubeconfig = '';
  let kubeconfigSources: string[] = [];
  let sourceConfigured = false;
  let restoringWorkspace = true;
  let resourceSearch = '';
  let selectedCategory: ResourceCategory | 'All resources' = 'All resources';
  let sidebarWorkloadMenuOpen = false;
  let sidebarResourceMenuOpen = false;
  let sidebarResourceSearch = '';
  let sidebarResourceCategory: ResourceCategory | 'All resources' = 'All resources';
  let theme: ThemeMode = 'light';
  let loadingCatalog = false;
  let selectedResource: ResourceDescriptor | null = null;
  let resourceObjects: ResourceObject[] = [];
  let loadingObjects = false;
  let resourceRequestGeneration = 0;
  let relatedPods: ResourceObject[] | null = null;
  let loadingRelatedPods = false;
  let logTarget: LogTarget | null = null;
  let logPods: ResourceObject[] = [];
  let logPorts: PodPort[] = [];
  let logScopeLabel = '';
  let logLines: string[] = [];
  let logContainers: string[] = [];
  let selectedLogContainer: string | undefined;
  let loadingLogs = false;
  let openingLogsTarget: OpeningLogsTarget | null = null;
  let logViewport: HTMLPreElement;
  let logRefreshTimer: ReturnType<typeof window.setInterval> | undefined;
  let logRequestGeneration = 0;
  let logWorkspaceGeneration = 0;
  let portForwardOpen = false;
  let portForwarding = false;
  let portForwardRemotePort = '';
  let portForwardLocalPort = '';
  let portForwards: PortForward[] = [];
  let syncingPortForwards = false;
  let stoppingPortForwardId = '';
  let workloadDetailMode: 'overview' | 'terminal' | 'logs' = 'overview';
  let terminalPods: ResourceObject[] = [];
  let terminalTarget: LogTarget | null = null;
  let terminalContainers: string[] = [];
  let terminalPorts: PodPort[] = [];
  let selectedTerminalContainer = '';
  let terminalCommand = 'id && uname -a';
  let terminalOutput = '';
  let loadingTerminalPods = false;
  let loadingTerminalRuntime = false;
  let runningTerminalCommand = false;
  let cliCommand = 'kubectl get pods';
  let cliOutput = '';
  let runningCli = false;
  let cliShellMode = false;
  let editorResource: ResourceDescriptor | null = null;
  let editorObject: ResourceObject | null = null;
  let editorManifest: Record<string, unknown> | null = null;
  let editorEntries: EditorEntry[] = [];
  let focusedEditorEntry = 0;
  let editorCertificate: CertificateInfo | undefined;
  let loadingEditor = false;
  let savingEditor = false;
  let revealSecret = false;
  let connectedKubeconfig = false;
  let toast = '';
  // Toasts are transient feedback. The bell is reserved for actionable items,
  // so it stays out of the header unless something genuinely needs attention.
  let notifications: string[] = [];
  let activeKubeconfigPath: string | undefined;
  let activeClusterId = '';
  let catalogError = '';
  let sidebarWidth = 280;
  let workloadObjects: ResourceObject[] = [];
  let workloadResource: ResourceDescriptor | null = null;
  let loadingWorkloads = false;
  let workloadRequestGeneration = 0;
  let workloadSearch = '';
  let sidebarHidden = false;
  let persistedClusterNamespaces: Record<string, string> = {};
  let favoriteClusterIds: string[] = [];
  let favoriteClusterNames: Record<string, string> = {};
  let favoriteContextMenu: { clusterId: string; x: number; y: number } | null = null;
  let favoriteRenameId = '';
  let favoriteRenameValue = '';
  let selectedNodeName = '';
  let clusterEvents: ClusterEvent[] = [];
  let loadingEvents = false;
  let eventsError = '';
  let eventsObservedAt = '';
  let eventsClusterId = '';
  let eventSearch = '';
  let eventTypeFilter: 'All' | 'Warning' | 'Normal' = 'All';
  let clusterOverview: ClusterOverview | null = null;
  let overviewError = '';
  let loadingOverview = false;
  let overviewRequestGeneration = 0;
  let eventsRequestGeneration = 0;
  let refreshingCluster = false;
  let overviewRefreshTimer: ReturnType<typeof window.setInterval> | undefined;
  let resourceWatchId = '';
  let resourceWatchKey = '';
  let resourceWatchGeneration = 0;
  let resourceWatchRefreshTimer: ReturnType<typeof window.setTimeout> | undefined;
  let resourceWatchRefreshPending = false;
  let resourceWatchUnlisten: (() => void) | undefined;
  let resourceWatchListenerReady: Promise<void> | null = null;
  let resourceWatchErrorNotified = false;
  const resourceWatchSignalBuffer = new Map<string, ResourceWatchSignal>();
  let resourceWatchStatus: ResourceWatchStatus = 'idle';
  let lastHiddenAt = 0;
  let resumeRecoveryTimer: ReturnType<typeof window.setTimeout> | undefined;
  let relatedObject: ResourceObject | null = null;
  let yamlResource: ResourceDescriptor | null = null;
  let yamlObject: ResourceObject | null = null;
  let yamlText = '';
  let yamlOriginal = '';
  let yamlMode: 'view' | 'edit' = 'view';
  let loadingYaml = false;
  let savingYaml = false;
  let deletionTarget: DeletionTarget | null = null;
  let deletionStep: 1 | 2 = 1;
  let deletionName = '';
  let deletingResource = false;
  let desktopWindow: TauriWindow | null = null;
  let windowControlsAvailable = false;
  let isWindowMaximized = false;
  let stopWindowResizeListening: (() => void) | undefined;
  let windowResizeStateTimer: ReturnType<typeof window.setTimeout> | undefined;
  const catalogCache = new Map<string, ClusterCatalog>();
  const clusterSessionCache = new Map<string, ClusterSession>();
  const resourceObjectCache = new Map<string, ResourceObject[]>();
  const workspaceStorageKey = 'kuberniva.workspace.v1';
  const themeStorageKey = 'kuberniva.theme.v1';

  const resourceCategories: ResourceCategory[] = ['Configuration', 'Access Control', 'Network', 'Gateway APIs', 'Storage', 'Cluster', 'Custom Resources'];
  let clusters: Cluster[] = [];
  let catalog: ClusterCatalog = { context: '', namespaces: [], resources: [] };

  $: favoriteClusters = favoriteClusterIds
    .map((id) => clusters.find((cluster) => cluster.id === id))
    .filter((cluster): cluster is Cluster => Boolean(cluster))
    .slice(0, 10);
  $: favoriteContextCluster = favoriteContextMenu ? clusters.find((cluster) => cluster.id === favoriteContextMenu?.clusterId) : null;
  $: selectedNode = clusterOverview?.nodes.find((node) => node.name === selectedNodeName) || clusterOverview?.nodes[0] || null;
  $: namespaceClusterEvents = clusterEvents.filter((event) =>
    namespace === 'all namespaces' || !event.namespace || event.namespace === namespace,
  );
  $: visibleClusterEvents = namespaceClusterEvents.filter((event) =>
    (eventTypeFilter === 'All' || event.eventType === eventTypeFilter) &&
    `${event.reason || ''} ${event.message || ''} ${event.involvedKind || ''} ${event.involvedName || ''} ${event.namespace || ''}`.toLowerCase().includes(eventSearch.toLowerCase()),
  );
  $: resourceWorkspaceResources = catalog.resources.filter((resource) => resource.category !== 'Workloads');
  $: sidebarVisibleResources = resourceWorkspaceResources.filter((resource) =>
    (sidebarResourceCategory === 'All resources' || resource.category === sidebarResourceCategory) &&
    resourceSearchText(resource).includes(sidebarResourceSearch.toLowerCase()),
  ).sort((left, right) => left.category.localeCompare(right.category) || left.kind.localeCompare(right.kind));
  $: globalSearchResults = buildGlobalSearchResults(commandQuery, resourceWorkspaceResources, selectedResource, resourceObjects, workloadResource, workloadObjects);
  $: categoryCounts = Object.fromEntries(resourceCategories.map((category) => [category, resourceWorkspaceResources.filter((resource) => resource.category === category).length]));
  $: showClusterWorkspaceControls = Boolean(activeClusterId) && ['Overview', 'Events', 'Resources', 'Workloads', 'Logs', 'CLI'].includes(activeView);
  $: refreshingCurrentView = refreshingCluster || loadingCatalog || (activeView === 'Overview'
    ? loadingOverview
    : activeView === 'Events'
      ? loadingEvents
      : activeView === 'Resources'
        ? loadingObjects
        : activeView === 'Workloads'
          ? loadingWorkloads
          : activeView === 'Logs'
            ? loadingLogs
            : false);
  $: namespaceControlBusy = loadingCatalog
    || (activeView === 'Resources' && loadingObjects)
    || (activeView === 'Workloads' && loadingWorkloads)
    || (activeView === 'Logs' && loadingLogs)
    || loadingEditor
    || savingEditor
    || loadingYaml
    || savingYaml;
  $: readyNodeCount = clusterOverview?.nodes.filter((node) => node.ready).length || 0;
  $: workloadResources = catalog.resources
    .filter((resource) => resource.category === 'Workloads')
    .sort((left, right) => {
      const preferredOrder = ['Deployment', 'StatefulSet', 'DaemonSet', 'Job', 'CronJob', 'Pod', 'ReplicaSet', 'ReplicationController'];
      const leftIndex = preferredOrder.indexOf(left.kind);
      const rightIndex = preferredOrder.indexOf(right.kind);
      return (leftIndex === -1 ? 99 : leftIndex) - (rightIndex === -1 ? 99 : rightIndex) || left.kind.localeCompare(right.kind);
    });
  $: visibleWorkloadObjects = workloadObjects.filter((workload) =>
    `${workload.name} ${workload.namespace || ''}`.toLowerCase().includes(workloadSearch.toLowerCase()),
  );
  $: suggestedForwardPorts = [...new Set(logPorts.map((port) => port.port))];
  // Native listeners can remain active for another context, but the workspace
  // must never imply they belong to the cluster currently being inspected.
  $: activeClusterPortForwards = activeClusterId
    ? portForwards.filter((forward) => forward.context === activeCluster)
    : [];
  $: selectedPodPortForwards = logTarget
    ? activeClusterPortForwards.filter((forward) => forward.pod === logTarget?.pod
      && forward.namespace === logTarget?.namespace)
    : [];
  $: editorLogsOpening = editorResource && editorObject
    ? isOpeningLogs(editorResource.kind, editorObject)
    : false;
  $: focusedEditorEntryData = editorEntries[focusedEditorEntry] || null;

  function autoSizeTextarea(node: HTMLTextAreaElement, _value: string) {
    const resize = () => {
      node.style.height = 'auto';
      const maxHeight = Math.min(560, Math.max(180, Math.round(window.innerHeight * 0.56)));
      const desiredHeight = Math.max(44, node.scrollHeight);
      node.style.height = `${Math.min(desiredHeight, maxHeight)}px`;
      node.style.overflowY = desiredHeight > maxHeight ? 'auto' : 'hidden';
    };
    resize();
    window.addEventListener('resize', resize);
    return {
      update: resize,
      destroy: () => window.removeEventListener('resize', resize),
    };
  }

  function resourceAge(createdAt?: string) {
    if (!createdAt) return '—';
    const timestamp = Date.parse(createdAt);
    if (!Number.isFinite(timestamp)) return '—';
    const elapsed = Math.max(0, Date.now() - timestamp);
    const minutes = Math.floor(elapsed / 60_000);
    if (minutes < 1) return 'now';
    if (minutes < 60) return `${minutes}m`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days}d`;
    const months = Math.floor(days / 30);
    if (months < 12) return `${months}mo`;
    return `${Math.floor(months / 12)}y`;
  }

  function logOpeningKey(kind: string, object: ResourceObject) {
    return `${kind}\u0000${object.namespace || namespace}\u0000${object.name}`;
  }

  function isOpeningLogs(kind: string, object: ResourceObject) {
    return openingLogsTarget?.key === logOpeningKey(kind, object);
  }

  function workloadStatusLabel(object: ResourceObject) {
    return object.status || 'Live';
  }

  function workloadStatusTone(object: ResourceObject) {
    const status = (object.status || 'live').toLowerCase();
    if (status === 'running' || status === 'succeeded' || status === 'live') return 'running';
    if (status === 'pending' || status === 'unknown') return 'pending';
    return 'failed';
  }

  function podContainerSummary(object: ResourceObject) {
    if (object.totalContainers === undefined) return '—';
    return `${object.readyContainers ?? 0}/${object.totalContainers}`;
  }

  function podMetricLabel(value?: string) {
    return value || '—';
  }

  function notify(message: string) {
    toast = message;
    window.setTimeout(() => (toast = ''), 2600);
  }

  function persistWorkspace() {
    try {
      if (activeClusterId) persistedClusterNamespaces = { ...persistedClusterNamespaces, [activeClusterId]: namespace };
      const workspace: PersistedWorkspace = {
        version: 6,
        sourceConfigured,
        kubeconfigPath,
        kubeconfigPaths: kubeconfigSources,
        clusters: clusters.map((cluster) => ({ ...cluster, status: 'Not connected', tone: 'gray' })),
        sidebarWidth,
        sidebarHidden,
        clusterNamespaces: persistedClusterNamespaces,
        favoriteClusterIds: favoriteClusterIds.filter((id) => clusters.some((cluster) => cluster.id === id)).slice(0, 10),
        favoriteClusterNames: Object.fromEntries(
          favoriteClusterIds
            .filter((id) => clusters.some((cluster) => cluster.id === id))
            .slice(0, 10)
            .flatMap((id) => {
              const label = favoriteClusterNames[id]?.trim();
              return label ? [[id, label]] : [];
            }),
        ),
        theme,
      };
      window.localStorage.setItem(workspaceStorageKey, JSON.stringify(workspace));
    } catch {
      // Workspace preferences are useful, but never required for a live connection.
    }
  }

  function loadWorkspacePreference(): PersistedWorkspace | null {
    try {
      const raw = window.localStorage.getItem(workspaceStorageKey);
      if (!raw) return null;
      const parsed = JSON.parse(raw) as {
        version?: number;
        sourceConfigured?: unknown;
        kubeconfigPath?: unknown;
        kubeconfigPaths?: unknown;
        clusters?: unknown;
        sidebarWidth?: unknown;
        sidebarHidden?: unknown;
        clusterNamespaces?: unknown;
        favoriteClusterIds?: unknown;
        favoriteClusterNames?: unknown;
        theme?: unknown;
      };
      if ((parsed.version !== 1 && parsed.version !== 2 && parsed.version !== 3 && parsed.version !== 4 && parsed.version !== 5 && parsed.version !== 6) || typeof parsed.sourceConfigured !== 'boolean' || typeof parsed.kubeconfigPath !== 'string') return null;
      const kubeconfigPaths = (parsed.version === 2 || parsed.version === 3 || parsed.version === 4 || parsed.version === 5 || parsed.version === 6) && Array.isArray(parsed.kubeconfigPaths)
        ? [...new Set(parsed.kubeconfigPaths.filter((path): path is string => typeof path === 'string').map((path) => path.trim()))]
        : [parsed.kubeconfigPath.trim()];
      const cachedClusters = Array.isArray(parsed.clusters)
        ? parsed.clusters.flatMap((candidate) => {
          if (!candidate || typeof candidate !== 'object' || Array.isArray(candidate)) return [];
          const cluster = candidate as Partial<Cluster>;
          if (typeof cluster.id !== 'string' || typeof cluster.name !== 'string' || typeof cluster.provider !== 'string') return [];
          return [{
            id: cluster.id,
            name: cluster.name,
            provider: cluster.provider,
            status: 'Not connected',
            tone: 'gray',
            authMethod: typeof cluster.authMethod === 'string' ? cluster.authMethod : undefined,
            namespace: typeof cluster.namespace === 'string' ? cluster.namespace : undefined,
            kubeconfigPath: typeof cluster.kubeconfigPath === 'string' ? cluster.kubeconfigPath : undefined,
            sourceId: typeof cluster.sourceId === 'string' ? cluster.sourceId : undefined,
          }];
        })
        : [];
      const clusterNamespaces = parsed.clusterNamespaces && typeof parsed.clusterNamespaces === 'object' && !Array.isArray(parsed.clusterNamespaces)
        ? Object.fromEntries(Object.entries(parsed.clusterNamespaces).filter(([id, selectedNamespace]) => Boolean(id) && typeof selectedNamespace === 'string'))
        : {};
      const favoriteClusterIds = Array.isArray(parsed.favoriteClusterIds)
        ? [...new Set(parsed.favoriteClusterIds.filter((id): id is string => typeof id === 'string' && cachedClusters.some((cluster) => cluster.id === id)))].slice(0, 10)
        : [];
      const favoriteClusterNames = parsed.favoriteClusterNames && typeof parsed.favoriteClusterNames === 'object' && !Array.isArray(parsed.favoriteClusterNames)
        ? Object.fromEntries(Object.entries(parsed.favoriteClusterNames).filter(([id, label]) => favoriteClusterIds.includes(id) && typeof label === 'string' && label.trim()).map(([id, label]) => [id, String(label).trim().slice(0, 80)]))
        : {};
      const workspace: PersistedWorkspace = {
        version: 6,
        sourceConfigured: parsed.sourceConfigured,
        kubeconfigPath: parsed.kubeconfigPath,
        kubeconfigPaths: kubeconfigPaths.length ? kubeconfigPaths : [''],
        clusters: cachedClusters,
        sidebarWidth: typeof parsed.sidebarWidth === 'number' ? parsed.sidebarWidth : undefined,
        sidebarHidden: typeof parsed.sidebarHidden === 'boolean' ? parsed.sidebarHidden : undefined,
        clusterNamespaces,
        favoriteClusterIds,
        favoriteClusterNames,
        theme: parsed.theme === 'dark' ? 'dark' : 'light',
      };
      window.localStorage.setItem(workspaceStorageKey, JSON.stringify(workspace));
      return workspace;
    } catch {
      return null;
    }
  }

  function loadThemePreference(): ThemeMode {
    try {
      return window.localStorage.getItem(themeStorageKey) === 'dark' ? 'dark' : 'light';
    } catch {
      return 'light';
    }
  }

  function sourceKey(path: string) {
    return path.trim() || '__default_kubeconfig__';
  }

  function rememberKubeconfigSource(path: string) {
    const normalizedPath = path.trim();
    if (!kubeconfigSources.some((source) => sourceKey(source) === sourceKey(normalizedPath))) {
      kubeconfigSources = [...kubeconfigSources, normalizedPath];
    }
  }

  function applyKubeconfigSummary(summary: KubeconfigSummary, source = kubeconfigPath, replaceSource = false) {
    const sourceId = sourceKey(source);
    const incoming = summary.contexts.map((context) => ({
      id: `${context.sourcePath || source || 'default'}\u0000${context.name}`,
      name: context.name,
      provider: context.cluster,
      status: 'Not connected',
      tone: 'gray',
      authMethod: context.authMethod,
      namespace: context.namespace,
      kubeconfigPath: context.sourcePath || source || undefined,
      sourceId,
    }));
    const existingClusters = new Map(clusters.map((cluster) => [cluster.id, cluster]));
    const incomingIds = new Set(incoming.map((cluster) => cluster.id));
    const mergedIncoming = incoming.map((cluster) => {
      const existing = existingClusters.get(cluster.id);
      return existing ? { ...cluster, status: existing.status, tone: existing.tone } : cluster;
    });
    const retainedClusters = clusters.filter((cluster) => {
      if (incomingIds.has(cluster.id)) return false;
      if (!replaceSource) return true;
      return cluster.sourceId !== sourceId;
    });
    clusters = [...retainedClusters, ...mergedIncoming];
    connectedKubeconfig = clusters.length > 0;
    if (!clusters.some((cluster) => cluster.id === activeClusterId)) {
      activeClusterId = '';
      activeKubeconfigPath = undefined;
      activeCluster = clusters.length ? 'Select a cluster' : 'No cluster connected';
      stopOverviewRefresh();
    }
  }

  function resourceObjectCacheKey(clusterId: string, resource: ResourceDescriptor, resourceNamespace: string) {
    return [clusterId, resource.group, resource.version, resource.plural, resourceNamespace || 'all namespaces'].join('\u0000');
  }

  function rememberActiveClusterSession() {
    if (!activeClusterId) return;
    persistedClusterNamespaces = { ...persistedClusterNamespaces, [activeClusterId]: namespace };
    clusterSessionCache.set(activeClusterId, {
      namespace,
      selectedCategory,
      resourceSearch,
      workloadResource,
      workloadObjects,
      workloadSearch,
      clusterOverview,
    });
  }

  function restoreClusterSession(cluster: Cluster) {
    const session = clusterSessionCache.get(cluster.id);
    namespace = session?.namespace || persistedClusterNamespaces[cluster.id] || cluster.namespace || 'all namespaces';
    selectedCategory = session?.selectedCategory === 'Workloads' ? 'All resources' : session?.selectedCategory || 'All resources';
    resourceSearch = session?.resourceSearch || '';
    workloadResource = session?.workloadResource || null;
    workloadObjects = session?.workloadObjects || [];
    workloadSearch = session?.workloadSearch || '';
    clusterOverview = session?.clusterOverview || null;
  }

  function clearClusterObjectCache(clusterId: string) {
    for (const cacheKey of resourceObjectCache.keys()) {
      if (cacheKey.startsWith(`${clusterId}\u0000`)) resourceObjectCache.delete(cacheKey);
    }
  }

  async function restoreWorkspace() {
    if (!('__TAURI_INTERNALS__' in window)) {
      restoringWorkspace = false;
      return;
    }
    const workspace = loadWorkspacePreference();
    if (!workspace) {
      restoringWorkspace = false;
      return;
    }
    applyTheme(workspace.theme || 'light');
    kubeconfigPath = workspace.kubeconfigPath;
    kubeconfigSources = workspace.kubeconfigPaths;
    sourceConfigured = workspace.sourceConfigured;
    sidebarWidth = Math.min(460, Math.max(230, workspace.sidebarWidth || sidebarWidth));
    sidebarHidden = workspace.sidebarHidden || false;
    persistedClusterNamespaces = workspace.clusterNamespaces || {};
    favoriteClusterIds = workspace.favoriteClusterIds || [];
    favoriteClusterNames = workspace.favoriteClusterNames || {};
    // Startup deliberately restores only the saved local snapshot. It never
    // rescans files or folders, so removed contexts stay removed and opening
    // Kuberniva remains immediate. Source reads happen only through Add or Sync.
    clusters = workspace.clusters;
    connectedKubeconfig = clusters.length > 0;
    activeCluster = clusters.length ? 'Select a cluster' : 'No cluster connected';
    activeView = clusters.length ? 'Clusters' : 'Overview';
    restoringWorkspace = false;
    if (clusters.length) {
      notify(`${clusters.length} cached context${clusters.length === 1 ? '' : 's'} restored locally. Sync sources only when you choose to.`);
    }
  }

  function usagePercentLabel(percent?: number) {
    if (percent === undefined) return 'No live metric';
    if (percent > 0 && percent < 1) return '<1% used';
    return `${Math.round(percent)}% used`;
  }

  function formatObservedTime(value?: string) {
    if (!value) return 'Time unavailable';
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  }

  function eventTone(eventType: string) {
    return eventType.toLowerCase() === 'warning' ? 'warning' : 'normal';
  }

  function remainingPercentLabel(percent?: number) {
    if (percent === undefined) return '';
    const remaining = Math.max(0, 100 - percent);
    return `${Math.round(remaining)}% left`;
  }

  function stopOverviewRefresh() {
    if (overviewRefreshTimer) window.clearInterval(overviewRefreshTimer);
    overviewRefreshTimer = undefined;
  }

  function stopLiveObjectRefresh() {
    resourceWatchGeneration += 1;
    if (resourceWatchRefreshTimer) window.clearTimeout(resourceWatchRefreshTimer);
    resourceWatchRefreshTimer = undefined;
    resourceWatchRefreshPending = false;
    const watchId = resourceWatchId;
    resourceWatchId = '';
    resourceWatchKey = '';
    resourceWatchErrorNotified = false;
    resourceWatchStatus = 'idle';
    resourceWatchSignalBuffer.clear();
    if (watchId && '__TAURI_INTERNALS__' in window) {
      void import('@tauri-apps/api/core')
        .then(({ invoke }) => invoke('stop_resource_watch', { watchId }))
        .catch(() => undefined);
    }
  }

  function startLiveObjectRefresh() {
    stopLiveObjectRefresh();
    if (!activeClusterId || !['Workloads', 'Resources'].includes(activeView)) return;
    const resource = activeView === 'Workloads' ? workloadResource : selectedResource;
    if (resource) void startResourceWatch(resource);
  }

  function resourceWatchRequest(resource: ResourceDescriptor) {
    return {
      kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
      context: activeCluster,
      group: resource.group,
      version: resource.version,
      kind: resource.kind,
      plural: resource.plural,
      namespaced: resource.namespaced,
      namespace,
    };
  }

  async function startResourceWatch(resource: ResourceDescriptor) {
    if (!activeClusterId || !('__TAURI_INTERNALS__' in window)) return;
    if (resourceWatchListenerReady) await resourceWatchListenerReady;
    const requestClusterId = activeClusterId;
    const requestNamespace = namespace;
    const nextWatchKey = `${requestClusterId}\u0000${resourceObjectCacheKey(requestClusterId, resource, requestNamespace)}`;
    if (resourceWatchId && resourceWatchKey === nextWatchKey) return;
    if (resourceWatchId) stopLiveObjectRefresh();
    const watchGeneration = resourceWatchGeneration;
    resourceWatchStatus = 'connecting';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const watchId = await invoke<string>('start_resource_watch', {
        request: resourceWatchRequest(resource),
      });
      if (
        watchGeneration !== resourceWatchGeneration
        ||
        requestClusterId !== activeClusterId
        || requestNamespace !== namespace
        || activeView !== (resource.category === 'Workloads' ? 'Workloads' : 'Resources')
        || (resource.category === 'Workloads' && (!workloadResource || resourceKey(workloadResource) !== resourceKey(resource)))
        || (resource.category !== 'Workloads' && (!selectedResource || resourceKey(selectedResource) !== resourceKey(resource)))
      ) {
        await invoke('stop_resource_watch', { watchId }).catch(() => undefined);
        return;
      }
      resourceWatchId = watchId;
      resourceWatchKey = nextWatchKey;
      resourceWatchErrorNotified = false;
      const bufferedSignal = resourceWatchSignalBuffer.get(watchId);
      resourceWatchSignalBuffer.delete(watchId);
      if (bufferedSignal) scheduleResourceWatchRefresh(bufferedSignal);
    } catch (error) {
      resourceWatchStatus = 'error';
      // A resource can be listable without watch permission. Keep the current
      // list usable and surface the limitation once instead of retrying loudly.
      if (!resourceWatchErrorNotified) {
        resourceWatchErrorNotified = true;
        notify(`Live updates unavailable for ${resource.kind}: ${String(error)}`);
      }
    }
  }

  function scheduleResourceWatchRefresh(signal: ResourceWatchSignal) {
    if (signal.watchId !== resourceWatchId) return;
    if (signal.action === 'connected') {
      resourceWatchStatus = 'connected';
      resourceWatchErrorNotified = false;
      return;
    }
    if (signal.error && !resourceWatchErrorNotified) {
      resourceWatchErrorNotified = true;
      resourceWatchStatus = 'reconnecting';
      notify(`Live updates paused: ${signal.error}`);
    }
    if (signal.error) resourceWatchStatus = 'reconnecting';
    if (!['added', 'modified', 'deleted'].includes(signal.action)) return;
    resourceWatchRefreshPending = true;
    if (resourceWatchRefreshTimer) window.clearTimeout(resourceWatchRefreshTimer);
    resourceWatchRefreshTimer = window.setTimeout(() => {
      resourceWatchRefreshTimer = undefined;
      void refreshVisibleObjectList();
    }, 120);
  }

  async function setupResourceWatchListener() {
    if (!('__TAURI_INTERNALS__' in window)) return;
    try {
      const { listen } = await import('@tauri-apps/api/event');
      resourceWatchUnlisten = await listen<ResourceWatchSignal>('kuberniva://resource-watch', ({ payload }) => {
        if (payload.watchId !== resourceWatchId) {
          resourceWatchSignalBuffer.set(payload.watchId, payload);
          return;
        }
        scheduleResourceWatchRefresh(payload);
      });
    } catch {
      resourceWatchUnlisten = undefined;
    }
  }

  function startOverviewRefresh() {
    stopOverviewRefresh();
    if (!activeClusterId || activeView !== 'Overview') return;
    overviewRefreshTimer = window.setInterval(() => void loadClusterOverview(), 60_000);
  }

  async function loadClusterOverview(force = false) {
    if (!activeClusterId || (loadingOverview && !force)) return;
    const overviewClusterId = activeClusterId;
    const requestGeneration = ++overviewRequestGeneration;
    loadingOverview = true;
    overviewError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const response = await invoke<ClusterOverview>('read_cluster_overview', {
        kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
        context: activeCluster,
      });
      if (requestGeneration === overviewRequestGeneration && overviewClusterId === activeClusterId) {
        clusterOverview = response;
        selectedNodeName = response.nodes.some((node) => node.name === selectedNodeName) ? selectedNodeName : response.nodes[0]?.name || '';
      }
    } catch (error) {
      if (requestGeneration === overviewRequestGeneration && overviewClusterId === activeClusterId) overviewError = String(error);
    } finally {
      if (requestGeneration === overviewRequestGeneration && overviewClusterId === activeClusterId) loadingOverview = false;
    }
  }

  async function loadClusterEvents(force = false) {
    if (!activeClusterId || (loadingEvents && !force)) return;
    if (!force && clusterEvents.length && eventsObservedAt && activeClusterId === eventsClusterId) return;
    const requestClusterId = activeClusterId;
    const requestGeneration = ++eventsRequestGeneration;
    loadingEvents = true;
    eventsError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const response = await invoke<ClusterEvent[]>('read_cluster_events', {
        kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
        context: activeCluster,
      });
      if (requestGeneration === eventsRequestGeneration && requestClusterId === activeClusterId) {
        clusterEvents = response;
        eventsClusterId = requestClusterId;
        eventsObservedAt = new Date().toISOString();
      }
    } catch (error) {
      if (requestGeneration === eventsRequestGeneration && requestClusterId === activeClusterId) eventsError = String(error);
    } finally {
      if (requestGeneration === eventsRequestGeneration && requestClusterId === activeClusterId) loadingEvents = false;
    }
  }

  async function chooseKubeconfig(directory: boolean) {
    if (!('__TAURI_INTERNALS__' in window)) {
      notify('The file picker is available in the Kuberniva desktop app');
      return;
    }
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selectedPath = await open({
        directory,
        multiple: false,
        title: directory ? 'Choose the kubeconfig folder' : 'Choose a kubeconfig file',
      });
      if (typeof selectedPath === 'string') kubeconfigPath = selectedPath;
    } catch (error) {
      notify(`Could not open the picker: ${String(error)}`);
    }
  }

  function closeKubeconfigModal(force = false) {
    if (loadingCatalog && !force) return;
    kubeconfigOpen = false;
    pastedKubeconfig = '';
  }

  function handleKubeconfigModalKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    closeKubeconfigModal();
  }

  function handleKubeconfigSourceTabKeydown(event: KeyboardEvent, mode: KubeconfigInputMode) {
    const modes: KubeconfigInputMode[] = ['file', 'folder', 'paste'];
    const currentIndex = modes.indexOf(mode);
    let nextIndex: number | undefined;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') nextIndex = (currentIndex + 1) % modes.length;
    if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') nextIndex = (currentIndex - 1 + modes.length) % modes.length;
    if (event.key === 'Home') nextIndex = 0;
    if (event.key === 'End') nextIndex = modes.length - 1;
    if (nextIndex === undefined) return;
    event.preventDefault();
    kubeconfigInputMode = modes[nextIndex];
    void tick().then(() => document.getElementById(`kubeconfig-source-tab-${modes[nextIndex!]}`)?.focus());
  }

  function startSidebarResize(event: PointerEvent) {
    if (sidebarHidden) return;
    const initialX = event.clientX;
    const initialWidth = sidebarWidth;
    const resize = (moveEvent: PointerEvent) => {
      sidebarWidth = Math.min(460, Math.max(230, initialWidth + moveEvent.clientX - initialX));
    };
    const stopResize = () => {
      window.removeEventListener('pointermove', resize);
      window.removeEventListener('pointerup', stopResize);
      persistWorkspace();
    };
    window.addEventListener('pointermove', resize);
    window.addEventListener('pointerup', stopResize, { once: true });
  }

  function toggleSidebar() {
    sidebarHidden = !sidebarHidden;
    persistWorkspace();
  }

  function isFavoriteCluster(clusterId: string) {
    return favoriteClusterIds.includes(clusterId);
  }

  function favoriteLabel(cluster: Cluster) {
    return favoriteClusterNames[cluster.id] || cluster.name;
  }

  function openFavoriteContextMenu(event: MouseEvent, cluster: Cluster) {
    event.preventDefault();
    const menuWidth = 190;
    const menuHeight = 92;
    favoriteContextMenu = {
      clusterId: cluster.id,
      x: Math.min(event.clientX, window.innerWidth - menuWidth - 12),
      y: Math.min(event.clientY, window.innerHeight - menuHeight - 12),
    };
  }

  function startFavoriteRename(clusterId: string) {
    const cluster = clusters.find((candidate) => candidate.id === clusterId);
    if (!cluster) return;
    favoriteRenameId = clusterId;
    favoriteRenameValue = favoriteLabel(cluster);
    favoriteContextMenu = null;
    void tick().then(() => document.querySelector<HTMLInputElement>('.favorite-rename input')?.select());
  }

  function cancelFavoriteRename() {
    favoriteRenameId = '';
    favoriteRenameValue = '';
  }

  function saveFavoriteRename() {
    const cluster = clusters.find((candidate) => candidate.id === favoriteRenameId);
    if (!cluster) return cancelFavoriteRename();
    const label = favoriteRenameValue.trim().slice(0, 80);
    favoriteClusterNames = { ...favoriteClusterNames };
    if (label && label !== cluster.name) favoriteClusterNames[cluster.id] = label;
    else delete favoriteClusterNames[cluster.id];
    persistWorkspace();
    notify(label && label !== cluster.name ? `Shortcut renamed to ${label}.` : 'Shortcut reset to the cluster name.');
    cancelFavoriteRename();
  }

  function toggleFavoriteCluster(cluster: Cluster) {
    if (isFavoriteCluster(cluster.id)) {
      favoriteClusterIds = favoriteClusterIds.filter((id) => id !== cluster.id);
      const { [cluster.id]: _removedFavoriteName, ...remainingFavoriteNames } = favoriteClusterNames;
      favoriteClusterNames = remainingFavoriteNames;
      notify(`${cluster.name} removed from Favorites.`);
    } else if (favoriteClusterIds.length >= 10) {
      notify('Favorites is limited to 10 cluster shortcuts. Remove one before adding another.');
      return;
    } else {
      favoriteClusterIds = [...favoriteClusterIds, cluster.id];
      notify(`${cluster.name} added to Favorites.`);
    }
    persistWorkspace();
  }

  function updateCluster(id: string, changes: Partial<Cluster>) {
    clusters = clusters.map((cluster) => cluster.id === id ? { ...cluster, ...changes } : cluster);
  }

  async function chooseNamespace(nextNamespace: string) {
    if (namespace === nextNamespace) {
      namespaceOpen = false;
      return;
    }
    const resourceToReload = selectedResource;
    namespace = nextNamespace;
    namespaceOpen = false;
    clusterPickerOpen = false;
    selectedResource = null;
    resourceObjects = [];
    closeEditor();
    closeYamlEditor();
    if (activeView === 'Workloads') {
      await navigateTo('Workloads');
    } else if (activeView === 'Resources' && resourceToReload) {
      await openResource(resourceToReload);
    } else if (activeView === 'Logs') {
      closeLogs();
      await navigateTo('Workloads');
    }
    rememberActiveClusterSession();
    persistWorkspace();
  }

  function selectResourceCategory(category: ResourceCategory | 'All resources') {
    selectedCategory = category;
    sidebarResourceCategory = category;
    resourceSearch = '';
    closeSidebarTypeMenus();
    clusterPickerOpen = false;
    selectedResource = null;
    resourceObjects = [];
    closeEditor();
    closeYamlEditor();
    const firstResource = resourceWorkspaceResources
      .filter((resource) => category === 'All resources' || resource.category === category)
      .sort((left, right) => left.kind.localeCompare(right.kind))[0];
    if (firstResource) void openResource(firstResource);
  }

  async function refreshClusterConnection(cluster: Cluster) {
    if (!activeClusterId || activeClusterId !== cluster.id) return false;
    const requestClusterId = cluster.id;
    resourceRequestGeneration += 1;
    workloadRequestGeneration += 1;
    overviewRequestGeneration += 1;
    eventsRequestGeneration += 1;
    stopLiveObjectRefresh();
    clearClusterObjectCache(requestClusterId);
    catalogCache.delete(requestClusterId);
    catalogError = '';
    overviewError = '';
    eventsError = '';
    loadingCatalog = true;
    updateCluster(requestClusterId, { status: 'Connecting', tone: 'blue' });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('invalidate_cluster_client', {
        kubeconfigPath: cluster.kubeconfigPath || kubeconfigPath || null,
        context: cluster.name,
      }).catch(() => undefined);
      const response = await invoke<ClusterCatalog>('discover_cluster_catalog', {
        kubeconfigPath: cluster.kubeconfigPath || kubeconfigPath || null,
        context: cluster.name,
      });
      if (requestClusterId !== activeClusterId) return false;
      catalog = response;
      catalogCache.set(requestClusterId, response);
      updateCluster(requestClusterId, { status: 'Connected', tone: 'green' });
      return true;
    } catch (error) {
      if (requestClusterId === activeClusterId) {
        catalogError = String(error);
        updateCluster(requestClusterId, { status: 'Connection failed', tone: 'red' });
        notify(`Could not refresh ${cluster.name}: ${catalogError}`);
      }
      return false;
    } finally {
      if (requestClusterId === activeClusterId) loadingCatalog = false;
    }
  }

  async function refreshCurrentView() {
    const cluster = clusters.find((candidate) => candidate.id === activeClusterId);
    if (!cluster || refreshingCluster || loadingCatalog) return;
    const requestClusterId = cluster.id;
    const requestView = activeView;
    const previousWorkloadResource = workloadResource;
    const previousSelectedResource = selectedResource;
    refreshingCluster = true;
    try {
      if (!await refreshClusterConnection(cluster)) return;
      if (requestClusterId !== activeClusterId || requestView !== activeView) return;
      if (requestView === 'Overview') {
        await loadClusterOverview(true);
        startOverviewRefresh();
        return;
      }
      if (requestView === 'Events') {
        await loadClusterEvents(true);
        return;
      }
      if (requestView === 'Workloads') {
        const resource = catalog.resources.find((candidate) => previousWorkloadResource && resourceKey(candidate) === resourceKey(previousWorkloadResource))
          || catalog.resources.find((candidate) => candidate.kind === 'Deployment')
          || catalog.resources.find((candidate) => candidate.category === 'Workloads');
        if (!resource) return;
        resourceObjectCache.delete(resourceObjectCacheKey(requestClusterId, resource, namespace));
        await loadWorkloadResource(resource);
        startLiveObjectRefresh();
        return;
      }
      if (requestView === 'Resources') {
        const resource = catalog.resources.find((candidate) => previousSelectedResource && resourceKey(candidate) === resourceKey(previousSelectedResource));
        if (!resource) {
          selectedResource = null;
          resourceObjects = [];
          return;
        }
        selectedResource = resource;
        resourceObjectCache.delete(resourceObjectCacheKey(requestClusterId, resource, namespace));
        await openResource(resource);
        return;
      }
      if (requestView === 'Logs') {
        logRequestGeneration += 1;
        loadingLogs = false;
        await loadLogs(true);
      }
    } catch (error) {
      notify(`Refresh failed for ${cluster.name}: ${String(error)}`);
    } finally {
      refreshingCluster = false;
    }
  }

  async function refreshVisibleObjectList() {
    if (
      !activeClusterId
      || loadingCatalog
      || loadingWorkloads
      || loadingObjects
      || loadingEditor
      || savingEditor
      || loadingYaml
      || savingYaml
      || deletingResource
    ) return;
    if (activeView === 'Workloads' && workloadResource && workloadDetailMode !== 'terminal') {
      const resource = workloadResource;
      resourceObjectCache.delete(resourceObjectCacheKey(activeClusterId, resource, namespace));
      await loadWorkloadResource(resource, true);
      return;
    }
    if (activeView === 'Resources' && selectedResource) {
      const resource = selectedResource;
      resourceObjectCache.delete(resourceObjectCacheKey(activeClusterId, resource, namespace));
      await openResource(resource, { silent: true });
    }
  }

  function flushPendingResourceWatchRefresh() {
    if (!resourceWatchRefreshPending || loadingCatalog || loadingWorkloads || loadingObjects || loadingEditor || savingEditor || loadingYaml || savingYaml || deletingResource) return;
    resourceWatchRefreshPending = false;
    void refreshVisibleObjectList();
  }

  function queueLiveResumeRecovery() {
    if (!activeClusterId || !['Workloads', 'Resources'].includes(activeView) || refreshingCluster || loadingCatalog) return;
    if (resumeRecoveryTimer) window.clearTimeout(resumeRecoveryTimer);
    resumeRecoveryTimer = window.setTimeout(() => {
      resumeRecoveryTimer = undefined;
      void refreshCurrentView();
    }, 250);
  }

  async function syncWindowMaximized() {
    if (!desktopWindow) return;
    try {
      isWindowMaximized = await desktopWindow.isMaximized();
    } catch {
      windowControlsAvailable = false;
    }
  }

  async function setupWindowControls() {
    if (!('__TAURI_INTERNALS__' in window)) return;
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      desktopWindow = getCurrentWindow();
      isWindowMaximized = await desktopWindow.isMaximized();
      windowControlsAvailable = true;
      stopWindowResizeListening = await desktopWindow.onResized(() => {
        if (windowResizeStateTimer) window.clearTimeout(windowResizeStateTimer);
        windowResizeStateTimer = window.setTimeout(() => void syncWindowMaximized(), 90);
      });
    } catch {
      desktopWindow = null;
      windowControlsAvailable = false;
    }
  }

  async function minimizeWindow() {
    if (!desktopWindow) return;
    try {
      await desktopWindow.minimize();
    } catch (error) {
      notify(`Could not minimize the window: ${String(error)}`);
    }
  }

  async function toggleWindowMaximized() {
    if (!desktopWindow) return;
    try {
      await desktopWindow.toggleMaximize();
      await syncWindowMaximized();
    } catch (error) {
      notify(`Could not resize the window: ${String(error)}`);
    }
  }

  function applyTheme(nextTheme: ThemeMode) {
    theme = nextTheme;
    if (typeof document !== 'undefined') document.documentElement.dataset.theme = nextTheme;
    try {
      window.localStorage.setItem(themeStorageKey, nextTheme);
    } catch {
      // A theme preference is optional and should never block the workspace.
    }
  }

  function toggleTheme() {
    applyTheme(theme === 'light' ? 'dark' : 'light');
    persistWorkspace();
    notify(`${theme === 'dark' ? 'Dark' : 'Light'} mode enabled.`);
  }

  function openCommandSearch() {
    commandQuery = '';
    commandOpen = true;
    void tick().then(() => document.getElementById('global-command-search')?.focus());
  }

  function handleCommandKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      commandOpen = false;
      commandQuery = '';
      return;
    }
    if (event.key === 'Enter' && globalSearchResults[0]) {
      event.preventDefault();
      void openGlobalSearchResult(globalSearchResults[0]);
    }
  }

  async function openGlobalSearchResult(result: GlobalSearchResult) {
    commandOpen = false;
    commandQuery = '';
    if (result.resource.category === 'Workloads') {
      workloadResource = result.resource;
      await navigateTo('Workloads');
      if (result.type === 'object' && result.object) await openObject(result.resource, result.object);
      return;
    }
    selectedCategory = result.resource.category;
    await navigateTo('Resources');
    await openResource(result.resource);
    if (result.type === 'object' && result.object) await openObject(result.resource, result.object);
  }

  function selectSidebarResourceCategory(category: ResourceCategory | 'All resources') {
    sidebarResourceCategory = category;
    sidebarResourceSearch = '';
  }

  async function runClusterCli() {
    const command = cliCommand.trim();
    if (!activeClusterId) {
      notify('Select a cluster before opening the CLI.');
      return;
    }
    if (!command) {
      notify('Enter a command, for example: kubectl get pods or helm list');
      return;
    }
    runningCli = true;
    cliOutput = `$ ${command}\n`;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const response = await invoke<KubeCliResponse>('run_cluster_command', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context: activeCluster,
          namespace: namespace === 'all namespaces' ? null : namespace,
          command,
          shell: cliShellMode,
        },
      });
      cliOutput += response.stdout || response.stderr || `(kubectl exited with code ${response.exitCode ?? 0})`;
      if (response.stderr && response.stdout) cliOutput += `\n${response.stderr}`;
    } catch (error) {
      cliOutput += `\n${String(error)}`;
    } finally {
      runningCli = false;
    }
  }

  async function navigateTo(view: View) {
    // Views are self-contained workspaces. Never leave an inspector, YAML tab,
    // stream, or flyout visually attached when the user changes context.
    clusterPickerOpen = false;
    namespaceOpen = false;
    closeSidebarTypeMenus();
    commandOpen = false;
    commandQuery = '';
    if (view !== 'Logs') closeLogs();
    closeEditor();
    closeYamlEditor();
    selectedResource = null;
    relatedPods = null;
    relatedObject = null;
    activeView = view;
    if (view === 'Port forwards') {
      stopOverviewRefresh();
      stopLiveObjectRefresh();
      void syncPortForwards(true);
      return;
    }
    if (view === 'Overview' && activeClusterId) {
      stopLiveObjectRefresh();
      void loadClusterOverview();
      startOverviewRefresh();
      return;
    }
    if (view === 'Events' && activeClusterId) {
      stopLiveObjectRefresh();
      void loadClusterEvents();
      return;
    }
    if (view !== 'Overview') stopOverviewRefresh();
    if (view === 'Resources') {
      startLiveObjectRefresh();
      return;
    }
    if (view !== 'Workloads' || !activeClusterId) {
      stopLiveObjectRefresh();
      return;
    }
    if (loadingWorkloads) return;
    const preferredResource = workloadResource
      || catalog.resources.find((resource) => resource.kind === 'Deployment')
      || catalog.resources.find((resource) => resource.category === 'Workloads');
    if (!preferredResource) return;
    await loadWorkloadResource(preferredResource);
    startLiveObjectRefresh();
  }

  async function loadWorkloadResource(resource: ResourceDescriptor, silent = false) {
    const requestGeneration = ++workloadRequestGeneration;
    const requestClusterId = activeClusterId;
    const requestNamespace = namespace;
    const requestResourceKey = resourceKey(resource);
    workloadResource = resource;
    const cacheKey = resourceObjectCacheKey(requestClusterId, resource, requestNamespace);
    const cachedObjects = resourceObjectCache.get(cacheKey);
    if (cachedObjects) {
      workloadObjects = cachedObjects;
      loadingWorkloads = false;
      return;
    }
    if (!silent) workloadObjects = [];
    if (!silent) loadingWorkloads = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const response = await invoke<ResourceObject[]>('list_resource_objects', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context: activeCluster,
          group: resource.group,
          version: resource.version,
          kind: resource.kind,
          plural: resource.plural,
          namespaced: resource.namespaced,
          namespace: requestNamespace,
        },
      });
      if (requestGeneration !== workloadRequestGeneration
        || requestClusterId !== activeClusterId
        || requestNamespace !== namespace
        || !workloadResource
        || resourceKey(workloadResource) !== requestResourceKey) return;
      workloadObjects = response;
      resourceObjectCache.set(cacheKey, response);
      if (
        editorResource
        && editorObject
        && resourceKey(editorResource) === requestResourceKey
        && !response.some((object) => object.name === editorObject?.name && object.namespace === editorObject?.namespace)
      ) {
        const removedName = editorObject.name;
        closeEditor(false);
        closeYamlEditor();
        notify(resource.kind + ' ' + removedName + ' is no longer present in this namespace');
      }
    } catch (error) {
      if (requestGeneration === workloadRequestGeneration && requestClusterId === activeClusterId) {
        notify(`Could not load ${resource.kind}s: ${String(error)}`);
      }
    } finally {
      if (requestGeneration === workloadRequestGeneration && requestClusterId === activeClusterId && !silent) loadingWorkloads = false;
      flushPendingResourceWatchRefresh();
    }
  }

  async function selectWorkloadResource(resource: ResourceDescriptor) {
    workloadSearch = '';
    // A resource-type switch starts a fresh workload workspace. Do not leave a
    // Pod/deployment inspector, terminal, or log stream attached to the
    // previous type.
    closeWorkloadLogs();
    closeEditor();
    closeYamlEditor();
    relatedPods = null;
    relatedObject = null;
    await loadWorkloadResource(resource);
    startLiveObjectRefresh();
  }

  type SidebarTypeMenu = 'workload' | 'resource';

  function sidebarTypeMenuId(menu: SidebarTypeMenu) {
    return `sidebar-${menu}-type-options`;
  }

  function sidebarTypeTriggerId(menu: SidebarTypeMenu) {
    return `sidebar-${menu}-type-trigger`;
  }

  function closeSidebarTypeMenus(restoreFocus?: SidebarTypeMenu) {
    sidebarWorkloadMenuOpen = false;
    sidebarResourceMenuOpen = false;
    sidebarResourceSearch = '';
    if (restoreFocus) void tick().then(() => document.getElementById(sidebarTypeTriggerId(restoreFocus))?.focus());
  }

  async function openSidebarTypeMenu(menu: SidebarTypeMenu, focusLast = false) {
    sidebarWorkloadMenuOpen = menu === 'workload';
    sidebarResourceMenuOpen = menu === 'resource';
    await tick();
    const options = Array.from(document.querySelectorAll<HTMLButtonElement>(`#${sidebarTypeMenuId(menu)} [role="option"]`));
    if (!options.length) return;
    const selected = options.find((option) => option.getAttribute('aria-selected') === 'true');
    (focusLast ? options.at(-1) : selected || options[0])?.focus();
  }

  function toggleSidebarTypeMenu(menu: SidebarTypeMenu) {
    const isOpen = menu === 'workload' ? sidebarWorkloadMenuOpen : sidebarResourceMenuOpen;
    if (isOpen) {
      closeSidebarTypeMenus();
      return;
    }
    sidebarWorkloadMenuOpen = menu === 'workload';
    sidebarResourceMenuOpen = menu === 'resource';
    sidebarResourceSearch = '';
  }

  function handleSidebarTypeTriggerKeydown(event: KeyboardEvent, menu: SidebarTypeMenu) {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    event.preventDefault();
    void openSidebarTypeMenu(menu, event.key === 'ArrowUp');
  }

  function handleSidebarTypeMenuKeydown(event: KeyboardEvent, menu: SidebarTypeMenu) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      closeSidebarTypeMenus(menu);
      return;
    }
    const options = Array.from(document.querySelectorAll<HTMLButtonElement>(`#${sidebarTypeMenuId(menu)} [role="option"]`));
    if (!options.length) return;
    const current = event.target instanceof Element ? event.target.closest<HTMLButtonElement>('[role="option"]') : null;
    const currentIndex = current ? options.indexOf(current) : -1;
    let nextIndex: number | null = null;
    if (event.key === 'ArrowDown') nextIndex = currentIndex < 0 ? 0 : (currentIndex + 1) % options.length;
    else if (event.key === 'ArrowUp') nextIndex = currentIndex < 0 ? options.length - 1 : (currentIndex - 1 + options.length) % options.length;
    else if (current && event.key === 'Home') nextIndex = 0;
    else if (current && event.key === 'End') nextIndex = options.length - 1;
    if (nextIndex === null) return;
    event.preventDefault();
    options[nextIndex]?.focus();
  }

  async function selectSidebarResourceType(resource: ResourceDescriptor) {
    closeSidebarTypeMenus();
    selectedCategory = resource.category;
    sidebarResourceCategory = resource.category;
    await navigateTo('Resources');
    await openResource(resource);
  }

  async function selectSidebarWorkloadType(resource: ResourceDescriptor) {
    closeSidebarTypeMenus();
    if (activeView !== 'Workloads') {
      workloadResource = resource;
      await navigateTo('Workloads');
      return;
    }
    await selectWorkloadResource(resource);
  }

  function objectNamespace(object: ResourceObject) {
    return object.namespace || (namespace === 'all namespaces' ? '' : namespace);
  }

  function resourceKey(resource: ResourceDescriptor) {
    return `${resource.group}\u0000${resource.version}\u0000${resource.plural}`;
  }

  function resourceSearchText(resource: ResourceDescriptor) {
    return `${resource.kind} ${resource.plural} ${resource.group} ${resource.version} ${resource.apiVersion} ${resource.category}`.toLowerCase();
  }

  function buildGlobalSearchResults(
    query: string,
    resources: ResourceDescriptor[],
    currentResource: ResourceDescriptor | null,
    currentObjects: ResourceObject[],
    currentWorkloadResource: ResourceDescriptor | null,
    currentWorkloadObjects: ResourceObject[],
  ): GlobalSearchResult[] {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return [];
    const results: GlobalSearchResult[] = [];
    for (const resource of resources) {
      if (!resourceSearchText(resource).includes(normalized)) continue;
      results.push({
        type: 'resource',
        resource,
        title: resource.kind,
        detail: `${resource.apiVersion} · ${resource.category}${resource.namespaced ? ' · namespaced' : ' · cluster-wide'}`,
      });
    }
    const objectCandidates: Array<{ resource: ResourceDescriptor; object: ResourceObject }> = [];
    if (currentResource) currentObjects.forEach((object) => objectCandidates.push({ resource: currentResource!, object }));
    if (currentWorkloadResource) currentWorkloadObjects.forEach((object) => objectCandidates.push({ resource: currentWorkloadResource!, object }));
    const seenObjects = new Set<string>();
    for (const { resource, object } of objectCandidates) {
      const key = `${resourceKey(resource)}\u0000${object.namespace || ''}\u0000${object.name}`;
      if (seenObjects.has(key)) continue;
      seenObjects.add(key);
      const objectText = `${object.name} ${object.namespace || ''} ${resource.kind} ${resource.apiVersion}`.toLowerCase();
      if (!objectText.includes(normalized)) continue;
      results.push({
        type: 'object',
        resource,
        object,
        title: object.name,
        detail: `${resource.kind} · ${object.namespace || 'cluster-scoped'} · ${resource.apiVersion}`,
      });
    }
    return results
      .sort((left, right) => {
        const leftExact = left.title.toLowerCase() === normalized ? 0 : 1;
        const rightExact = right.title.toLowerCase() === normalized ? 0 : 1;
        return leftExact - rightExact || left.title.localeCompare(right.title);
      })
      .slice(0, 28);
  }

  function decodeSecret(value: string) {
    try {
      const bytes = Uint8Array.from(atob(value), (character) => character.charCodeAt(0));
      return new TextDecoder().decode(bytes);
    } catch {
      return value;
    }
  }

  function encodeSecret(value: string) {
    const bytes = new TextEncoder().encode(value);
    let binary = '';
    bytes.forEach((byte) => (binary += String.fromCharCode(byte)));
    return btoa(binary);
  }

  function updateEditorEntry(index: number, value: string) {
    editorEntries = editorEntries.map((entry, entryIndex) => entryIndex === index
      ? { ...entry, value: editorResource?.kind === 'Secret' && revealSecret ? encodeSecret(value) : value }
      : entry,
    );
  }

  function addEditorEntry() {
    const value = editorResource?.kind === 'Secret' && revealSecret ? encodeSecret('') : '';
    editorEntries = [...editorEntries, { key: 'new-key', value }];
    focusedEditorEntry = editorEntries.length - 1;
  }

  function removeEditorEntry(index: number) {
    editorEntries = editorEntries.filter((_, entryIndex) => entryIndex !== index);
    focusedEditorEntry = Math.max(0, Math.min(focusedEditorEntry, editorEntries.length - 1));
  }

  function asRecord(value: unknown): Record<string, unknown> {
    return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : {};
  }

  function asString(value: unknown): string | undefined {
    return typeof value === 'string' && value.trim() ? value : undefined;
  }

  function asArray(value: unknown): unknown[] {
    return Array.isArray(value) ? value : [];
  }

  function resourceLabels(manifest: Record<string, unknown> | null) {
    const labels = asRecord(asRecord(manifest).metadata).labels;
    return Object.entries(asRecord(labels)).slice(0, 16);
  }

  function networkAddressFacts(manifest: Record<string, unknown> | null): NetworkFact[] {
    const facts: NetworkFact[] = [];
    const seen = new Set<string>();
    const resource = asRecord(manifest);
    const spec = asRecord(resource.spec);
    const status = asRecord(resource.status);
    const add = (label: string, value: unknown, tone: NetworkFact['tone'] = 'neutral') => {
      const text = asString(value);
      if (!text || text.toLowerCase() === '<none>') return;
      const key = `${label}\u0000${text}`;
      if (!seen.has(key)) {
        seen.add(key);
        facts.push({ label, value: text, tone });
      }
    };
    asArray(spec.clusterIPs).forEach((value) => add('Cluster IP', value, 'primary'));
    if (!asArray(spec.clusterIPs).length) add('Cluster IP', spec.clusterIP, 'primary');
    asArray(spec.externalIPs).forEach((value) => add('External IP', value, 'external'));
    add('External hostname', spec.externalName, 'external');
    add('Load balancer IP', spec.loadBalancerIP, 'external');
    asArray(spec.rules).forEach((rule) => add('Host', asRecord(rule).host, 'primary'));
    asArray(spec.hostnames).forEach((hostname) => add('Host', hostname, 'primary'));
    asArray(spec.tls).forEach((entry) => asArray(asRecord(entry).hosts).forEach((value) => add('TLS host', value, 'primary')));
    asArray(spec.listeners).forEach((listener) => add('Listener host', asRecord(listener).hostname, 'primary'));
    asArray(spec.addresses).forEach((entry) => {
      if (typeof entry === 'string') add('Address', entry, 'primary');
      else {
        const address = asRecord(entry);
        add(asString(address.type) || 'Address', address.address ?? address.value, 'primary');
      }
    });
    asArray(spec.endpoints).forEach((entry) => {
      if (typeof entry === 'string') add('Endpoint', entry, 'primary');
      else {
        const endpoint = asRecord(entry);
        asArray(endpoint.addresses).forEach((address) => add('Endpoint', address, 'primary'));
        add('Endpoint', endpoint.address, 'primary');
      }
    });
    // Legacy Endpoints objects store backend addresses in root-level subsets,
    // rather than in spec or status. EndpointSlice uses root-level endpoints.
    // Handle both shapes so the inspector never hides live backend addresses.
    asArray(resource.subsets).forEach((entry) => {
      const subset = asRecord(entry);
      asArray(subset.addresses).forEach((address) => {
        const endpoint = asRecord(address);
        add('Endpoint IP', endpoint.ip, 'primary');
        add('Endpoint host', endpoint.hostname, 'primary');
      });
      asArray(subset.notReadyAddresses).forEach((address) => {
        const endpoint = asRecord(address);
        add('Not ready endpoint', endpoint.ip, 'neutral');
        add('Not ready host', endpoint.hostname, 'neutral');
      });
    });
    asArray(resource.endpoints).forEach((entry) => {
      const endpoint = asRecord(entry);
      asArray(endpoint.addresses).forEach((address) => add('Endpoint IP', address, 'primary'));
      add('Endpoint host', endpoint.hostname, 'primary');
    });
    asArray(asRecord(status.loadBalancer).ingress).forEach((entry) => {
      const ingress = asRecord(entry);
      add('Load balancer', ingress.hostname, 'external');
      add('Load balancer', ingress.ip, 'external');
    });
    asArray(status.addresses).forEach((entry) => {
      if (typeof entry === 'string') add('Address', entry, 'primary');
      else {
        const address = asRecord(entry);
        add(asString(address.type) || 'Address', address.address ?? address.value, 'primary');
      }
    });
    return facts;
  }

  function networkHosts(manifest: Record<string, unknown> | null) {
    return networkAddressFacts(manifest).map((fact) => `${fact.label} · ${fact.value}`);
  }

  function networkPortFacts(manifest: Record<string, unknown> | null): NetworkFact[] {
    const resource = asRecord(manifest);
    const spec = asRecord(resource.spec);
    const facts: NetworkFact[] = [];
    const seen = new Set<string>();
    const add = (label: string, value: string, tone: NetworkFact['tone'] = 'neutral') => {
      const key = `${label}\u0000${value}`;
      if (!seen.has(key)) {
        seen.add(key);
        facts.push({ label, value, tone });
      }
    };
    const addPort = (entry: unknown, fallbackLabel = 'Port') => {
      const port = asRecord(entry);
      const number = port.port ?? port.targetPort ?? port.containerPort;
      const target = port.targetPort && port.port !== port.targetPort ? ` → ${port.targetPort}` : '';
      const nodePort = port.nodePort ? ` · node ${port.nodePort}` : '';
      add(asString(port.name) || fallbackLabel, `${number ?? '—'}${target}${port.protocol ? `/${port.protocol}` : ''}${nodePort}`, port.nodePort ? 'external' : 'neutral');
    };
    asArray(spec.ports).forEach((entry) => addPort(entry));
    // Endpoints uses subsets[].ports; EndpointSlice publishes ports at the
    // object root. Both are backend listener facts, not service spec fields.
    asArray(resource.subsets).forEach((entry) => asArray(asRecord(entry).ports).forEach((port) => addPort(port, 'Endpoint port')));
    asArray(resource.ports).forEach((port) => addPort(port, 'Endpoint port'));
    asArray(spec.listeners).forEach((entry) => {
      const listener = asRecord(entry);
      const number = listener.port ?? listener.targetPort ?? '—';
      const host = asString(listener.hostname) ? ` · ${listener.hostname}` : '';
      add(asString(listener.name) || 'Listener', `${number}${listener.protocol ? `/${listener.protocol}` : ''}${host}`, 'primary');
    });
    asArray(spec.rules).forEach((rule) => {
      asArray(asRecord(rule).backendRefs).forEach((entry) => {
        const backend = asRecord(entry);
        const name = asString(backend.name) || 'Backend';
        const port = backend.port;
        if (port !== undefined && port !== null) add(`Backend · ${name}`, `${port}`, 'primary');
      });
    });
    return facts;
  }

  function networkPorts(manifest: Record<string, unknown> | null) {
    return networkPortFacts(manifest).map((fact) => `${fact.label} · ${fact.value}`);
  }

  function networkServiceType(manifest: Record<string, unknown> | null) {
    return asString(asRecord(asRecord(manifest).spec).type) || 'ClusterIP';
  }

  function networkServiceClusterIp(manifest: Record<string, unknown> | null) {
    const spec = asRecord(asRecord(manifest).spec);
    const clusterIps = asArray(spec.clusterIPs)
      .map(asString)
      .filter((value): value is string => Boolean(value));
    return clusterIps[0] || asString(spec.clusterIP) || 'Assigned by Kubernetes';
  }

  function networkServiceExternalEndpoints(manifest: Record<string, unknown> | null) {
    const endpoints = new Set<string>();
    const resource = asRecord(manifest);
    const spec = asRecord(resource.spec);
    const status = asRecord(resource.status);
    const add = (value: unknown) => {
      const endpoint = asString(value);
      if (endpoint && endpoint.toLowerCase() !== '<none>') endpoints.add(endpoint);
    };
    asArray(spec.externalIPs).forEach(add);
    add(spec.externalName);
    add(spec.loadBalancerIP);
    asArray(asRecord(status.loadBalancer).ingress).forEach((entry) => {
      add(asRecord(entry).ip);
      add(asRecord(entry).hostname);
    });
    return [...endpoints];
  }

  function networkServiceTrafficPolicy(manifest: Record<string, unknown> | null) {
    const spec = asRecord(asRecord(manifest).spec);
    return asString(spec.externalTrafficPolicy) || asString(spec.internalTrafficPolicy) || 'Cluster routing';
  }

  function networkServiceExposure(manifest: Record<string, unknown> | null) {
    const type = networkServiceType(manifest);
    const external = networkServiceExternalEndpoints(manifest);
    if (external.length) return 'Reachable outside the cluster';
    if (type === 'LoadBalancer') return 'Waiting for an external address';
    if (type === 'NodePort') return 'Exposed through node ports';
    if (type === 'ExternalName') return 'Routes to an external hostname';
    return 'Internal cluster service';
  }

  function genericResourcePreview(manifest: Record<string, unknown> | null) {
    const resource = asRecord(manifest);
    const preview = resource.spec || resource.status || {};
    return JSON.stringify(preview, null, 2);
  }

  function workloadPodSpec(manifest: Record<string, unknown> | null) {
    const resource = asRecord(manifest);
    const spec = asRecord(resource.spec);
    return resource.kind === 'Pod' ? spec : asRecord(asRecord(spec.template).spec);
  }

  function workloadImages(manifest: Record<string, unknown> | null) {
    const podSpec = workloadPodSpec(manifest);
    return [...asArray(podSpec.initContainers), ...asArray(podSpec.containers)]
      .map((container) => {
        const entry = asRecord(container);
        return { name: asString(entry.name) || 'container', image: asString(entry.image) || 'image unavailable', init: asArray(podSpec.initContainers).includes(container) };
      });
  }

  function workloadAttachments(manifest: Record<string, unknown> | null) {
    const configMaps = new Set<string>();
    const secrets = new Set<string>();
    const podSpec = workloadPodSpec(manifest);
    const addName = (target: Set<string>, value: unknown) => { const name = asString(value); if (name) target.add(name); };
    const scanContainer = (container: unknown) => {
      const entry = asRecord(container);
      asArray(entry.envFrom).forEach((source) => {
        const ref = asRecord(source);
        addName(configMaps, asRecord(ref.configMapRef).name);
        addName(secrets, asRecord(ref.secretRef).name);
      });
      asArray(entry.env).forEach((variable) => {
        const valueFrom = asRecord(asRecord(variable).valueFrom);
        addName(configMaps, asRecord(valueFrom.configMapKeyRef).name);
        addName(secrets, asRecord(valueFrom.secretKeyRef).name);
      });
    };
    [...asArray(podSpec.initContainers), ...asArray(podSpec.containers)].forEach(scanContainer);
    asArray(podSpec.imagePullSecrets).forEach((entry) => addName(secrets, asRecord(entry).name));
    asArray(podSpec.volumes).forEach((volume) => {
      const entry = asRecord(volume);
      addName(configMaps, asRecord(entry.configMap).name);
      addName(secrets, asRecord(entry.secret).secretName);
      asArray(asRecord(entry.projected).sources).forEach((source) => {
        addName(configMaps, asRecord(asRecord(source).configMap).name);
        addName(secrets, asRecord(asRecord(source).secret).name);
      });
    });
    return { configMaps: [...configMaps], secrets: [...secrets] };
  }

  function workloadReplicaSummary(manifest: Record<string, unknown> | null) {
    const spec = asRecord(asRecord(manifest).spec);
    const status = asRecord(asRecord(manifest).status);
    const desired = spec.replicas;
    const ready = status.readyReplicas ?? status.availableReplicas;
    return desired === undefined && ready === undefined ? 'Managed by Kubernetes' : `${ready ?? 0} ready · ${desired ?? '—'} desired`;
  }

  function resetWorkloadTerminal() {
    workloadDetailMode = 'overview';
    terminalPods = [];
    terminalTarget = null;
    terminalContainers = [];
    terminalPorts = [];
    selectedTerminalContainer = '';
    terminalOutput = '';
    loadingTerminalPods = false;
    loadingTerminalRuntime = false;
    runningTerminalCommand = false;
  }

  async function listWorkloadPods(resource: ResourceDescriptor, object: ResourceObject) {
    const resourceNamespace = objectNamespace(object);
    if (!resourceNamespace) throw new Error('A namespace is required to find workload Pods');
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<ResourceObject[]>('list_workload_pods', {
      request: {
        kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
        context: activeCluster,
        group: resource.group,
        version: resource.version,
        kind: resource.kind,
        plural: resource.plural,
        namespace: resourceNamespace,
        name: object.name,
      },
    });
  }

  async function openWorkloadLogs(resource: ResourceDescriptor, object: ResourceObject) {
    if (openingLogsTarget) return;
    const openingKey = logOpeningKey(resource.kind, object);
    const openingGeneration = logWorkspaceGeneration;
    openingLogsTarget = { key: openingKey, label: `${resource.kind} · ${object.name}` };
    await tick();
    try {
      const pods = resource.kind === 'Pod' ? [object] : await listWorkloadPods(resource, object);
      if (openingGeneration !== logWorkspaceGeneration || openingLogsTarget?.key !== openingKey) return;
      if (!pods.length) {
        notify(`No live Pods match ${resource.kind} ${object.name}`);
        return;
      }
      const podNamespace = objectNamespace(pods[0]);
      const podsInNamespace = podNamespace ? await namespacePodIndex(podNamespace, pods) : pods;
      if (openingGeneration !== logWorkspaceGeneration || openingLogsTarget?.key !== openingKey) return;
      await openPodLogsWorkspace(pods[0], podsInNamespace, `${resource.kind} · ${object.name}`);
    } catch (error) {
      if (openingLogsTarget?.key === openingKey) notify(`Could not open logs for ${object.name}: ${String(error)}`);
    } finally {
      if (openingLogsTarget?.key === openingKey) openingLogsTarget = null;
    }
  }

  async function selectTerminalPod(object: ResourceObject) {
    const podNamespace = objectNamespace(object);
    if (!podNamespace) {
      notify('A namespace is required to open a Pod terminal');
      return;
    }
    terminalTarget = { pod: object.name, namespace: podNamespace };
    terminalContainers = [];
    terminalPorts = [];
    selectedTerminalContainer = '';
    terminalOutput = '';
    loadingTerminalRuntime = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const runtime = await invoke<PodRuntime>('get_pod_runtime', {
        request: { kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null, context: activeCluster, namespace: podNamespace, pod: object.name },
      });
      terminalContainers = runtime.containers;
      terminalPorts = runtime.ports || [];
      selectedTerminalContainer = runtime.containers[0] || '';
    } catch (error) {
      notify(`Could not inspect ${object.name}: ${String(error)}`);
      terminalTarget = null;
    } finally {
      loadingTerminalRuntime = false;
    }
  }

  async function openWorkloadTerminal(resource: ResourceDescriptor, object: ResourceObject) {
    resetWorkloadTerminal();
    workloadDetailMode = 'terminal';
    loadingTerminalPods = true;
    try {
      terminalPods = resource.kind === 'Pod' ? [object] : await listWorkloadPods(resource, object);
      if (!terminalPods.length) {
        notify(`No live Pods match ${resource.kind} ${object.name}`);
        return;
      }
      await selectTerminalPod(terminalPods[0]);
    } catch (error) {
      notify(`Could not prepare a terminal for ${object.name}: ${String(error)}`);
    } finally {
      loadingTerminalPods = false;
    }
  }

  async function runTerminalCommand() {
    if (!terminalTarget || !terminalCommand.trim()) return;
    runningTerminalCommand = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const response = await invoke<PodExecResponse>('exec_pod_command', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context: activeCluster,
          namespace: terminalTarget.namespace,
          pod: terminalTarget.pod,
          container: selectedTerminalContainer || null,
          command: terminalCommand,
        },
      });
      const output = `${response.stdout || ''}${response.stderr ? `${response.stdout ? '\n' : ''}${response.stderr}` : ''}`.trimEnd();
      terminalOutput = `$ ${terminalCommand}\n${output || '(command completed without output)'}`;
    } catch (error) {
      terminalOutput = `$ ${terminalCommand}\n${String(error)}`;
    } finally {
      runningTerminalCommand = false;
    }
  }

  async function openResourceEditor(resource: ResourceDescriptor, object: ResourceObject) {
    const resourceNamespace = objectNamespace(object);
    if (resource.namespaced && !resourceNamespace) {
      notify('A namespace is required to open this resource');
      return;
    }
    resetWorkloadTerminal();
    editorResource = resource;
    editorObject = object;
    editorManifest = null;
    editorEntries = [];
    focusedEditorEntry = 0;
    editorCertificate = undefined;
    revealSecret = false;
    loadingEditor = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const detail = await invoke<ResourceDetail>('get_resource_detail', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context: activeCluster,
          group: resource.group,
          version: resource.version,
          kind: resource.kind,
          plural: resource.plural,
          namespaced: resource.namespaced,
          namespace: resourceNamespace || null,
          name: object.name,
        },
      });
      editorManifest = detail.manifest;
      editorCertificate = detail.certificate;
      const data = detail.manifest.data;
      if (data && typeof data === 'object' && !Array.isArray(data)) {
        editorEntries = Object.entries(data).map(([key, value]) => ({ key, value: String(value) }));
      }
    } catch (error) {
      notify(`Could not open ${resource.kind}: ${String(error)}`);
      editorResource = null;
      editorObject = null;
    } finally {
      loadingEditor = false;
    }
  }

  async function saveEditor() {
    if (!editorResource || !editorObject || !editorManifest) return;
    savingEditor = true;
    try {
      const resourceNamespace = objectNamespace(editorObject);
      const manifest = { ...editorManifest, data: Object.fromEntries(editorEntries.map((entry) => [entry.key, entry.value])) };
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_resource_detail', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context: activeCluster,
          group: editorResource.group,
          version: editorResource.version,
          kind: editorResource.kind,
          plural: editorResource.plural,
          namespaced: editorResource.namespaced,
          namespace: resourceNamespace || null,
          name: editorObject.name,
          manifest,
        },
      });
      notify(`${editorResource.kind} saved to ${activeCluster}`);
      editorManifest = manifest;
    } catch (error) {
      notify(`Could not save ${editorResource.kind}: ${String(error)}`);
    } finally {
      savingEditor = false;
    }
  }

  async function deleteResourceObject(resource: ResourceDescriptor, object: ResourceObject) {
    const resourceNamespace = objectNamespace(object);
    if (resource.namespaced && !resourceNamespace) {
      notify('A namespace is required to delete this resource');
      return;
    }
    deletingResource = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_resource_object', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context: activeCluster,
          group: resource.group,
          version: resource.version,
          kind: resource.kind,
          plural: resource.plural,
          namespaced: resource.namespaced,
          namespace: resourceNamespace || null,
          name: object.name,
        },
      });
      clearClusterObjectCache(activeClusterId);
      resourceObjects = resourceObjects.filter((candidate) => candidate.name !== object.name || candidate.namespace !== object.namespace);
      if (workloadResource && resourceKey(workloadResource) === resourceKey(resource)) {
        workloadObjects = workloadObjects.filter((candidate) => candidate.name !== object.name || candidate.namespace !== object.namespace);
      }
      closeEditor();
      closeYamlEditor();
      notify(`${resource.kind} ${object.name} was deleted from ${activeCluster}`);
    } catch (error) {
      notify(`Could not delete ${resource.kind} ${object.name}: ${String(error)}`);
    } finally {
      deletingResource = false;
      deletionTarget = null;
      deletionStep = 1;
      deletionName = '';
    }
  }

  function closeEditor(cancelOpeningLogs = true) {
    if (savingEditor) return;
    if (cancelOpeningLogs && openingLogsTarget) {
      openingLogsTarget = null;
      logWorkspaceGeneration += 1;
    }
    if (workloadDetailMode === 'logs') {
      closeLogs(false);
      workloadDetailMode = 'overview';
    }
    resetWorkloadTerminal();
    editorResource = null;
    editorObject = null;
    editorManifest = null;
    editorEntries = [];
    focusedEditorEntry = 0;
  }

  async function openYamlEditor(resource: ResourceDescriptor, object: ResourceObject) {
    const resourceNamespace = objectNamespace(object);
    if (resource.namespaced && !resourceNamespace) {
      notify('A namespace is required to view this resource');
      return;
    }
    yamlResource = resource;
    yamlObject = object;
    yamlText = '';
    yamlOriginal = '';
    yamlMode = 'view';
    loadingYaml = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const detail = await invoke<ResourceDetail>('get_resource_detail', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context: activeCluster,
          group: resource.group,
          version: resource.version,
          kind: resource.kind,
          plural: resource.plural,
          namespaced: resource.namespaced,
          namespace: resourceNamespace || null,
          name: object.name,
        },
      });
      yamlText = detail.yaml;
      yamlOriginal = detail.yaml;
    } catch (error) {
      notify(`Could not load YAML for ${object.name}: ${String(error)}`);
      yamlResource = null;
      yamlObject = null;
    } finally {
      loadingYaml = false;
    }
  }

  async function saveYamlEditor() {
    if (!yamlResource || !yamlObject || !yamlText.trim()) return;
    savingYaml = true;
    try {
      const resourceNamespace = objectNamespace(yamlObject);
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_resource_yaml', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context: activeCluster,
          group: yamlResource.group,
          version: yamlResource.version,
          kind: yamlResource.kind,
          plural: yamlResource.plural,
          namespaced: yamlResource.namespaced,
          namespace: resourceNamespace || null,
          name: yamlObject.name,
          yaml: yamlText,
        },
      });
      yamlOriginal = yamlText;
      yamlMode = 'view';
      notify(`${yamlResource.kind} saved to ${activeCluster}`);
    } catch (error) {
      notify(`Could not save YAML: ${String(error)}`);
    } finally {
      savingYaml = false;
    }
  }

  function closeYamlEditor() {
    if (savingYaml) return;
    yamlResource = null;
    yamlObject = null;
    yamlText = '';
    yamlOriginal = '';
    yamlMode = 'view';
  }

  function closeLogs(clearOpening = true) {
    if (logRefreshTimer) window.clearInterval(logRefreshTimer);
    logRefreshTimer = undefined;
    logWorkspaceGeneration += 1;
    logRequestGeneration += 1;
    if (clearOpening) openingLogsTarget = null;
    loadingLogs = false;
    logTarget = null;
    logPods = [];
    logPorts = [];
    logScopeLabel = '';
    logLines = [];
    logContainers = [];
    selectedLogContainer = undefined;
    portForwardOpen = false;
  }

  function closeWorkloadLogs() {
    closeLogs();
    workloadDetailMode = 'overview';
  }

  async function loadLogs(reset = false) {
    if (!logTarget || loadingLogs) return;
    const requestGeneration = ++logRequestGeneration;
    const target = { ...logTarget };
    const clusterId = activeClusterId;
    const context = activeCluster;
    loadingLogs = true;
    const wasAtBottom = !logViewport || logViewport.scrollHeight - logViewport.scrollTop - logViewport.clientHeight < 24;
    const previousHeight = logViewport?.scrollHeight || 0;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const response = await invoke<PodLogResponse>('read_pod_logs', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context,
          namespace: target.namespace,
          pod: target.pod,
          container: selectedLogContainer || null,
          tailLines: 750,
        },
      });
      const isStillActive = requestGeneration === logRequestGeneration
        && clusterId === activeClusterId
        && context === activeCluster
        && logTarget?.pod === target.pod
        && logTarget?.namespace === target.namespace;
      if (!isStillActive) return;
      logContainers = response.containers;
      logPorts = response.ports || [];
      selectedLogContainer = response.selectedContainer;
      if (reset) {
        logLines = response.lines;
      } else {
        const existing = new Set(logLines);
        logLines = [...logLines, ...response.lines.filter((line) => !existing.has(line))].slice(-5_000);
      }
      await tick();
      if (logViewport) {
        if (wasAtBottom) logViewport.scrollTop = logViewport.scrollHeight;
        else logViewport.scrollTop += logViewport.scrollHeight - previousHeight;
      }
    } catch (error) {
      if (requestGeneration === logRequestGeneration && clusterId === activeClusterId) {
        notify(`Could not read logs for ${target.pod}: ${String(error)}`);
      }
    } finally {
      if (requestGeneration === logRequestGeneration) loadingLogs = false;
    }
  }

  function logPodKey(object: ResourceObject) {
    return `${object.namespace || (namespace === 'all namespaces' ? '' : namespace)}\u0000${object.name}`;
  }

  function logTargetKey(target: LogTarget) {
    return `${target.namespace}\u0000${target.pod}`;
  }

  function availableLogPods(primary: ResourceObject, candidates: ResourceObject[]) {
    const pods = new Map<string, ResourceObject>();
    [...candidates, primary].forEach((pod) => pods.set(logPodKey(pod), pod));
    return [...pods.values()].sort((left, right) => logPodKey(left).localeCompare(logPodKey(right)));
  }

  async function namespacePodIndex(resourceNamespace: string, fallback: ResourceObject[]) {
    const podResource = catalog.resources.find((resource) => resource.kind === 'Pod' && resource.namespaced);
    if (!podResource) return fallback;
    const cacheKey = resourceObjectCacheKey(activeClusterId, podResource, resourceNamespace);
    const cachedPods = resourceObjectCache.get(cacheKey);
    if (cachedPods) return cachedPods;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const pods = await invoke<ResourceObject[]>('list_resource_objects', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context: activeCluster,
          group: podResource.group,
          version: podResource.version,
          kind: podResource.kind,
          plural: podResource.plural,
          namespaced: true,
          namespace: resourceNamespace,
        },
      });
      resourceObjectCache.set(cacheKey, pods);
      return pods.length ? pods : fallback;
    } catch {
      // The selected workload still has its authoritative Pod match list available.
      return fallback;
    }
  }

  async function selectLogPod(object: ResourceObject) {
    const objectNamespace = object.namespace || (namespace === 'all namespaces' ? '' : namespace);
    if (!objectNamespace) {
      notify('A namespace is required to read pod logs');
      return;
    }
    logRequestGeneration += 1;
    loadingLogs = false;
    logTarget = { pod: object.name, namespace: objectNamespace };
    logLines = [];
    selectedLogContainer = undefined;
    logContainers = [];
    logPorts = [];
    portForwardOpen = false;
    portForwardRemotePort = '';
    portForwardLocalPort = '';
    await loadLogs(true);
  }

  function selectLogPodByKey(key: string) {
    const pod = logPods.find((candidate) => logPodKey(candidate) === key);
    if (pod) void selectLogPod(pod);
  }

  function openPortForwardForm() {
    if (!logTarget) return;
    const suggested = suggestedForwardPorts[0];
    if (!portForwardRemotePort && suggested) portForwardRemotePort = String(suggested);
    if (!portForwardLocalPort && suggested) portForwardLocalPort = String(suggested);
    portForwardOpen = !portForwardOpen;
  }

  async function syncPortForwards(showError = false) {
    if (!('__TAURI_INTERNALS__' in window) || syncingPortForwards) return;
    syncingPortForwards = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      portForwards = await invoke<PortForward[]>('list_port_forwards');
    } catch (error) {
      if (showError) notify(`Could not check port forwards: ${String(error)}`);
    } finally {
      syncingPortForwards = false;
    }
  }

  function validPort(value: string) {
    const port = Number(value);
    return Number.isInteger(port) && port > 0 && port <= 65_535 ? port : undefined;
  }

  async function startPortForward() {
    if (!logTarget) return;
    const remotePort = validPort(portForwardRemotePort);
    const localPort = validPort(portForwardLocalPort);
    if (!remotePort || !localPort) {
      notify('Choose valid remote and local ports between 1 and 65535');
      return;
    }
    portForwarding = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const forward = await invoke<PortForward>('start_port_forward', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context: activeCluster,
          namespace: logTarget.namespace,
          pod: logTarget.pod,
          remotePort,
          localPort,
        },
      });
      portForwards = [...portForwards.filter((candidate) => candidate.id !== forward.id), forward];
      await syncPortForwards();
      portForwardOpen = false;
      notify(`Forwarding ${logTarget.pod}:${remotePort} on ${forward.localAddress}`);
    } catch (error) {
      notify(`Could not start port forward: ${String(error)}`);
    } finally {
      portForwarding = false;
    }
  }

  async function stopPortForward(id: string) {
    if (stoppingPortForwardId) return;
    stoppingPortForwardId = id;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke<PortForward>('stop_port_forward', { request: { id } });
      portForwards = portForwards.filter((forward) => forward.id !== id);
      notify('Port forward stopped');
    } catch (error) {
      await syncPortForwards();
      notify(`Could not stop port forward: ${String(error)}`);
    } finally {
      stoppingPortForwardId = '';
    }
  }

  async function openPodLogsWorkspace(object: ResourceObject, candidates: ResourceObject[] = [], scopeLabel = 'Pod') {
    closeLogs(false);
    const workspaceGeneration = logWorkspaceGeneration;
    stopOverviewRefresh();
    // Logs live in the Workloads inspector so the workload-type rail and the
    // selected object list remain available while an operator follows a stream.
    closeYamlEditor();
    logPods = availableLogPods(object, candidates);
    logScopeLabel = scopeLabel;
    selectedResource = null;
    relatedPods = null;
    relatedObject = null;
    clusterPickerOpen = false;
    namespaceOpen = false;
    if (activeView !== 'Workloads') {
      const podResource = catalog.resources.find((resource) => resource.category === 'Workloads' && resource.kind === 'Pod');
      if (podResource) {
        workloadResource = podResource;
        workloadObjects = logPods;
        workloadSearch = '';
      }
      closeEditor(false);
      activeView = 'Workloads';
    }
    workloadDetailMode = 'logs';
    await selectLogPod(object);
    if (workspaceGeneration !== logWorkspaceGeneration || activeView !== 'Workloads' || !logTarget) return;
    logRefreshTimer = window.setInterval(() => loadLogs(), 30_000);
  }

  async function openPodLogs(object: ResourceObject, candidates: ResourceObject[] = [], scopeLabel = 'Pod') {
    if (openingLogsTarget) return;
    const openingKey = logOpeningKey('Pod', object);
    openingLogsTarget = { key: openingKey, label: `Pod · ${object.name}` };
    await tick();
    try {
      if (openingLogsTarget?.key !== openingKey) return;
      await openPodLogsWorkspace(object, candidates, scopeLabel);
    } finally {
      if (openingLogsTarget?.key === openingKey) openingLogsTarget = null;
    }
  }

  async function openObject(resource: ResourceDescriptor, object: ResourceObject) {
    if (resource.kind === 'Secret' || resource.kind === 'ConfigMap' || resource.kind === 'Certificate') {
      await openResourceEditor(resource, object);
      return;
    }
    // In the Workloads workspace every object first opens its live detail pane.
    // Logs and terminal access are deliberate actions from there; a Deployment
    // is never itself a log source.
    if (activeView === 'Workloads' && resource.category === 'Workloads') {
      closeWorkloadLogs();
      await openResourceEditor(resource, object);
      return;
    }
    if (resource.kind === 'Pod') {
      const candidatePods = workloadObjects.some((candidate) => logPodKey(candidate) === logPodKey(object))
        ? workloadObjects
        : resourceObjects;
      await openPodLogs(object, candidatePods, 'Pod');
      return;
    }
    const workloadKindsWithPodSelectors = new Set(['Deployment', 'StatefulSet', 'DaemonSet', 'ReplicaSet', 'ReplicationController', 'Job']);
    if (resource.category !== 'Workloads' || !workloadKindsWithPodSelectors.has(resource.kind)) {
      if (activeView === 'Resources') {
        await openResourceEditor(resource, object);
        return;
      }
      await openYamlEditor(resource, object);
      return;
    }
    await openResourceEditor(resource, object);
  }

  onDestroy(() => {
    rememberActiveClusterSession();
    persistWorkspace();
    closeLogs();
    stopOverviewRefresh();
    stopLiveObjectRefresh();
    resourceWatchUnlisten?.();
    resourceWatchUnlisten = undefined;
    stopWindowResizeListening?.();
    if (windowResizeStateTimer) window.clearTimeout(windowResizeStateTimer);
  });

  onMount(() => {
    applyTheme(loadThemePreference());
    void restoreWorkspace();
    void setupWindowControls();
    resourceWatchListenerReady = setupResourceWatchListener();
    void syncPortForwards();
    const closeFloatingMenus = (event: PointerEvent) => {
      const target = event.target instanceof Element ? event.target : null;
      if (!target?.closest('.cluster-selector')) clusterPickerOpen = false;
      if (!target?.closest('.namespace-picker')) namespaceOpen = false;
      if (!target?.closest('.sidebar-workload-menu, .sidebar-resource-menu')) closeSidebarTypeMenus();
      if (!target?.closest('.favorite-context-menu, .favorite-shortcut, .favorite-card-open')) favoriteContextMenu = null;
    };
    const closeFloatingMenusOnEscape = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        openCommandSearch();
        return;
      }
      if (event.key !== 'Escape') return;
      if (commandOpen) {
        commandOpen = false;
        commandQuery = '';
      }
      closeSidebarTypeMenus();
    };
    window.addEventListener('pointerdown', closeFloatingMenus);
    window.addEventListener('keydown', closeFloatingMenusOnEscape);
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        lastHiddenAt = Date.now();
        return;
      }
      if (lastHiddenAt && Date.now() - lastHiddenAt > 5000) queueLiveResumeRecovery();
      lastHiddenAt = 0;
    };
    const handleWindowFocus = () => {
      if (lastHiddenAt && Date.now() - lastHiddenAt > 5000) queueLiveResumeRecovery();
    };
    const handleNetworkOnline = () => queueLiveResumeRecovery();
    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('focus', handleWindowFocus);
    window.addEventListener('online', handleNetworkOnline);
    return () => {
      window.removeEventListener('pointerdown', closeFloatingMenus);
      window.removeEventListener('keydown', closeFloatingMenusOnEscape);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('focus', handleWindowFocus);
      window.removeEventListener('online', handleNetworkOnline);
      if (resumeRecoveryTimer) window.clearTimeout(resumeRecoveryTimer);
      resumeRecoveryTimer = undefined;
    };
  });

  async function loadCluster(cluster: Cluster, force = false) {
    // A Pod name belongs to exactly one cluster context. Never carry its stream across a switch.
    resourceRequestGeneration += 1;
    workloadRequestGeneration += 1;
    stopLiveObjectRefresh();
    loadingObjects = false;
    loadingWorkloads = false;
    closeLogs();
    closeEditor();
    closeYamlEditor();
    clusterPickerOpen = false;
    namespaceOpen = false;
    closeSidebarTypeMenus();
    activeClusterId = cluster.id;
    activeCluster = cluster.name;
    activeKubeconfigPath = cluster.kubeconfigPath;
    if (eventsClusterId !== cluster.id) {
      clusterEvents = [];
      eventsObservedAt = '';
      eventsError = '';
      eventsClusterId = '';
    }
    restoreClusterSession(cluster);
    catalogError = '';
    overviewError = '';
    selectedResource = null;
    const cachedCatalog = catalogCache.get(cluster.id);
    if (cachedCatalog && !force) {
      catalog = cachedCatalog;
      updateCluster(cluster.id, { status: 'Connected', tone: 'green' });
      notify(`Switched to ${cluster.name} · ${catalog.resources.length} resources`);
      if (activeView === 'Overview') {
        void loadClusterOverview();
        startOverviewRefresh();
      } else if (activeView === 'Events') {
        void loadClusterEvents(true);
      }
      return;
    }

    catalog = { context: cluster.name, namespaces: [], resources: [] };
    loadingCatalog = true;
    updateCluster(cluster.id, { status: 'Connecting', tone: 'blue' });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const sourcePath = cluster.kubeconfigPath || kubeconfigPath || null;
      catalog = await invoke<ClusterCatalog>('discover_cluster_catalog', { kubeconfigPath: sourcePath, context: cluster.name });
      catalogCache.set(cluster.id, catalog);
      updateCluster(cluster.id, { status: 'Connected', tone: 'green' });
      notify(`Connected to ${cluster.name} · ${catalog.resources.length} resources discovered`);
      if (activeView === 'Overview') {
        void loadClusterOverview();
        startOverviewRefresh();
      } else if (activeView === 'Events') {
        void loadClusterEvents(true);
      }
    } catch (error) {
      catalogError = String(error);
      updateCluster(cluster.id, { status: 'Connection failed', tone: 'red' });
      notify(`Could not connect to ${cluster.name}: ${catalogError}`);
    } finally {
      loadingCatalog = false;
    }
  }

  async function selectCluster(id: string) {
    const cluster = clusters.find((candidate) => candidate.id === id);
    if (!cluster) return;
    rememberActiveClusterSession();
    clusterPickerOpen = false;
    stopOverviewRefresh();
    closeEditor();
    closeYamlEditor();
    selectedResource = null;
    activeView = 'Overview';
    await loadCluster(cluster);
  }

  function deletionTargetName(target = deletionTarget) {
    if (!target) return '';
    return target.type === 'resource' ? target.object.name : target.cluster.name;
  }

  function requestResourceDeletion(resource: ResourceDescriptor, object: ResourceObject) {
    if (savingEditor || loadingEditor) return;
    deletionTarget = { type: 'resource', resource, object };
    deletionStep = 1;
    deletionName = '';
  }

  function requestClusterRemoval(cluster: Cluster) {
    deletionTarget = { type: 'cluster', cluster };
    deletionStep = 1;
    deletionName = '';
  }

  function removeCluster(id: string) {
    const cluster = clusters.find((candidate) => candidate.id === id);
    if (cluster) requestClusterRemoval(cluster);
  }

  function cancelDeletion() {
    if (deletingResource) return;
    deletionTarget = null;
    deletionStep = 1;
    deletionName = '';
  }

  function continueDeletion() {
    deletionStep = 2;
    deletionName = '';
  }

  async function confirmDeletion() {
    const target = deletionTarget;
    if (!target || deletionName !== deletionTargetName(target)) return;
    if (target.type === 'resource') await deleteResourceObject(target.resource, target.object);
    else await removeClusterContext(target.cluster);
  }

  async function removeClusterContext(cluster: Cluster) {
    deletingResource = true;
    try {
      const wasActive = activeClusterId === cluster.id;
      clusters = clusters.filter((candidate) => candidate.id !== cluster.id);
      catalogCache.delete(cluster.id);
      clusterSessionCache.delete(cluster.id);
      const { [cluster.id]: _removedNamespace, ...remainingNamespaces } = persistedClusterNamespaces;
      persistedClusterNamespaces = remainingNamespaces;
      favoriteClusterIds = favoriteClusterIds.filter((id) => id !== cluster.id);
      const { [cluster.id]: _removedFavoriteName, ...remainingFavoriteNames } = favoriteClusterNames;
      favoriteClusterNames = remainingFavoriteNames;
      clearClusterObjectCache(cluster.id);
      if (wasActive) {
        stopOverviewRefresh();
        activeClusterId = '';
        activeCluster = clusters.length ? 'Select a cluster' : 'No cluster connected';
        activeKubeconfigPath = undefined;
        catalog = { context: '', namespaces: [], resources: [] };
        clusterOverview = null;
        clusterEvents = [];
        eventsObservedAt = '';
        eventsError = '';
        eventsClusterId = '';
        selectedResource = null;
        closeEditor();
        closeYamlEditor();
        activeView = clusters.length ? 'Clusters' : 'Overview';
      }
      if (!clusters.length) {
        connectedKubeconfig = false;
        sourceConfigured = kubeconfigSources.length > 0;
        if (!kubeconfigSources.length) kubeconfigPath = '';
      }
      persistWorkspace();
      notify(`Removed ${cluster.name} from Kuberniva. The source kubeconfig was not changed.`);
    } catch (error) {
      notify(`Could not remove ${cluster.name}: ${String(error)}`);
    } finally {
      deletingResource = false;
      deletionTarget = null;
      deletionStep = 1;
      deletionName = '';
    }
  }

  async function refreshActiveCluster() {
    await refreshCurrentView();
  }

  async function connectKubeconfig() {
    loadingCatalog = true;
    try {
      if (!('__TAURI_INTERNALS__' in window)) {
        throw new Error('Kubeconfig connections are available in the Kuberniva desktop app');
      } else {
        const { invoke } = await import('@tauri-apps/api/core');
        const summary = await invoke<KubeconfigSummary>('read_kubeconfig_contexts', { kubeconfigPath: kubeconfigPath || null });
        if (!summary.contexts.length) throw new Error('No contexts found in this kubeconfig');
        const clusterCountBeforeAdd = clusters.length;
        rememberKubeconfigSource(kubeconfigPath);
        applyKubeconfigSummary(summary, kubeconfigPath);
        sourceConfigured = true;
        persistWorkspace();
        await navigateTo('Clusters');
        closeKubeconfigModal(true);
        const addedCount = clusters.length - clusterCountBeforeAdd;
        notify(`${addedCount > 0 ? `${addedCount} new context${addedCount === 1 ? '' : 's'} added` : 'Source refreshed'} · ${clusters.length} context${clusters.length === 1 ? '' : 's'} tracked locally.`);
      }
    } catch (error) {
      notify(`Could not connect: ${String(error)}`);
    } finally {
      loadingCatalog = false;
    }
  }

  async function importPastedKubeconfig() {
    const content = pastedKubeconfig.trim();
    if (!content) {
      notify('Paste kubeconfig YAML before importing');
      return;
    }
    loadingCatalog = true;
    try {
      if (!('__TAURI_INTERNALS__' in window)) {
        throw new Error('Pasted kubeconfig import is available in the Kuberniva desktop app');
      }
      const { invoke } = await import('@tauri-apps/api/core');
      const summary = await invoke<KubeconfigSummary>('import_pasted_kubeconfig', { content });
      if (!summary.contexts.length) throw new Error('No contexts found in the pasted kubeconfig');
      const savedSource = summary.contexts[0]?.sourcePath;
      if (!savedSource) throw new Error('The pasted kubeconfig could not be saved locally');

      const clusterCountBeforeAdd = clusters.length;
      kubeconfigPath = savedSource;
      rememberKubeconfigSource(savedSource);
      applyKubeconfigSummary(summary, savedSource);
      sourceConfigured = true;
      persistWorkspace();
      await navigateTo('Clusters');
      closeKubeconfigModal(true);
      const addedCount = clusters.length - clusterCountBeforeAdd;
      notify(`${addedCount > 0 ? `${addedCount} new context${addedCount === 1 ? '' : 's'} imported` : 'Pasted source refreshed'} · ${clusters.length} context${clusters.length === 1 ? '' : 's'} tracked locally.`);
    } catch (error) {
      notify(`Could not import kubeconfig: ${String(error)}`);
    } finally {
      loadingCatalog = false;
    }
  }

  async function syncKubeconfigSources() {
    if (!kubeconfigSources.length) {
      notify('Add a kubeconfig file or folder before syncing');
      return;
    }
    loadingCatalog = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const results = await Promise.allSettled(kubeconfigSources.map((source) =>
        invoke<KubeconfigSummary>('read_kubeconfig_contexts', { kubeconfigPath: source || null }),
      ));
      let syncedSourceCount = 0;
      results.forEach((result, index) => {
        if (result.status !== 'fulfilled') return;
        applyKubeconfigSummary(result.value, kubeconfigSources[index], true);
        syncedSourceCount += 1;
      });
      if (!syncedSourceCount) {
        const firstFailure = results.find((result) => result.status === 'rejected');
        throw new Error(firstFailure && firstFailure.status === 'rejected' ? String(firstFailure.reason) : 'No source could be read');
      }
      sourceConfigured = true;
      persistWorkspace();
      notify(`Synced ${syncedSourceCount} source${syncedSourceCount === 1 ? '' : 's'} · ${clusters.length} context${clusters.length === 1 ? '' : 's'} tracked locally.`);
    } catch (error) {
      notify(`Could not sync kubeconfig sources: ${String(error)}`);
    } finally {
      loadingCatalog = false;
    }
  }

  async function openResource(resource: ResourceDescriptor, options: { silent?: boolean } = {}) {
    const silent = options.silent === true;
    const requestGeneration = ++resourceRequestGeneration;
    const requestClusterId = activeClusterId;
    const requestNamespace = namespace;
    const requestResourceKey = resourceKey(resource);
    if (!silent) {
      closeEditor();
      closeYamlEditor();
    }
    selectedResource = resource;
    if (!silent) {
      relatedPods = null;
      relatedObject = null;
    }
    const cacheKey = resourceObjectCacheKey(requestClusterId, resource, requestNamespace);
    const cachedObjects = resourceObjectCache.get(cacheKey);
    if (cachedObjects) {
      resourceObjects = cachedObjects;
      loadingObjects = false;
      if (!silent && activeView === 'Resources') startLiveObjectRefresh();
      return;
    }
    if (!silent) {
      loadingObjects = true;
      resourceObjects = [];
    }
    try {
      if (!('__TAURI_INTERNALS__' in window)) {
        throw new Error('Resource listing is available in the Kuberniva desktop app');
      } else {
        const { invoke } = await import('@tauri-apps/api/core');
        const response = await invoke<ResourceObject[]>('list_resource_objects', {
          request: {
            kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
            context: activeCluster,
            group: resource.group,
            version: resource.version,
            kind: resource.kind,
            plural: resource.plural,
            namespaced: resource.namespaced,
            namespace: requestNamespace,
          },
        });
        if (requestGeneration !== resourceRequestGeneration
          || requestClusterId !== activeClusterId
          || requestNamespace !== namespace
          || !selectedResource
          || resourceKey(selectedResource) !== requestResourceKey) return;
        resourceObjects = response;
        resourceObjectCache.set(cacheKey, response);
        if (!silent && activeView === 'Resources') startLiveObjectRefresh();
        if (
          editorResource
          && editorObject
          && resourceKey(editorResource) === requestResourceKey
          && !response.some((object) => object.name === editorObject?.name && object.namespace === editorObject?.namespace)
        ) {
          const removedName = editorObject.name;
          closeEditor(false);
          closeYamlEditor();
          notify(resource.kind + ' ' + removedName + ' is no longer present in this namespace');
        }
      }
    } catch (error) {
      if (requestGeneration === resourceRequestGeneration && requestClusterId === activeClusterId) {
        notify(`Could not list ${resource.kind}: ${String(error)}`);
      }
    } finally {
      if (requestGeneration === resourceRequestGeneration && requestClusterId === activeClusterId && !silent) loadingObjects = false;
      flushPendingResourceWatchRefresh();
    }
  }
</script>

<svelte:head>
  <title>Kuberniva — Kubernetes, in focus</title>
  <meta name="description" content="A calm, fast Kubernetes control surface." />
</svelte:head>

<main class:sidebar-collapsed={sidebarHidden} class:theme-dark={theme === 'dark'}>
  <aside class:sidebar-hidden={sidebarHidden} class:sidebar-flyout-open={sidebarWorkloadMenuOpen || sidebarResourceMenuOpen} class="sidebar" style:width={`${sidebarWidth}px`} style:flex-basis={`${sidebarWidth}px`}>
    <div class="brand">
      <img class="brand-mark" src="/kuberniva-mark.png" alt="" />
      <span class="brand-wordmark"><strong>Kube</strong><span>rniva</span></span>
    </div>

    <nav aria-label="Cluster navigation">
      <p class="eyebrow">Cluster workspace</p>
      {#each ['Overview', 'Events', 'Workloads', 'Resources', 'CLI', 'Port forwards'] as view}
        {#if view === 'Workloads'}
          <div class:sidebar-type-open={sidebarWorkloadMenuOpen} class="sidebar-type-nav sidebar-workload-menu">
            <div class="sidebar-type-nav-row"><button class:active={activeView === 'Workloads'} class="nav-item" on:click={() => navigateTo('Workloads')}><span class="nav-icon"><Workflow size={17} strokeWidth={1.8} /></span>Workloads{#if workloadObjects.length}<span class="count">{workloadObjects.length}</span>{/if}</button><button id="sidebar-workload-type-trigger" class:active={activeView === 'Workloads'} class="sidebar-type-toggle" type="button" aria-label="Choose workload type" aria-haspopup="dialog" aria-expanded={sidebarWorkloadMenuOpen} on:click={() => toggleSidebarTypeMenu('workload')} on:keydown={(event) => handleSidebarTypeTriggerKeydown(event, 'workload')}><ChevronDown size={15} /></button></div>
            {#if sidebarWorkloadMenuOpen}
              <div class="sidebar-type-flyout sidebar-workload-flyout" role="dialog" aria-label="Choose workload type" tabindex="-1" on:keydown={(event) => handleSidebarTypeMenuKeydown(event, 'workload')}><div class="sidebar-type-heading"><div><span><Boxes size={17} /></span><div><strong>Workload types</strong><small>{namespace} · loaded on demand</small></div></div><kbd>esc</kbd></div><div id="sidebar-workload-type-options" class="sidebar-type-options" role="listbox" aria-label="Workload types">{#if workloadResources.length}{#each workloadResources as resource}<button type="button" role="option" aria-selected={workloadResource !== null && resourceKey(workloadResource) === resourceKey(resource)} class:sidebar-type-selected={workloadResource !== null && resourceKey(workloadResource) === resourceKey(resource)} on:click={() => selectSidebarWorkloadType(resource)}><span class="sidebar-type-icon"><Boxes size={15} /></span><span><strong>{resource.kind}</strong><small>{resource.apiVersion} · {resource.namespaced ? 'namespaced' : 'cluster-wide'}</small></span><b>{workloadResource !== null && resourceKey(workloadResource) === resourceKey(resource) ? '✓' : ''}</b></button>{/each}{:else}<div class="sidebar-type-empty"><strong>No workload APIs discovered</strong><small>Select or refresh a cluster first.</small></div>{/if}</div></div>
            {/if}
          </div>
        {:else if view === 'Resources'}
          <div class:sidebar-type-open={sidebarResourceMenuOpen} class="sidebar-type-nav sidebar-resource-menu">
            <div class="sidebar-type-nav-row"><button class:active={activeView === 'Resources'} class="nav-item" on:click={() => navigateTo('Resources')}><span class="nav-icon"><Database size={17} strokeWidth={1.8} /></span>Resources<span class="count">{activeClusterId ? resourceWorkspaceResources.length : 0}</span></button><button id="sidebar-resource-type-trigger" class:active={activeView === 'Resources'} class="sidebar-type-toggle" type="button" aria-label="Choose API resource type" aria-haspopup="dialog" aria-expanded={sidebarResourceMenuOpen} on:click={() => toggleSidebarTypeMenu('resource')} on:keydown={(event) => handleSidebarTypeTriggerKeydown(event, 'resource')}><ChevronDown size={15} /></button></div>
            {#if sidebarResourceMenuOpen}
              <div class="sidebar-type-flyout sidebar-resource-flyout" role="dialog" aria-label="Choose API resource type" tabindex="-1" on:keydown={(event) => handleSidebarTypeMenuKeydown(event, 'resource')}><div class="sidebar-type-heading"><div><span><Database size={17} /></span><div><strong>API resource types</strong><small>{resourceWorkspaceResources.length} discovered · filter by category</small></div></div><kbd>esc</kbd></div><div class="sidebar-resource-category-tabs" role="tablist" aria-label="Resource categories"><button class:sidebar-resource-category-active={sidebarResourceCategory === 'All resources'} type="button" role="tab" aria-selected={sidebarResourceCategory === 'All resources'} on:click={() => selectSidebarResourceCategory('All resources')}><span>All</span><b>{resourceWorkspaceResources.length}</b></button>{#each resourceCategories as category}<button class:sidebar-resource-category-active={sidebarResourceCategory === category} type="button" role="tab" aria-selected={sidebarResourceCategory === category} on:click={() => selectSidebarResourceCategory(category)}><span>{category === 'Custom Resources' ? 'Custom APIs' : category}</span><b>{categoryCounts[category] || 0}</b></button>{/each}</div><label class="sidebar-type-search"><Search size={15} /><input bind:value={sidebarResourceSearch} placeholder="Search kind, group, or API version" aria-label="Search API resource types" /></label><div id="sidebar-resource-type-options" class="sidebar-type-options" role="listbox" aria-label="API resource types">{#if sidebarVisibleResources.length}{#each sidebarVisibleResources as resource}<button type="button" role="option" aria-selected={selectedResource !== null && resourceKey(selectedResource) === resourceKey(resource)} class:sidebar-type-selected={selectedResource !== null && resourceKey(selectedResource) === resourceKey(resource)} on:click={() => selectSidebarResourceType(resource)}><span class:custom={resource.crd} class="sidebar-type-icon">{resource.crd ? '◇' : '○'}</span><span><strong>{resource.kind}</strong><small>{resource.apiVersion} · {resource.category}</small></span><b>{selectedResource !== null && resourceKey(selectedResource) === resourceKey(resource) ? '✓' : ''}</b></button>{/each}{:else}<div class="sidebar-type-empty"><strong>No matching API resources</strong><small>Try another type, group, or category.</small></div>{/if}</div></div>
            {/if}
          </div>
        {:else}
          <button class:active={activeView === view} class="nav-item" on:click={() => navigateTo(view as View)}><span class="nav-icon">{#if view === 'Overview'}<LayoutDashboard size={17} strokeWidth={1.8} />{:else if view === 'Events'}<ScrollText size={17} strokeWidth={1.8} />{:else if view === 'CLI'}<Terminal size={17} strokeWidth={1.8} />{:else}<Cable size={17} strokeWidth={1.8} />{/if}</span>{view}{#if view === 'Events' && namespaceClusterEvents.length}<span class="count">{namespaceClusterEvents.length}</span>{/if}{#if view === 'Port forwards'}<span class:port-forward-count-active={activeClusterPortForwards.length > 0} class="count">{activeClusterPortForwards.length}</span>{/if}</button>
        {/if}
      {/each}
    </nav>

    <section class="sidebar-favorites" aria-label="Favorite cluster shortcuts">
      <div class="sidebar-favorites-heading">
        <button class:active={activeView === 'Favorites'} class="sidebar-favorites-title" on:click={() => navigateTo('Favorites')}>
          <Star size={14} strokeWidth={1.9} fill={favoriteClusters.length ? 'currentColor' : 'none'} />
          <span>Favorites</span>
        </button>
        <small>{favoriteClusters.length}/10</small>
      </div>
      {#if favoriteClusters.length}
        <div class="sidebar-favorite-list">
          {#each favoriteClusters as cluster}
            {#if favoriteRenameId === cluster.id}
              <form class="favorite-rename" on:submit|preventDefault={saveFavoriteRename}>
                <input bind:value={favoriteRenameValue} maxlength="80" aria-label={`Rename ${cluster.name} shortcut`} />
                <button type="submit" aria-label="Save shortcut name" title="Save">✓</button>
                <button type="button" aria-label="Cancel shortcut rename" title="Cancel" on:click={cancelFavoriteRename}>×</button>
              </form>
            {:else}
              <button class:favorite-shortcut-active={cluster.id === activeClusterId} class="favorite-shortcut" title={`Open ${favoriteLabel(cluster)} · right-click to rename`} on:click={() => { favoriteContextMenu = null; void selectCluster(cluster.id); }} on:contextmenu={(event) => openFavoriteContextMenu(event, cluster)}>
                <i class="status-dot {cluster.tone}"></i><span>{favoriteLabel(cluster)}</span><b>→</b>
              </button>
            {/if}
          {/each}
        </div>
      {:else}
        <p>Use the star in Available clusters to add up to 10 shortcuts.</p>
      {/if}
    </section>

    {#if favoriteContextMenu && favoriteContextCluster}
      <div class="favorite-context-menu" style:left={`${favoriteContextMenu.x}px`} style:top={`${favoriteContextMenu.y}px`} role="menu" aria-label={`Actions for ${favoriteLabel(favoriteContextCluster)}`}>
        <strong>{favoriteLabel(favoriteContextCluster)}</strong>
        <button on:click={() => startFavoriteRename(favoriteContextCluster.id)}>Rename shortcut</button>
        <button on:click={() => { toggleFavoriteCluster(favoriteContextCluster); favoriteContextMenu = null; }}>Remove from Favorites</button>
      </div>
    {/if}

    <div class="sidebar-bottom">
      <button class:active={activeView === 'Settings'} class="nav-item" on:click={() => navigateTo('Settings')}><span class="nav-icon"><Settings2 size={17} strokeWidth={1.8} /></span>Settings</button>
    </div>
  </aside>
  <div class:sidebar-hidden={sidebarHidden} class="sidebar-resizer" role="separator" aria-orientation="vertical" aria-label="Resize sidebar" on:pointerdown={startSidebarResize}></div>

  <section class="app-shell">
    <header class="topbar">
      <div class="toolbar-leading">
        <button class="sidebar-toggle" aria-label={sidebarHidden ? 'Show sidebar' : 'Hide sidebar'} title={sidebarHidden ? 'Show sidebar' : 'Hide sidebar'} on:click={toggleSidebar}><Menu size={17} strokeWidth={2} /></button>
        <div class="cluster-selector topbar-cluster-selector">
          <button class:cluster-picker-open={clusterPickerOpen} class="cluster-picker" aria-expanded={clusterPickerOpen} on:click={() => (clusterPickerOpen = !clusterPickerOpen)}><span class="cluster-dot"></span><span class="cluster-picker-copy"><small>Active cluster</small><span class="cluster-picker-name">{activeCluster}</span></span><span class="chevron">⌄</span></button>
          {#if clusterPickerOpen}
            <div class="cluster-selector-menu" role="menu" aria-label="Select cluster"><div class="cluster-selector-heading"><span>Clusters</span><b>{clusters.length}</b></div>{#if clusters.length}<div class="cluster-selector-items">{#each clusters as cluster}<button class:chosen={cluster.id === activeClusterId} title={`${cluster.authMethod || cluster.provider} · ${cluster.status}`} on:click={() => selectCluster(cluster.id)}><span class="status-dot {cluster.tone}"></span><span><strong>{cluster.name}</strong><small>{cluster.authMethod || cluster.provider}</small></span>{#if cluster.id === activeClusterId}<i>✓</i>{/if}</button>{/each}</div>{:else}<p class="cluster-selector-empty">No kubeconfig contexts added yet.</p>{/if}<button class="cluster-selector-manage" on:click={() => { clusterPickerOpen = false; void navigateTo('Clusters') }}>Manage clusters <span>→</span></button></div>
          {/if}
        </div>
        {#if showClusterWorkspaceControls}
          <div class="namespace-picker global-namespace-picker">
            <button class="global-namespace-trigger" disabled={namespaceControlBusy} aria-label={`Namespace: ${namespace === 'all namespaces' ? 'All namespaces' : namespace}`} aria-expanded={namespaceOpen} on:click={() => (namespaceOpen = !namespaceOpen)}><span class="namespace-dot"></span><span class="global-namespace-copy"><small>Namespace</small><strong>{namespace === 'all namespaces' ? 'All namespaces' : namespace}</strong></span><span class="chevron">⌄</span></button>
            {#if namespaceOpen}
              <div class="namespace-menu global-namespace-menu" role="menu" aria-label="Select namespace"><button class:namespace-selected={namespace === 'all namespaces'} on:click={() => chooseNamespace('all namespaces')}><span>All namespaces</span>{#if namespace === 'all namespaces'}✓{/if}</button>{#each catalog.namespaces as availableNamespace}<button class:namespace-selected={namespace === availableNamespace} on:click={() => chooseNamespace(availableNamespace)}><span>{availableNamespace}</span>{#if namespace === availableNamespace}✓{/if}</button>{/each}</div>
            {/if}
          </div>
        {/if}
      </div>
      <div class="top-actions">
        {#if showClusterWorkspaceControls}<button class="topbar-refresh" disabled={refreshingCurrentView} aria-label="Reconnect and refresh current view" title="Reconnect to the active cluster and refresh this view" on:click={refreshCurrentView}><RefreshCw size={15} class={refreshingCurrentView ? 'animate-spin' : ''} /><span>{refreshingCurrentView ? 'Refreshing…' : 'Refresh'}</span></button>{/if}
        <button class="command-button" aria-label="Search resources" title="Search resources · ⌘ K" on:click={openCommandSearch}><Search size={15} strokeWidth={2} /><span>Search resources</span><kbd>⌘ K</kbd></button>
        <button class="icon-button theme-toggle" aria-label={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'} title={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'} on:click={toggleTheme}>{#if theme === 'dark'}<Sun size={17} strokeWidth={1.9} />{:else}<Moon size={17} strokeWidth={1.9} />{/if}</button>
        {#if windowControlsAvailable}
          <div class="window-controls" role="group" aria-label="Window controls">
            <button class="icon-button window-size-button" aria-label="Minimize window" title="Minimize window" on:click={minimizeWindow}><Minus size={17} strokeWidth={1.9} /></button>
            <button class="icon-button window-size-button" aria-label={isWindowMaximized ? 'Restore window' : 'Maximize window'} title={isWindowMaximized ? 'Restore window' : 'Maximize window'} on:click={toggleWindowMaximized}>{#if isWindowMaximized}<Minimize2 size={17} strokeWidth={1.9} />{:else}<Maximize2 size={17} strokeWidth={1.9} />{/if}</button>
          </div>
        {/if}
        {#if notifications.length}
          <button class="icon-button" aria-label={`${notifications.length} notification${notifications.length === 1 ? '' : 's'}`} title="Clear notifications" on:click={() => (notifications = [])}><Bell size={18} strokeWidth={1.9} /><i></i></button>
        {/if}
      </div>
    </header>

    <div class="content">
      <div class:cluster-page-heading={activeView === 'Clusters'} class:resource-page-heading={activeView === 'Resources'} class="page-heading">
        <div>
          <div class="title-line"><h1>{activeView}</h1>{#if activeClusterId && !catalogError}<span class="live-pill"><b></b> Live</span>{/if}</div>
          <p>{activeClusterId ? `Browsing ${activeCluster} in real time.` : connectedKubeconfig ? 'Choose a cluster from the sidebar to connect.' : 'Connect a kubeconfig to begin.'}</p>
        </div>
      </div>

      {#if activeView === 'Settings'}
        <section class="settings-theme-panel panel"><div><p class="eyebrow">Appearance</p><h2>{theme === 'dark' ? 'Dark mode' : 'Light mode'}</h2><p>{theme === 'dark' ? 'A low-light palette for long operational sessions.' : 'A bright daylight palette for quick scanning.'}</p></div><button class="secondary settings-theme-button" on:click={toggleTheme}>{#if theme === 'dark'}<Sun size={15} /> Switch to light{:else}<Moon size={15} /> Switch to dark{/if}</button></section>
        <section class="settings-panel panel"><div class="panel-heading"><div><h2>Workspace settings</h2><p>Connection and display preferences for this device.</p></div></div><div class="settings-row"><div><strong>Kubeconfig sources</strong><small>{kubeconfigSources.length ? `${kubeconfigSources.length} tracked source${kubeconfigSources.length === 1 ? '' : 's'} · newest: ${kubeconfigPath || 'Default: ~/.kube/config'}` : 'No kubeconfig source added'}</small></div><button class="secondary" on:click={() => (kubeconfigOpen = true)}>+ Add source</button></div><div class="settings-row"><div><strong>Manual source sync</strong><small>Startup uses the local context snapshot. Sync only when you want Kuberniva to rescan saved files and folders.</small></div><button class="secondary" disabled={loadingCatalog || !kubeconfigSources.length} on:click={syncKubeconfigSources}>{loadingCatalog ? 'Syncing…' : 'Sync sources'}</button></div><div class="settings-row"><div><strong>Loaded contexts</strong><small>{clusters.length ? `${clusters.length} available locally; OIDC is requested only when one is selected` : 'No kubeconfig context is currently available'}</small></div><button class="secondary" on:click={() => activeClusterId ? refreshActiveCluster() : (kubeconfigOpen = true)}>{activeClusterId ? 'Refresh current' : 'Add source'}</button></div></section>
      {:else if restoringWorkspace}
        <section class="overview-loading"><i></i><div><h2>Restoring your workspace…</h2><p>Reading your saved kubeconfig source locally. Kuberniva will not connect to a cluster or start OIDC until you select one.</p></div></section>
      {:else if !connectedKubeconfig}
        <section class="empty-view connect-empty"><div class="explore-orbit"><i></i><i></i><b>⌁</b></div><h2>{kubeconfigSources.length ? 'No cached contexts yet' : 'Your workspace is empty'}</h2><p>{kubeconfigSources.length ? 'Your saved sources are not scanned at startup. Sync them only when you want to discover their current contexts.' : 'Add a kubeconfig to discover the contexts and API resources you can access.'}</p><button class="primary" on:click={() => kubeconfigSources.length ? syncKubeconfigSources() : (kubeconfigOpen = true)}>{kubeconfigSources.length ? 'Sync saved sources' : '+ Add kubeconfig'}</button></section>
      {:else if activeView === 'Clusters'}
        <section class="clusters-landing">
          <div class="clusters-landing-heading"><div><p class="eyebrow">Cluster manager</p><h2>{clusters.length} available cluster{clusters.length === 1 ? '' : 's'}</h2><p>Open a cluster to connect. Add kubeconfigs and remove contexts from Kuberniva without changing source files; OIDC starts only for the cluster you select.</p></div><button class="secondary" on:click={() => (kubeconfigOpen = true)}>+ Add kubeconfig</button></div>
          {#if clusters.length}
            <div class="cluster-list" role="list" aria-label="Tracked clusters">
              <div class="cluster-list-header" aria-hidden="true"><span>Cluster</span><span>Authentication</span><span>Source</span><span>Status</span><span>Favorite</span><span></span></div>
              {#each clusters as cluster}
                <article class:cluster-list-active={cluster.id === activeClusterId} class="cluster-list-row" role="listitem">
                  <button class="cluster-list-open" on:click={() => selectCluster(cluster.id)} title={`Open ${cluster.name}`}><span class="cluster-list-mark"><i class="status-dot {cluster.tone}"></i></span><span class="cluster-list-name"><strong>{cluster.name}</strong><small>{cluster.provider}</small></span><span class="cluster-list-arrow">Open overview →</span></button>
                  <span class="cluster-list-auth">{cluster.authMethod || 'Credentials unavailable'}</span>
                  <small class="cluster-list-source" title={cluster.kubeconfigPath || ''}>{cluster.kubeconfigPath || 'Source path unavailable'}</small>
                  <span class="cluster-list-status"><i class="status-dot {cluster.tone}"></i>{cluster.status}</span>
                  <button class:favorite-toggle-active={isFavoriteCluster(cluster.id)} class="favorite-toggle" aria-pressed={isFavoriteCluster(cluster.id)} aria-label={`${isFavoriteCluster(cluster.id) ? 'Remove' : 'Add'} ${cluster.name} ${isFavoriteCluster(cluster.id) ? 'from' : 'to'} Favorites`} title={isFavoriteCluster(cluster.id) ? 'Remove from Favorites' : 'Add to Favorites'} on:click|stopPropagation={() => toggleFavoriteCluster(cluster)}><Star size={16} fill={isFavoriteCluster(cluster.id) ? 'currentColor' : 'none'} /></button>
                  <button class="remove-cluster-list" title={`Remove ${cluster.name} from Kuberniva only`} on:click={() => removeCluster(cluster.id)}>Remove</button>
                </article>
              {/each}
            </div>
          {:else}
            <div class="cluster-management-empty"><h3>No clusters are currently tracked</h3><p>Connect a kubeconfig source to discover contexts again.</p><button class="primary" on:click={() => (kubeconfigOpen = true)}>Choose kubeconfig source</button></div>
          {/if}
        </section>
      {:else if activeView === 'Favorites'}
        <section class="favorites-page">
          <div class="favorites-page-heading">
            <div><p class="eyebrow">Pinned shortcuts</p><h2>Favorite clusters</h2><p>Keep up to 10 frequently used clusters one click away. Favorites are stored locally on this device.</p></div>
            <span class="favorites-limit"><Star size={16} fill="currentColor" /> {favoriteClusters.length}/10</span>
          </div>
          {#if favoriteClusters.length}
            <div class="favorites-list" role="list" aria-label="Favorite clusters">
              {#each favoriteClusters as cluster}
                <article class:favorite-card-active={cluster.id === activeClusterId} class="favorite-card" role="listitem">
                  <button class="favorite-card-open" title={`Open ${favoriteLabel(cluster)} · right-click to rename`} on:click={() => { favoriteContextMenu = null; void selectCluster(cluster.id); }} on:contextmenu={(event) => openFavoriteContextMenu(event, cluster)}>
                    <span class="favorite-card-mark"><i class="status-dot {cluster.tone}"></i></span>
                    <span class="favorite-card-copy"><strong>{favoriteLabel(cluster)}</strong><small>{cluster.name} · {cluster.provider} · {cluster.authMethod || 'Credentials unavailable'}</small><em>{cluster.status}</em></span>
                    <span class="favorite-card-arrow">Open overview →</span>
                  </button>
                  <button class="favorite-card-remove" aria-label={`Remove ${cluster.name} from Favorites`} title="Remove shortcut" on:click={() => toggleFavoriteCluster(cluster)}><Star size={17} fill="currentColor" /></button>
                </article>
              {/each}
            </div>
          {:else}
            <section class="empty-view favorites-empty"><div class="explore-orbit"><Star size={28} /></div><h2>No favorite clusters yet</h2><p>Open Cluster manager and use the star in a cluster row to create a shortcut here.</p><button class="primary" on:click={() => navigateTo('Clusters')}>Open Cluster manager</button></section>
          {/if}
        </section>
      {:else if activeView === 'Events'}
        <section class="events-page panel">
          <div class="events-heading">
            <div><p class="eyebrow">Cluster activity</p><h2>Events</h2><p>{namespace === 'all namespaces' ? `Recent Kubernetes events across ${activeCluster}.` : `Recent events for ${namespace}, including cluster-scoped activity.`} Warnings stay visible until the API server expires them.</p></div>
            <div class="events-actions"><small>{eventsObservedAt ? `Updated ${formatObservedTime(eventsObservedAt)}` : 'Not loaded yet'}</small></div>
          </div>
          {#if !activeClusterId}
            <div class="events-empty"><ScrollText size={28} /><h3>Select a cluster first</h3><p>Events are fetched only after a live cluster is selected.</p></div>
          {:else if loadingEvents && !clusterEvents.length}
            <div class="events-loading"><i></i><div><h3>Reading cluster events…</h3><p>Fetching the latest Kubernetes Event objects.</p></div></div>
          {:else if eventsError}
            <div class="events-error"><ScrollText size={24} /><div><h3>Events could not be loaded</h3><p>{eventsError}</p></div><button class="secondary" on:click={() => loadClusterEvents(true)}>Try again</button></div>
          {:else}
            <div class="events-toolbar">
              <label class="events-search"><Search size={15} /><input bind:value={eventSearch} placeholder="Filter events, reasons, objects…" aria-label="Filter cluster events" /></label>
              <div class="events-filter" role="group" aria-label="Event severity"><button class:active={eventTypeFilter === 'All'} on:click={() => (eventTypeFilter = 'All')}>All <b>{namespaceClusterEvents.length}</b></button><button class:active={eventTypeFilter === 'Warning'} on:click={() => (eventTypeFilter = 'Warning')}>Warnings <b>{namespaceClusterEvents.filter((event) => event.eventType === 'Warning').length}</b></button><button class:active={eventTypeFilter === 'Normal'} on:click={() => (eventTypeFilter = 'Normal')}>Normal <b>{namespaceClusterEvents.filter((event) => event.eventType !== 'Warning').length}</b></button></div>
            </div>
            {#if visibleClusterEvents.length}
              <div class="events-list" role="list" aria-label="Cluster events">
                {#each visibleClusterEvents as event}
                  <article class:event-warning={eventTone(event.eventType) === 'warning'} class="event-card" role="listitem">
                    <span class="event-severity">{event.eventType === 'Warning' ? '!' : '·'}</span>
                    <div class="event-card-main"><div class="event-card-title"><strong>{event.reason || event.eventType}</strong><span>{event.involvedKind || 'Cluster'}{event.involvedName ? ` · ${event.involvedName}` : ''}</span></div><p>{event.message || 'No event message was supplied.'}</p><div class="event-card-meta"><span>{event.namespace || 'cluster scope'}</span>{#if event.source}<span>{event.source}</span>{/if}{#if event.action}<span>{event.action}</span>{/if}{#if event.count && event.count > 1}<span>×{event.count}</span>{/if}</div></div>
                    <time datetime={event.lastObserved || event.firstObserved || ''}>{formatObservedTime(event.lastObserved || event.firstObserved)}</time>
                  </article>
                {/each}
              </div>
            {:else}
              <div class="events-empty"><ScrollText size={28} /><h3>{namespaceClusterEvents.length ? 'No matching events' : 'No recent events'}</h3><p>{namespaceClusterEvents.length ? 'Try a different filter or severity.' : namespace === 'all namespaces' ? 'This cluster has not returned any retained Kubernetes events.' : `No retained events were returned for ${namespace}.`}</p></div>
            {/if}
          {/if}
        </section>
      {:else if activeView === 'Overview'}
        {#if !activeClusterId}
          <section class="empty-view"><div class="explore-orbit"><i></i><i></i><b>⌁</b></div><h2>Select a cluster</h2><p>{clusters.length} kubeconfig context{clusters.length === 1 ? '' : 's'} loaded locally. Click one in the sidebar when you are ready to authenticate and connect.</p></section>
        {:else if catalogError}
          <section class="empty-view"><div class="explore-orbit"><i></i><i></i><b>!</b></div><h2>{activeCluster} needs attention</h2><p>{catalogError}</p><button class="primary" on:click={refreshActiveCluster}>Try connecting again</button></section>
        {:else if loadingOverview && !clusterOverview}
          <section class="overview-loading"><i></i><div><h2>Reading cluster signals…</h2><p>Loading Nodes and current metrics only for {activeCluster}.</p></div></section>
        {:else if overviewError}
          <section class="overview-error panel"><div><span>!</span><div><h2>Node overview is unavailable</h2><p>{overviewError}</p></div></div><button class="secondary" disabled={loadingOverview} on:click={() => loadClusterOverview(true)}>Try again</button></section>
        {:else if clusterOverview}
          <section class="cluster-overview-dashboard">
          <section class="cluster-hero">
            <div class="cluster-hero-mark"><img class="brand-mark" src="/kuberniva-mark.png" alt="" /></div>
            <div class="cluster-hero-copy"><p class="eyebrow">Cluster overview</p><h2>{activeCluster}</h2><p>Live node capacity and usage. Metrics refresh at most once a minute while this page is open.</p></div>
            <div class="overview-summary"><div><strong>{clusterOverview.nodes.length}</strong><span>Nodes</span></div><div><strong>{readyNodeCount}</strong><span>Ready</span></div><div><strong>{clusterOverview.metricsAvailable ? `${clusterOverview.totals.metricNodes}/${clusterOverview.nodes.length}` : '—'}</strong><span>Metrics nodes</span></div></div>
          </section>
          <section class="node-overview panel">
            <div class="panel-heading"><div><p class="eyebrow">Infrastructure</p><h2>Nodes</h2><p>Cluster-wide capacity first. Select any node below for its exact usage, allocation, network, and runtime details.</p></div><div class="overview-actions"><small>Updated {new Date(clusterOverview.observedAt).toLocaleTimeString()}</small></div></div>
            <section class="cluster-capacity-overview" aria-label="Cluster capacity summary">
              <div class="cluster-capacity-heading"><div><strong>Cluster capacity</strong><small>{clusterOverview.metricsAvailable ? `${clusterOverview.totals.metricNodes} of ${clusterOverview.nodes.length} nodes reporting live CPU and memory` : 'Live usage is unavailable; total capacity remains visible.'}</small></div><span>{clusterOverview.metricsAvailable ? 'Live metrics' : 'Capacity only'}</span></div>
              <div class="cluster-capacity-grid">
                <section class="cluster-capacity-card cluster-capacity-cpu"><div class="cluster-capacity-card-heading"><span>CPU</span><strong>{clusterOverview.totals.cpuUsage || '—'}</strong><b>{usagePercentLabel(clusterOverview.totals.cpuUsagePercent)}</b></div>{#if clusterOverview.totals.cpuUsagePercent !== undefined}<div class="usage-meter" aria-label={`Cluster CPU ${usagePercentLabel(clusterOverview.totals.cpuUsagePercent)}`}><i style:width={`${clusterOverview.totals.cpuUsagePercent}%`}></i></div>{/if}<small>{clusterOverview.totals.cpuUsage ? `${clusterOverview.totals.cpuUsage} used of ${clusterOverview.totals.cpuCapacity || 'unknown capacity'} · ${remainingPercentLabel(clusterOverview.totals.cpuUsagePercent)}` : `Total capacity ${clusterOverview.totals.cpuCapacity || 'unavailable'}`}</small></section>
                <section class="cluster-capacity-card cluster-capacity-memory"><div class="cluster-capacity-card-heading"><span>Memory</span><strong>{clusterOverview.totals.memoryUsage || '—'}</strong><b>{usagePercentLabel(clusterOverview.totals.memoryUsagePercent)}</b></div>{#if clusterOverview.totals.memoryUsagePercent !== undefined}<div class="usage-meter memory-meter" aria-label={`Cluster memory ${usagePercentLabel(clusterOverview.totals.memoryUsagePercent)}`}><i style:width={`${clusterOverview.totals.memoryUsagePercent}%`}></i></div>{/if}<small>{clusterOverview.totals.memoryUsage ? `${clusterOverview.totals.memoryUsage} used of ${clusterOverview.totals.memoryCapacity || 'unknown capacity'} · ${remainingPercentLabel(clusterOverview.totals.memoryUsagePercent)}` : `Total capacity ${clusterOverview.totals.memoryCapacity || 'unavailable'}`}</small></section>
                <section class="cluster-capacity-card cluster-capacity-storage"><div class="cluster-capacity-card-heading"><span>Node storage</span><strong>{clusterOverview.totals.storageCapacity || '—'}</strong></div><small>Total ephemeral storage advertised by all cluster nodes.</small></section>
              </div>
            </section>
            {#if clusterOverview.nodes.length === 0}
              <div class="node-empty">No Nodes were returned by this cluster.</div>
            {:else}
              <div class="node-workbench">
                <aside class="node-list" aria-label="Cluster nodes">
                  {#each clusterOverview.nodes as node}
                    <button class:node-list-active={node.name === selectedNode?.name} on:click={() => (selectedNodeName = node.name)}>
                      <span class="node-list-mark"><i class:ready={node.ready}></i></span>
                      <span><strong>{node.name}</strong><small>{node.roles.length ? node.roles.join(' · ') : 'Worker node'} · {node.architecture || 'architecture unavailable'}</small></span>
                      <b>›</b>
                    </button>
                  {/each}
                </aside>
                {#if selectedNode}
                  <div class="node-inspector">
                    <div class="node-inspector-heading"><div><p class="eyebrow">Node details</p><h3>{selectedNode.name}</h3><p>{selectedNode.ready ? 'Ready and accepting workloads' : 'Not ready · scheduling may be affected'}</p></div><span class:node-inspector-not-ready={!selectedNode.ready} class="node-inspector-status">{selectedNode.ready ? 'Ready' : 'Not ready'}</span></div>
                    <div class="node-detail-grid">
                      <div><span>Architecture</span><strong>{selectedNode.architecture || '—'}</strong></div><div><span>OS image</span><strong>{selectedNode.osImage || '—'}</strong></div><div><span>Kubelet</span><strong>{selectedNode.kubeletVersion || '—'}</strong></div><div><span>Runtime</span><strong>{selectedNode.containerRuntimeVersion || '—'}</strong></div>
                    </div>
                    <div class="node-metrics-grid">
                      <section class="node-metric-card node-metric-cpu"><div class="node-metric-heading"><span>CPU usage</span><strong>{selectedNode.cpuUsage || '—'}</strong><b>{usagePercentLabel(selectedNode.cpuUsagePercent)}</b></div>{#if selectedNode.cpuUsagePercent !== undefined}<div class="usage-meter" aria-label={`CPU ${usagePercentLabel(selectedNode.cpuUsagePercent)}`}><i style:width={`${selectedNode.cpuUsagePercent}%`}></i></div>{/if}<small>{selectedNode.cpuUsage ? `${selectedNode.cpuUsage} used of ${selectedNode.cpuCapacity || 'unknown capacity'} · ${remainingPercentLabel(selectedNode.cpuUsagePercent)}` : `Capacity ${selectedNode.cpuCapacity || 'unavailable'}`}</small></section>
                      <section class="node-metric-card node-metric-memory"><div class="node-metric-heading"><span>Memory usage</span><strong>{selectedNode.memoryUsage || '—'}</strong><b>{usagePercentLabel(selectedNode.memoryUsagePercent)}</b></div>{#if selectedNode.memoryUsagePercent !== undefined}<div class="usage-meter memory-meter" aria-label={`Memory ${usagePercentLabel(selectedNode.memoryUsagePercent)}`}><i style:width={`${selectedNode.memoryUsagePercent}%`}></i></div>{/if}<small>{selectedNode.memoryUsage ? `${selectedNode.memoryUsage} used of ${selectedNode.memoryCapacity || 'unknown capacity'} · ${remainingPercentLabel(selectedNode.memoryUsagePercent)}` : `Capacity ${selectedNode.memoryCapacity || 'unavailable'}`}</small></section>
                    </div>
                    <div class="node-detail-columns">
                      <section class="node-detail-section"><div class="node-detail-section-heading"><strong>Capacity & allocation</strong><small>{selectedNode.unschedulable ? 'Cordoned' : 'Schedulable'}</small></div><div class="node-property-list">{#each selectedNode.capacity as property}<div><span>{property.key}</span><strong>{property.value}</strong><small>allocatable {selectedNode.allocatable.find((candidate) => candidate.key === property.key)?.value || '—'}</small></div>{/each}</div></section>
                      <section class="node-detail-section"><div class="node-detail-section-heading"><strong>Network & identity</strong><small>{selectedNode.podCidrs.length ? selectedNode.podCidrs.join(' · ') : 'Pod CIDR unavailable'}</small></div><div class="node-property-list">{#each selectedNode.addresses as address}<div><span>{address.type}</span><strong>{address.address}</strong></div>{/each}{#if selectedNode.providerId}<div><span>Provider</span><strong>{selectedNode.providerId}</strong></div>{/if}{#if selectedNode.uid}<div><span>UID</span><strong>{selectedNode.uid}</strong></div>{/if}</div></section>
                    </div>
                    <div class="node-detail-columns">
                      <section class="node-detail-section"><div class="node-detail-section-heading"><strong>Conditions</strong><small>{selectedNode.conditions.length}</small></div><div class="node-condition-list">{#each selectedNode.conditions as condition}<div><span class:condition-false={condition.status !== 'True'}>{condition.status === 'True' ? '●' : '○'}</span><strong>{condition.type}</strong><small>{condition.reason || condition.message || condition.status}</small></div>{/each}</div></section>
                      <section class="node-detail-section"><div class="node-detail-section-heading"><strong>Taints</strong><small>{selectedNode.taints.length}</small></div>{#if selectedNode.taints.length}<div class="node-condition-list">{#each selectedNode.taints as taint}<div><span class="condition-false">!</span><strong>{taint.key}</strong><small>{taint.value ? `${taint.value} · ` : ''}{taint.effect}</small></div>{/each}</div>{:else}<p class="node-detail-empty">No taints are currently applied.</p>{/if}</section>
                    </div>
                    {#if selectedNode.labels.length}<details class="node-metadata"><summary>Labels & metadata <span>{selectedNode.labels.length} labels</span></summary><div class="node-metadata-grid">{#each selectedNode.labels as property}<span><b>{property.key}</b><em>{property.value}</em></span>{/each}</div></details>{/if}
                  </div>
                {/if}
              </div>
            {/if}
          </section>
          <section class="overview-quick-actions"><button class="overview-workloads-tile" on:click={() => navigateTo('Workloads')}><span>▦</span><div><em>Operate</em><strong>Workloads</strong><small>Deployments, Pods, logs, terminal access, and ports in the selected namespace.</small></div><b>→</b></button><button class="overview-resources-tile" on:click={() => navigateTo('Resources')}><span>▤</span><div><em>Explore</em><strong>Resources</strong><small>{resourceWorkspaceResources.length} discovered configuration, network, storage, access, cluster, and custom APIs.</small></div><b>→</b></button></section>
          </section>
        {:else}
          <section class="overview-loading"><i></i><div><h2>Preparing cluster overview…</h2><p>Waiting for the first live node response.</p></div></section>
        {/if}
      {:else if activeView === 'Resources'}
        <section class="resource-workbench panel">
          {#if !activeClusterId}
            <div class="connection-error"><strong>Select a cluster to begin</strong><p>Kuberniva has only read your local kubeconfig metadata. No cluster connection or OIDC login has been started.</p></div>
          {:else if catalogError}
            <div class="connection-error"><strong>Unable to connect to {activeCluster}</strong><p>{catalogError}</p><button class="secondary" on:click={refreshActiveCluster}>Try again</button></div>
          {:else if loadingCatalog}
            <div class="connection-error"><strong>Connecting to {activeCluster}…</strong><p>Reading the live API catalog and namespaces.</p></div>
          {:else}
            <div class="resource-focus-heading">
              {#if selectedResource}
                <div class="resource-focus-title"><span class:custom={selectedResource.crd}>{selectedResource.crd ? '◇' : '○'}</span><div><p class="eyebrow">Live resource</p><h2>{selectedResource.kind}</h2><p>{selectedResource.apiVersion} · {selectedResource.namespaced ? (namespace === 'all namespaces' ? 'All namespaces' : namespace) : 'Cluster-wide'}</p></div></div>
                <span class:live-list-status-connected={resourceWatchStatus === 'connected'} class:live-list-status-reconnecting={resourceWatchStatus === 'reconnecting' || resourceWatchStatus === 'connecting'} class="live-list-status"><i></i>{resourceWatchStatus === 'connected' ? 'Live' : resourceWatchStatus === 'reconnecting' ? 'Reconnecting' : resourceWatchStatus === 'connecting' ? 'Connecting' : 'Live updates off'}</span>
              {:else}
                <div class="resource-focus-title"><span>⌁</span><div><p class="eyebrow">Resource workspace</p><h2>Choose a resource</h2><p>Use the Resources menu in the left sidebar to select an API kind.</p></div></div>
              {/if}
            </div>
            <div class="resource-workbench-body resource-workbench-body-focused">
              <aside class="resource-object-browser" aria-label="Resource objects">
                {#if selectedResource}
                  <div class="resource-object-heading resource-pane-heading"><span class="resource-pane-step">01</span><div><span class:custom={selectedResource.crd} class="resource-pane-icon">{selectedResource.crd ? '◇' : '○'}</span><div><strong>{selectedResource.kind} objects</strong><small>{selectedResource.namespaced ? (namespace === 'all namespaces' ? 'All namespaces' : namespace) : 'Cluster-wide'} · {selectedResource.plural}</small></div></div><b>{resourceObjects.length}</b></div>
                  <div class="resource-object-columns" aria-hidden="true"><span></span><span>Name</span><span>Namespace</span><span>Age</span><span>Action</span></div>
                  {#if loadingObjects}<div class="drawer-state"><i></i>Listing {selectedResource.plural}…</div>{:else if resourceObjects.length === 0}<div class="resource-object-empty"><span>○</span><strong>No {selectedResource.plural} found</strong><p>Try another namespace or use Refresh in the top bar.</p></div>{:else}<div class="object-list">{#each resourceObjects as object}<button aria-busy={selectedResource.kind === 'Pod' && isOpeningLogs('Pod', object)} class:object-selected={editorObject?.name === object.name && editorObject?.namespace === object.namespace} on:click={() => openObject(selectedResource!, object)}><span class="object-icon">□</span><div class="resource-object-primary"><strong>{object.name}</strong></div><span class="resource-object-namespace">{object.namespace || 'cluster scoped'}</span><small class="resource-object-age">{object.createdAt ? resourceAge(object.createdAt) : '—'}</small><span class="resource-object-action">{selectedResource.kind === 'Pod' ? (isOpeningLogs('Pod', object) ? 'Opening…' : 'Logs →') : 'Open →'}</span></button>{/each}</div>{/if}
                {:else}
                  <div class="resource-object-empty"><span>⌘</span><strong>Select a resource kind</strong><p>Choose a kind from the left. Kuberniva loads only that API.</p></div>
                {/if}
              </aside>
              <aside class="resource-inspector" aria-label="Resource details">
                <div class="resource-inspector-surface">
                <div class="resource-details-heading resource-pane-heading"><span>02</span><div><strong>Object details</strong><small>Properties, YAML, and actions</small></div></div>
                {#if editorResource && editorObject}
                  <div class="drawer-heading inspector-heading"><div><span class:custom={editorResource.custom}>{editorResource.kind === 'Secret' ? '◈' : editorResource.kind === 'ConfigMap' ? '◇' : '⌁'}</span><div><h2>{editorObject.name}</h2><p>{editorResource.kind} · {editorObject.namespace || 'cluster scoped'}</p></div></div><button aria-label="Back to resource objects" on:click={() => closeEditor()}>×</button></div>
                  {#if loadingEditor}
                    <div class="drawer-state"><i></i>Loading live resource details…</div>
                  {:else}
                    {#if editorCertificate}
                      <section class:expired={editorCertificate.expired} class="certificate-card"><div><span>⌁</span><div><strong>{editorCertificate.expired ? 'Certificate expired' : 'TLS certificate'}</strong><p>Expires {editorCertificate.expiresAt}</p></div></div><b>{editorCertificate.expired ? `${Math.abs(editorCertificate.daysRemaining)} days ago` : `${editorCertificate.daysRemaining} days remaining`}</b></section>
                    {/if}
                    {#if editorResource.kind === 'Secret' || editorResource.kind === 'ConfigMap'}
                      <section class="configuration-inspector focus-canvas-editor">
                        <div class="configuration-data-heading"><div><span>{editorResource.kind === 'Secret' ? '◈' : '◇'}</span><div><strong>{editorResource.kind === 'Secret' ? 'Secret data' : 'ConfigMap data'}</strong><small>{editorResource.kind === 'Secret' ? (revealSecret ? 'Decoded values are visible locally' : 'Values are base64 encoded') : 'Plain-text values loaded from this namespace'}</small></div></div><b>{editorEntries.length} {editorEntries.length === 1 ? 'entry' : 'entries'}</b></div>
                        <div class="editor-toolbar focus-canvas-toolbar"><div><strong>{editorResource.kind === 'Secret' ? 'Protecting sensitive values' : 'Focus editor'}</strong><small>{editorResource.kind === 'Secret' ? 'Choose a key on the left, then reveal or edit its value.' : 'Choose a key on the left to give its value room to breathe.'}</small></div>{#if editorResource.kind === 'Secret'}<button class="reveal-button" on:click={() => (revealSecret = !revealSecret)}>{revealSecret ? '◉ Hide decoded' : '◌ Reveal decoded'}</button>{/if}</div>
                        <div class="focus-canvas-shell">
                          {#if editorEntries.length === 0}
                            <div class="drawer-state focus-canvas-empty">This {editorResource!.kind} has no data entries.<button class="focus-canvas-empty-action" on:click={addEditorEntry}>＋ Add key</button></div>
                          {:else}
                            <nav class="focus-canvas-rail" aria-label={`${editorResource.kind} keys`}>
                              <div class="focus-canvas-rail-heading"><span>Keys</span><b>{editorEntries.length}</b></div>
                              <div class="focus-canvas-key-list">{#each editorEntries as entry, index}<button type="button" class:focus-canvas-key-active={index === focusedEditorEntry} class="focus-canvas-key" on:click={() => (focusedEditorEntry = index)}><i></i><span>{entry.key || 'Unnamed key'}</span></button>{/each}</div>
                              <div class="focus-canvas-rail-actions"><button type="button" class="focus-canvas-remove focus-canvas-remove-rail" on:click={() => removeEditorEntry(focusedEditorEntry)}>Remove selected</button><button type="button" class="focus-canvas-add" on:click={addEditorEntry}><span>＋</span>Add key</button></div>
                            </nav>
                            {#if focusedEditorEntryData}
                              <div class="focus-canvas-main">
                                <label class="focus-canvas-field focus-canvas-value-field"><span>Value</span><textarea use:autoSizeTextarea={editorResource.kind === 'Secret' && revealSecret ? decodeSecret(focusedEditorEntryData.value) : focusedEditorEntryData.value} value={editorResource.kind === 'Secret' && revealSecret ? decodeSecret(focusedEditorEntryData.value) : focusedEditorEntryData.value} on:input={(event) => updateEditorEntry(focusedEditorEntry, event.currentTarget.value)} spellcheck="false"></textarea></label>
                                <div class="focus-canvas-hint"><span>✓</span>{editorResource.kind === 'Secret' ? (revealSecret ? 'Decoded locally; saving writes base64 back to Kubernetes.' : 'Values remain encoded until you reveal them.') : 'Changes are staged locally until you save.'}</div>
                              </div>
                            {/if}
                          {/if}
                        </div>
                      </section>
                    {:else}
                      <div class="inspector-overview">
                        <section class="inspector-summary-grid"><div><span>Kind</span><strong>{editorResource.kind}</strong></div><div><span>Scope</span><strong>{editorObject.namespace || 'Cluster-wide'}</strong></div><div><span>API</span><strong>{editorResource.apiVersion}</strong></div></section>
                        {#if editorResource.category === 'Network' || editorResource.category === 'Gateway APIs'}
                          {#if editorResource.kind === 'Service'}
                            <section class="service-overview-card">
                              <div class="service-overview-heading"><div><span class="service-overview-icon">⇄</span><div><strong>Service routing</strong><small>Exposure, address, and traffic policy</small></div></div><b class="service-type-badge">{networkServiceType(editorManifest)}</b></div>
                              <div class="service-fact-grid"><div class="service-fact service-fact-type"><span>Service type</span><strong>{networkServiceType(editorManifest)}</strong><small>{networkServiceExposure(editorManifest)}</small></div><div class="service-fact"><span>Cluster IP</span><strong>{networkServiceClusterIp(editorManifest)}</strong><small>{networkPorts(editorManifest).length} declared {networkPorts(editorManifest).length === 1 ? 'port' : 'ports'}</small></div><div class="service-fact"><span>Traffic policy</span><strong>{networkServiceTrafficPolicy(editorManifest)}</strong><small>How traffic is routed</small></div></div>
                              {#if networkServiceExternalEndpoints(editorManifest).length}
                                <div class="service-external-card service-external-card-active"><span>↗</span><div><strong>External endpoint{networkServiceExternalEndpoints(editorManifest).length === 1 ? '' : 's'}</strong><small>Reachable outside the cluster</small><div class="service-endpoint-list">{#each networkServiceExternalEndpoints(editorManifest) as endpoint}<code>{endpoint}</code>{/each}</div></div></div>
                              {:else if networkServiceType(editorManifest) === 'LoadBalancer'}
                                <div class="service-external-card service-external-card-pending"><span>…</span><div><strong>External address pending</strong><small>The LoadBalancer has not received an IP or hostname yet.</small></div></div>
                              {:else}
                                <div class="service-external-card"><span>●</span><div><strong>No external endpoint</strong><small>This service is currently reachable only through cluster networking.</small></div></div>
                              {/if}
                            </section>
                          {/if}
                          <section class="network-inspector-card"><div><strong>Hosts &amp; addresses</strong><small>Resolved from service status, endpoint subsets, and Gateway route hosts</small></div>{#if networkAddressFacts(editorManifest).length}<div class="network-fact-list">{#each networkAddressFacts(editorManifest) as fact}<div class={`network-fact network-fact-${fact.tone}`}><span>{fact.label}</span><strong>{fact.value}</strong></div>{/each}</div>{:else}<p>No host or address is declared on this resource.</p>{/if}</section>
                          <section class="network-inspector-card"><div><strong>Ports &amp; listeners</strong><small>Service ports, endpoint ports, route backends, and Gateway listeners</small></div>{#if networkPortFacts(editorManifest).length}<div class="network-fact-list">{#each networkPortFacts(editorManifest) as fact}<div class={`network-fact network-fact-${fact.tone}`}><span>{fact.label}</span><strong>{fact.value}</strong></div>{/each}</div>{:else}<p>No ports or listeners are declared on this resource.</p>{/if}</section>
                        {/if}
                        {#if resourceLabels(editorManifest).length}<section class="resource-labels"><strong>Labels</strong><div class="inspector-chip-list">{#each resourceLabels(editorManifest) as [key, value]}<span><b>{key}</b>{value}</span>{/each}</div></section>{/if}
                        <details class="resource-properties" open><summary>Resource properties</summary><pre>{genericResourcePreview(editorManifest)}</pre></details>
                      </div>
                    {/if}
                  {/if}
                  <div class="drawer-footer"><span>{editorResource.kind === 'Secret' ? 'Decoded values never leave this device.' : 'Loaded directly from the Kubernetes API.'}</span><div class="editor-footer-actions"><button class="secondary" disabled={loadingEditor} on:click={() => openYamlEditor(editorResource!, editorObject!)}>View YAML</button><button class="destructive" disabled={loadingEditor || savingEditor} on:click={() => requestResourceDeletion(editorResource!, editorObject!)}>Delete</button>{#if editorResource.kind === 'Secret' || editorResource.kind === 'ConfigMap'}<button class="primary" disabled={loadingEditor || savingEditor} on:click={saveEditor}>{savingEditor ? 'Saving…' : 'Save changes'}</button>{/if}</div></div>
                {:else}
                  <div class="inspector-empty"><span>{selectedResource?.crd ? '◇' : '⌁'}</span><h3>{selectedResource ? `Choose a ${selectedResource.kind}` : 'Ready when you are'}</h3><p>{selectedResource ? 'Select an object from the list to view its live properties, edit supported data, or open YAML.' : 'Pick an API type to load its objects. Kuberniva does not fan out requests in the background.'}</p></div>
                {/if}
              </div>
            </aside>
            </div>
          {/if}
        </section>
      {:else if activeView === 'CLI'}
        <section class="cli-page panel">
          <div class="cli-heading"><div class="cli-heading-mark"><Terminal size={22} strokeWidth={1.8} /></div><div><p class="eyebrow">Cluster command line</p><h2>CLI</h2><p>Run kubectl, Helm, kustomize, or any installed command against this workspace. Direct Kubernetes commands receive the active kubeconfig, context, and namespace automatically.</p></div><span class="cli-context-badge">{activeCluster}</span></div>
          <div class="cli-context-strip"><div><span>Context</span><strong>{activeCluster}</strong></div><div><span>Namespace</span><strong>{namespace === 'all namespaces' ? 'All namespaces' : namespace}</strong></div><div><span>Kubeconfig</span><strong title={activeKubeconfigPath || kubeconfigPath || 'Default kubeconfig'}>{activeKubeconfigPath || kubeconfigPath || 'Default kubeconfig'}</strong></div></div>
          <form class="cli-command-form" on:submit|preventDefault={runClusterCli}><span class="cli-prompt">$</span><input bind:value={cliCommand} aria-label="Cluster command" placeholder="kubectl get pods -o wide" spellcheck="false" /><button class="primary" type="submit" disabled={runningCli}>{runningCli ? 'Running…' : 'Run command'}</button></form>
          <div class="cli-controls"><label class="cli-shell-toggle"><input type="checkbox" bind:checked={cliShellMode} /><span>Shell mode</span></label><small>{cliShellMode ? 'Runs the command through your login shell for pipes, redirects, and local helpers.' : 'Direct mode runs one executable with quote-aware arguments. Try kubectl, helm, kustomize, or any command on PATH.'}</small></div>
          <div class="cli-suggestions"><span>Quick commands</span><button type="button" on:click={() => (cliCommand = 'kubectl get pods -o wide')}>kubectl get pods</button><button type="button" on:click={() => (cliCommand = 'kubectl get events --sort-by=.lastTimestamp')}>kubectl events</button><button type="button" on:click={() => (cliCommand = 'helm list --all-namespaces')}>helm list</button><button type="button" on:click={() => (cliCommand = 'kubectl api-resources')}>api-resources</button></div>
          <pre class="cli-output" aria-live="polite">{cliOutput || 'Output will appear here. Kubernetes tools are bound to the active context; shell mode is available for pipelines and other local commands.'}</pre>
        </section>
      {:else if activeView === 'Port forwards'}
        <section class="port-forwards-page panel">
          <div class="port-forwards-heading"><div class="port-forwards-heading-mark"><Cable size={22} strokeWidth={1.8} /></div><div><p class="eyebrow">Local listeners</p><h2>Port forwarding</h2><p>Listeners shown here belong only to {activeClusterId ? activeCluster : 'the selected cluster'} and remain active until you stop them or quit Kuberniva.</p></div><div class:port-forward-summary-active={activeClusterPortForwards.length > 0} class="port-forward-summary"><span class="port-forward-listening-dot"></span><strong>{activeClusterPortForwards.length}</strong><small>{activeClusterPortForwards.length === 1 ? 'active listener' : 'active listeners'}</small></div><button class="secondary port-forward-refresh" disabled={syncingPortForwards} on:click={() => syncPortForwards(true)}><RefreshCw size={15} class={syncingPortForwards ? 'animate-spin' : ''} />{syncingPortForwards ? 'Checking…' : 'Check status'}</button></div>
          {#if syncingPortForwards && activeClusterPortForwards.length === 0}
            <div class="port-forward-page-state"><RefreshCw size={21} class="animate-spin" /><strong>Checking native listeners…</strong></div>
          {:else if activeClusterPortForwards.length === 0}
            <div class="port-forward-page-empty"><span><Cable size={24} strokeWidth={1.7} /></span><h3>No active port forwards for this cluster</h3><p>Open a Pod’s logs and choose <strong>Forward</strong>. Only listeners from {activeClusterId ? activeCluster : 'the selected cluster'} appear here.</p><button class="primary" on:click={() => navigateTo('Workloads')}>Browse workloads</button></div>
          {:else}
            <div class="port-forward-table" role="table" aria-label="Active port forwards">
              <div class="port-forward-table-header" role="row"><span>Status</span><span>Local listener</span><span>Pod target</span><span>Cluster context</span><span>Action</span></div>
              {#each activeClusterPortForwards as forward}
                <div class="port-forward-table-row" role="row"><div class="port-forward-status"><span class="port-forward-listening-dot"></span><strong>Listening</strong></div><div class="port-forward-endpoint"><strong>{forward.localAddress}</strong><small>localhost:{forward.localPort}</small></div><div class="port-forward-endpoint"><strong>{forward.namespace}/{forward.pod}</strong><small>Remote port {forward.remotePort}</small></div><div class="port-forward-context"><strong>{forward.context || 'Current context'}</strong><small>Native Kubernetes tunnel</small></div><button class="port-forward-stop" disabled={Boolean(stoppingPortForwardId)} on:click={() => stopPortForward(forward.id)}>{stoppingPortForwardId === forward.id ? 'Stopping…' : 'Stop forward'}</button></div>
              {/each}
            </div>
          {/if}
        </section>
      {:else if activeView === 'Workloads'}
        {#if !activeClusterId}
          <section class="empty-view"><div class="explore-orbit"><i></i><i></i><b>▦</b></div><h2>Select a cluster first</h2><p>Workload inventory is loaded only for the cluster and namespace you choose.</p></section>
        {:else}
          <section class="workloads-page font-sans">
            <div class:workload-detail-open={(editorResource?.category === 'Workloads' && editorObject !== null) || (workloadDetailMode === 'logs' && logTarget !== null)} class:workload-logs-open={workloadDetailMode === 'logs' && logTarget !== null} class="workload-grid grid min-h-[560px]">
              <div class="workload-list-panel overflow-hidden rounded-2xl border border-white/10 bg-[#151924]/90 shadow-2xl shadow-black/10"><div class="workload-list-header flex items-center justify-between gap-4 border-b border-white/10 px-5 py-4"><div><div class="flex items-center gap-2"><Container size={18} class="text-cyan-300" /><h3 class="m-0 text-lg font-semibold text-white">{workloadResource?.kind || 'Select a type'}</h3></div><p class="mb-0 mt-1 text-xs text-slate-400">{namespace} · {workloadResource?.apiVersion || 'Kubernetes API'}{#if workloadResource?.kind === 'Pod'} · CPU/memory from Metrics API{/if}</p></div><span class:live-list-status-connected={resourceWatchStatus === 'connected'} class:live-list-status-reconnecting={resourceWatchStatus === 'reconnecting' || resourceWatchStatus === 'connecting'} class="live-list-status"><i></i>{resourceWatchStatus === 'connected' ? 'Live' : resourceWatchStatus === 'reconnecting' ? 'Reconnecting' : resourceWatchStatus === 'connecting' ? 'Connecting' : 'Live updates off'}</span><label class="flex h-10 w-72 items-center gap-2 rounded-lg border border-white/10 bg-black/20 px-3 text-slate-500 focus-within:border-indigo-400 focus-within:bg-black/30 focus-within:ring-2 focus-within:ring-indigo-500/20"><Search size={16} /><input class="min-w-0 flex-1 border-0 bg-transparent text-sm text-slate-100 outline-none placeholder:text-slate-500" bind:value={workloadSearch} placeholder={`Filter ${workloadResource?.plural || 'workloads'}`} /></label></div>
                {#if loadingWorkloads}
                  <div class="grid min-h-96 place-items-center text-sm text-slate-400"><div class="flex items-center gap-3"><RefreshCw size={18} class="animate-spin text-cyan-300" />Loading {workloadResource?.plural || 'workloads'}…</div></div>
                {:else if visibleWorkloadObjects.length === 0}
                  <div class="grid min-h-96 place-items-center px-6 text-center"><div><div class="mx-auto grid h-12 w-12 place-items-center rounded-2xl bg-indigo-500/15 text-cyan-300"><Boxes size={22} /></div><h4 class="mb-0 mt-4 text-base font-semibold text-slate-100">{workloadObjects.length ? 'No matching workloads' : `No ${workloadResource?.plural || 'workloads'} found`}</h4><p class="mb-0 mt-2 text-sm text-slate-400">{workloadObjects.length ? 'Try a different name or namespace filter.' : `Nothing was returned for ${namespace}.`}</p></div></div>
                {:else}
                  <div class="workload-object-list" aria-label={`${workloadResource?.kind || 'Workload'} list`}>
                    <div class:workload-pod-row={workloadResource?.kind === 'Pod'} class="workload-object-list-header" aria-label="Workload columns"><span></span><span>Name</span><span>Status</span>{#if workloadResource?.kind === 'Pod'}<span>Ready</span><span>CPU</span><span>Memory</span>{/if}<span>Age</span><span></span></div>
                    {#each visibleWorkloadObjects as workload}
                      <button
                        class:workload-pod-row={workloadResource?.kind === 'Pod'}
                        class:workload-object-selected={Boolean(
                          editorResource
                            && editorObject
                            && workloadResource
                            && resourceKey(editorResource) === resourceKey(workloadResource)
                            && editorObject.name === workload.name
                            && editorObject.namespace === workload.namespace
                        )}
                        class="workload-object-row group"
                        on:click={() => workloadResource && openObject(workloadResource, workload)}
                      >
                        <span class={`workload-status-dot ${workloadStatusTone(workload)}`}></span>
                        <div class="workload-object-name"><strong>{workload.name}</strong><small>{workload.namespace || 'cluster scope'}{#if workload.nodeName} · {workload.nodeName}{/if}</small></div>
                        <div class="workload-row-fact"><b class={`workload-status-label ${workloadStatusTone(workload)}`}>{workloadStatusLabel(workload)}</b></div>
                        {#if workloadResource?.kind === 'Pod'}<div class="workload-row-fact"><b>{podContainerSummary(workload)}{#if workload.restarts !== undefined && workload.restarts > 0}<small> · {workload.restarts} restarts</small>{/if}</b></div><div class="workload-row-fact"><b title={workload.cpuUsage || 'Metrics unavailable'}>{podMetricLabel(workload.cpuUsage)}</b></div><div class="workload-row-fact"><b title={workload.memoryUsage || 'Metrics unavailable'}>{podMetricLabel(workload.memoryUsage)}</b></div>{/if}
                        <div class="workload-row-fact workload-row-age"><b>{resourceAge(workload.createdAt)}</b></div>
                        <ChevronRight size={17} class="workload-row-arrow" />
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
              {#if workloadDetailMode === 'logs' && logTarget}
                <aside class="workload-inspector workload-log-inspector" aria-label={`${logTarget.pod} logs`}>
                  <div class="workload-inspector-heading workload-log-heading"><div><p class="eyebrow">Live logs</p><h3>{logTarget.pod}</h3><p>{activeCluster} · {logScopeLabel || 'Pod'} · {logTarget.namespace}</p></div><div class="workload-inspector-actions"><button class="secondary workload-log-back" on:click={closeWorkloadLogs}>← Details</button><button aria-label="Close logs" on:click={closeWorkloadLogs}>×</button></div></div>
                  <div class="workload-log-body">
                    <section class="workload-log-pod-picker"><div><div><strong>Pod stream</strong><small>{logPods.length} available for this workload</small></div><label>Switch Pod<select value={logTargetKey(logTarget)} on:change={(event) => selectLogPodByKey(event.currentTarget.value)}>{#each logPods as pod}<option value={logPodKey(pod)}>{pod.name} · {pod.namespace || namespace}</option>{/each}</select></label></div></section>
                    <section class="workload-log-toolbar"><div><strong>Live logs</strong><small>{openingLogsTarget ? 'Opening the first stream…' : loadingLogs ? 'Refreshing…' : 'Refreshes every 30 seconds · scroll up to hold your place'}</small></div><div class="workload-log-toolbar-actions">{#if logContainers.length > 1}<label>Container <select bind:value={selectedLogContainer} on:change={() => loadLogs(true)}>{#each logContainers as container}<option value={container}>{container}</option>{/each}</select></label>{/if}<button class:port-forward-open={portForwardOpen} class="port-forward-button" on:click={openPortForwardForm}>⇄ Forward</button></div></section>
                    {#if logPorts.length}<section class="workload-log-ports"><strong>Container ports</strong><div>{#each logPorts as port}<span title={`${port.container}${port.name ? ` · ${port.name}` : ''} · ${port.protocol}`}>{port.port}/{port.protocol}<small>{port.container}</small></span>{/each}</div></section>{/if}
                    {#if portForwardOpen}<section class="port-forward-form workload-log-forward-form"><div><strong>Port forward</strong><button aria-label="Close port forward" on:click={() => (portForwardOpen = false)}>×</button></div><p>Expose {logTarget.pod} only on this Mac.</p><label>Remote port<input list="kuberniva-pod-ports" type="number" min="1" max="65535" bind:value={portForwardRemotePort} placeholder="e.g. 8080" /></label><datalist id="kuberniva-pod-ports">{#each suggestedForwardPorts as port}<option value={port}></option>{/each}</datalist><label>Local port<input type="number" min="1" max="65535" bind:value={portForwardLocalPort} placeholder="e.g. 8080" /></label><button class="primary" disabled={portForwarding} on:click={startPortForward}>{portForwarding ? 'Starting…' : 'Start forward'}</button></section>{/if}
                    {#if selectedPodPortForwards.length}<section class="log-active-forwards"><div><span>Active forwards</span><small>{selectedPodPortForwards.length}</small></div>{#each selectedPodPortForwards as forward}<div class="log-active-forward"><span class="port-forward-listening-dot"></span><div><strong>{forward.localAddress}</strong><small>Local → {forward.remotePort}</small></div><button disabled={Boolean(stoppingPortForwardId)} on:click={() => stopPortForward(forward.id)}>{stoppingPortForwardId === forward.id ? 'Stopping…' : 'Stop'}</button></div>{/each}</section>{/if}
                    {#if loadingLogs && logLines.length === 0}<div class="workload-log-opening"><RefreshCw size={18} class="animate-spin" /><div><strong>Opening logs…</strong><small>Connecting to {logTarget.pod}{selectedLogContainer ? ` · ${selectedLogContainer}` : ''}</small></div></div>{:else}<pre bind:this={logViewport} class="workload-log-output">{logLines.length ? logLines.join('\n') : 'No log lines returned yet.'}</pre>{/if}
                  </div>
                </aside>
              {:else if editorResource && editorObject && editorResource.category === 'Workloads'}
                <aside class="workload-inspector" aria-label="Workload details">
                  <div class="workload-inspector-heading"><div><p class="eyebrow">Live workload details</p><h3>{editorObject.name}</h3><p>{editorResource.kind} · {editorObject.namespace || 'cluster scoped'}</p></div><div class="workload-inspector-actions"><button class="workload-delete" disabled={loadingEditor} on:click={() => requestResourceDeletion(editorResource!, editorObject!)}>Delete</button><button aria-label="Close workload details" on:click={() => closeEditor()}>×</button></div></div>
                  {#if loadingEditor}
                    <div class="drawer-state"><i></i>Loading live workload details…</div>
                  {:else if workloadDetailMode === 'terminal'}
                    <section class="workload-terminal">
                      <div class="workload-terminal-heading"><button on:click={resetWorkloadTerminal}>← Details</button><div><strong>Pod terminal</strong><small>Kubernetes exec — runs in the selected container</small></div></div>
                      {#if loadingTerminalPods}<div class="drawer-state"><i></i>Finding live Pods…</div>{:else if terminalPods.length === 0}<div class="workload-terminal-empty"><strong>No live Pod is available</strong><p>Wait for a replica to become ready, then try again.</p></div>{:else}
                        <div class="terminal-pod-strip"><span>Pod</span><div>{#each terminalPods as pod}<button class:terminal-pod-selected={terminalTarget?.pod === pod.name && terminalTarget?.namespace === (pod.namespace || namespace)} on:click={() => selectTerminalPod(pod)}>{pod.name}</button>{/each}</div></div>
                        {#if loadingTerminalRuntime}<div class="drawer-state"><i></i>Inspecting containers…</div>{:else if terminalTarget}
                          <div class="terminal-target"><div><span>Connected target</span><strong>{terminalTarget.namespace}/{terminalTarget.pod}</strong></div>{#if terminalContainers.length > 1}<label>Container<select bind:value={selectedTerminalContainer}>{#each terminalContainers as container}<option value={container}>{container}</option>{/each}</select></label>{:else}<small>{selectedTerminalContainer || 'Container unavailable'}</small>{/if}</div>
                          {#if terminalPorts.length}<div class="terminal-port-hints"><span>Declared ports</span><div>{#each terminalPorts as port}<small>{port.container} · {port.port}/{port.protocol}</small>{/each}</div></div>{/if}
                          <form class="terminal-command" on:submit|preventDefault={runTerminalCommand}><label>Command<input bind:value={terminalCommand} placeholder="e.g. printenv | sort" spellcheck="false" /></label><button class="primary" disabled={runningTerminalCommand}>{runningTerminalCommand ? 'Running…' : 'Run'}</button></form>
                          <pre class="terminal-output">{terminalOutput || 'Run a command to inspect this container. Commands use /bin/sh -lc and require pods/exec permission.'}</pre>
                        {/if}
                      {/if}
                    </section>
                  {:else}
                    <div class="workload-inspector-body">
                      <section class="workload-action-grid"><button class:workload-action-loading={editorLogsOpening} class="workload-action-card workload-logs-action" disabled={Boolean(openingLogsTarget)} aria-busy={editorLogsOpening} on:click={() => openWorkloadLogs(editorResource!, editorObject!)}><span>{#if editorLogsOpening}<RefreshCw size={18} class="workload-action-spinner" />{:else}≡{/if}</span><div><strong>{editorLogsOpening ? 'Opening logs…' : 'View logs'}</strong><small>{editorLogsOpening ? `Preparing ${editorResource.kind} logs and the first live stream` : editorResource.kind === 'Pod' ? 'Keep workload types visible beside this Pod stream' : 'Choose a live Pod and stream its output without leaving Workloads'}</small></div><b>{editorLogsOpening ? '•••' : '→'}</b></button><button class="workload-action-card workload-terminal-action" disabled={loadingTerminalPods || Boolean(openingLogsTarget)} on:click={() => openWorkloadTerminal(editorResource!, editorObject!)}><span>⌘</span><div><strong>Terminal</strong><small>Tunnel into a Pod container with Kubernetes exec</small></div><b>→</b></button></section>
                      <section class="workload-status-grid"><div><span>Replicas</span><strong>{workloadReplicaSummary(editorManifest)}</strong></div><div><span>API</span><strong>{editorResource.apiVersion}</strong></div></section>
                      {#if editorResource.kind === 'Pod'}<section class="workload-pod-facts"><div><span>Phase</span><strong class={`workload-status-label ${workloadStatusTone(editorObject)}`}>{workloadStatusLabel(editorObject)}</strong></div><div><span>Containers</span><strong>{podContainerSummary(editorObject)}</strong></div><div><span>Restarts</span><strong>{editorObject.restarts ?? 0}</strong></div><div><span>Age</span><strong>{resourceAge(editorObject.createdAt)}</strong></div>{#if editorObject.cpuUsage}<div><span>CPU used</span><strong>{editorObject.cpuUsage}</strong></div>{/if}{#if editorObject.memoryUsage}<div><span>Memory used</span><strong>{editorObject.memoryUsage}</strong></div>{/if}{#if editorObject.nodeName}<div class="workload-pod-fact-wide"><span>Node</span><strong>{editorObject.nodeName}</strong></div>{/if}</section>{/if}
                      <section class="workload-detail-card"><div class="workload-detail-card-heading"><div><strong>Container images</strong><small>Images declared on the Pod template</small></div><b>{workloadImages(editorManifest).length}</b></div>{#if workloadImages(editorManifest).length}<div class="workload-image-list">{#each workloadImages(editorManifest) as container}<div><span>{container.init ? 'Init' : 'App'}</span><strong>{container.name}</strong><small title={container.image}>{container.image}</small></div>{/each}</div>{:else}<p>No container image is declared on this resource.</p>{/if}</section>
                      <section class="workload-attachment-grid"><div class="workload-detail-card"><div class="workload-detail-card-heading"><div><strong>ConfigMaps</strong><small>Environment and volume references</small></div><b>{workloadAttachments(editorManifest).configMaps.length}</b></div>{#if workloadAttachments(editorManifest).configMaps.length}<div class="workload-reference-list">{#each workloadAttachments(editorManifest).configMaps as configMap}<span>◇ {configMap}</span>{/each}</div>{:else}<p>No ConfigMap is attached.</p>{/if}</div><div class="workload-detail-card"><div class="workload-detail-card-heading"><div><strong>Secrets</strong><small>Environment, pull, and volume references</small></div><b>{workloadAttachments(editorManifest).secrets.length}</b></div>{#if workloadAttachments(editorManifest).secrets.length}<div class="workload-reference-list secret-reference-list">{#each workloadAttachments(editorManifest).secrets as secret}<span>◈ {secret}</span>{/each}</div>{:else}<p>No Secret is attached.</p>{/if}</div></section>
                      {#if resourceLabels(editorManifest).length}<section class="resource-labels workload-labels"><strong>Labels</strong><div class="inspector-chip-list">{#each resourceLabels(editorManifest) as [key, value]}<span><b>{key}</b>{value}</span>{/each}</div></section>{/if}
                    </div>
                  {/if}
                </aside>
              {/if}
            </div>
          </section>
        {/if}
      {:else if activeView === 'Explore'}
        <section class="empty-view"><div class="explore-orbit"><i></i><i></i><b>⌕</b></div><h2>Explore without memorizing paths</h2><p>Ask for a resource, filter it, and move between related objects in one place.</p><button class="primary" on:click={openCommandSearch}>Search resources</button></section>
      {:else}
        {#if logTarget}
          <section class="logs-workspace panel">
            <aside class="log-pod-sidebar" aria-label="Pods with logs">
              <div class="log-pod-sidebar-heading"><div><p class="eyebrow">{logScopeLabel || 'Pod'} stream</p><h2>Pods</h2><p>{logPods.length} available in this view</p></div><div class="log-pod-heading-actions"><button class:port-forward-open={portForwardOpen} class="port-forward-button" on:click={openPortForwardForm}>⇄ Forward</button><span>{logPods.length}</span></div></div>
              <div class="log-pod-list">{#each logPods as pod}<button class:log-pod-selected={logTarget.pod === pod.name && logTarget.namespace === (pod.namespace || namespace)} on:click={() => selectLogPod(pod)}><span class="log-pod-dot"></span><div><strong>{pod.name}</strong><small>{pod.namespace || namespace}</small></div><span class="log-pod-arrow">→</span></button>{/each}</div>
              {#if logPorts.length}<section class="log-port-section"><div><span>Container ports</span><small>{logPorts.length}</small></div>{#each logPorts as port}<span class="log-port-chip" title={`${port.container}${port.name ? ` · ${port.name}` : ''} · ${port.protocol}`}><b>{port.port}/{port.protocol}</b><small>{port.container}{port.name ? ` · ${port.name}` : ''}</small></span>{/each}</section>{/if}
              {#if portForwardOpen}<section class="port-forward-form"><div><strong>Port forward</strong><button aria-label="Close port forward" on:click={() => (portForwardOpen = false)}>×</button></div><p>Expose {logTarget.pod} only on this Mac.</p><label>Remote port<input list="kuberniva-pod-ports" type="number" min="1" max="65535" bind:value={portForwardRemotePort} placeholder="e.g. 8080" /></label><datalist id="kuberniva-pod-ports">{#each suggestedForwardPorts as port}<option value={port}></option>{/each}</datalist><label>Local port<input type="number" min="1" max="65535" bind:value={portForwardLocalPort} placeholder="e.g. 8080" /></label><button class="primary" disabled={portForwarding} on:click={startPortForward}>{portForwarding ? 'Starting…' : 'Start forward'}</button></section>{/if}
              {#if selectedPodPortForwards.length}<section class="log-active-forwards"><div><span>Active forwards</span><small>{selectedPodPortForwards.length}</small></div>{#each selectedPodPortForwards as forward}<div class="log-active-forward"><span class="port-forward-listening-dot"></span><div><strong>{forward.localAddress}</strong><small>Local → {forward.remotePort}</small></div><button disabled={Boolean(stoppingPortForwardId)} on:click={() => stopPortForward(forward.id)}>{stoppingPortForwardId === forward.id ? 'Stopping…' : 'Stop'}</button></div>{/each}</section>{/if}
              <div class="log-pod-sidebar-footer">Switch Pods without leaving the log stream. Port forwards remain active until you stop them or quit Kuberniva.</div>
            </aside>
            <div class="log-stream-panel">
              <div class="log-stream-heading"><div><p class="eyebrow">Streaming output</p><h2>{logTarget.pod}</h2><p>{activeCluster} · {logScopeLabel || 'Pod'} · {logTarget.namespace}</p></div><div class="table-actions"><button class="secondary" on:click={() => { closeLogs(); void navigateTo('Workloads') }}>← Back to workloads</button></div></div>
              <div class="log-toolbar"><div><strong>Live logs</strong><small>{openingLogsTarget ? 'Opening the first live stream…' : loadingLogs ? 'Refreshing…' : 'Refreshes every 30 seconds · scroll up to hold your place'}</small></div>{#if logContainers.length > 1}<label>Container <select bind:value={selectedLogContainer} on:change={() => loadLogs(true)}>{#each logContainers as container}<option value={container}>{container}</option>{/each}</select></label>{/if}</div>
              {#if loadingLogs && logLines.length === 0}<div class="log-opening-state"><span><RefreshCw size={22} class="animate-spin" /></span><div><p class="eyebrow">Opening logs</p><h3>{logTarget.pod}</h3><p>Connecting to the Pod and preparing the first live output. You can switch Pods from the left after it opens.</p></div></div>{:else}<pre class="live-log-output" bind:this={logViewport}>{#if logLines.length}{logLines.join('\n')}{:else}The Pod returned no log lines for this container yet.{/if}</pre>{/if}
            </div>
          </section>
        {:else}
          <section class="empty-view"><div class="explore-orbit"><i></i><i></i><b>≡</b></div><h2>No logs selected</h2><p>Open a Pod directly or open a workload to choose one of its Pods. Logs stay in this full workspace tab.</p><button class="primary" on:click={() => navigateTo('Workloads')}>Browse workloads</button></section>
        {/if}
      {/if}
    </div>
  </section>

  {#if deletionTarget}
    <div class="modal-backdrop deletion-backdrop" role="presentation" on:click={cancelDeletion}>
      <div class="deletion-modal" role="dialog" aria-modal="true" aria-label="Confirm deletion" tabindex="-1" on:click|stopPropagation on:keydown|stopPropagation>
        <div class="deletion-modal-mark">!</div>
        {#if deletionStep === 1}
          <p class="eyebrow">Review deletion request</p>
          <h2>{deletionTarget.type === 'resource' ? `Delete ${deletionTarget.resource.kind}?` : 'Remove kubeconfig context?'}</h2>
          <p class="deletion-intro">{deletionTarget.type === 'resource' ? 'This sends a Kubernetes DELETE request only to the cluster and namespace shown below.' : 'This removes the context from Kuberniva only. The original kubeconfig source remains unchanged.'}</p>
          <dl class="deletion-target-summary">
            <div><dt>Target</dt><dd>{deletionTargetName()}</dd></div>
            {#if deletionTarget.type === 'resource'}
              <div><dt>Cluster</dt><dd>{activeCluster}</dd></div><div><dt>Namespace</dt><dd>{deletionTarget.object.namespace || 'cluster-scoped'}</dd></div>
            {:else}
              <div><dt>Source</dt><dd title={deletionTarget.cluster.kubeconfigPath}>{deletionTarget.cluster.kubeconfigPath}</dd></div>
            {/if}
          </dl>
          <div class="deletion-warning">{deletionTarget.type === 'resource' ? 'Deletion cannot be undone. Kubernetes may handle dependents according to the resource’s configured deletion policy.' : 'You can bring this context back later by manually syncing its kubeconfig source. No source file is edited or deleted.'}</div>
          <div class="deletion-actions"><button class="secondary" disabled={deletingResource} on:click={cancelDeletion}>Cancel</button><button class="destructive" disabled={deletingResource} on:click={continueDeletion}>Continue</button></div>
        {:else}
          <p class="eyebrow">Final confirmation</p>
          <h2>Type the name to confirm</h2>
          <p class="deletion-intro">To complete this deletion, type <strong>{deletionTargetName()}</strong> exactly. This prevents an accidental delete from a fast click.</p>
          <label class="deletion-name-input">{deletionTarget.type === 'resource' ? 'Resource name' : 'Context name'}<input bind:value={deletionName} autocomplete="off" spellcheck="false" placeholder={deletionTargetName()} /></label>
          <div class="deletion-actions"><button class="secondary" disabled={deletingResource} on:click={() => (deletionStep = 1)}>Back</button><button class="destructive" disabled={deletingResource || deletionName !== deletionTargetName()} on:click={confirmDeletion}>{deletingResource ? 'Deleting…' : deletionTarget.type === 'resource' ? 'Delete resource' : 'Remove from Kuberniva'}</button></div>
        {/if}
      </div>
    </div>
  {/if}

  {#if commandOpen}
    <div class="modal-backdrop" role="presentation" on:click={() => (commandOpen = false)}>
      <div class="command-modal" role="dialog" aria-modal="true" aria-label="Search Kuberniva resources" tabindex="-1" on:click|stopPropagation on:keydown|stopPropagation={handleCommandKeydown}>
        <div class="command-input"><Search size={16} /><input id="global-command-search" bind:value={commandQuery} aria-label="Search Kubernetes resources" placeholder="Search kind, object, API group, or version…" /><kbd>esc</kbd></div>
        {#if commandQuery.trim()}
          <p class="eyebrow">{globalSearchResults.length ? `${globalSearchResults.length} matches` : 'No matches'}</p>
          <div class="command-results" role="listbox" aria-label="Search results">
            {#each globalSearchResults as result}
              <button class="command-result" type="button" role="option" aria-selected={false} on:click={() => openGlobalSearchResult(result)}><span class="command-result-mark">{result.type === 'object' ? '○' : '◇'}</span><span><strong>{result.title}</strong><small>{result.detail}</small></span><b>{result.type === 'object' ? 'Open' : 'Browse'}</b></button>
            {:else}
              <div class="command-empty"><Search size={18} /><strong>No discovered resource matches “{commandQuery}”</strong><small>Try a kind, object name, API group, or version such as v1beta1.</small></div>
            {/each}
          </div>
        {:else}
          <p class="eyebrow">Search the active cluster</p>
          <div class="command-hint"><Search size={18} /><div><strong>Find any discovered API or loaded object</strong><small>Search is case-insensitive and includes alpha/beta API versions.</small></div></div>
          <button class="command-secondary-action" type="button" on:click={() => { commandOpen = false; void navigateTo('Resources') }}>▤ <span>Browse all discovered resources</span><kbd>↵</kbd></button>
          <button class="command-secondary-action" type="button" on:click={() => { commandOpen = false; kubeconfigOpen = true }}>⌁ <span>Add a kubeconfig</span></button>
        {/if}
      </div>
    </div>
  {/if}

  {#if kubeconfigOpen}
    <div class="modal-backdrop kubeconfig-backdrop" role="presentation" on:click={() => closeKubeconfigModal()}>
      <div class="kubeconfig-modal" role="dialog" aria-modal="true" aria-labelledby="kubeconfig-modal-title" aria-describedby="kubeconfig-modal-description" tabindex="-1" on:click|stopPropagation on:keydown|stopPropagation={handleKubeconfigModalKeydown}>
        <div class="kubeconfig-modal-heading"><div class="modal-kube-mark">⌁</div><div><h2 id="kubeconfig-modal-title">Add kubeconfig</h2><p id="kubeconfig-modal-description">Add one source without replacing any clusters already in your workspace.</p></div></div>

        <div class="kubeconfig-source-modes" role="tablist" aria-label="Kubeconfig source type">
          <button id="kubeconfig-source-tab-file" type="button" class:source-mode-active={kubeconfigInputMode === 'file'} role="tab" aria-selected={kubeconfigInputMode === 'file'} aria-controls="kubeconfig-source-panel" tabindex={kubeconfigInputMode === 'file' ? 0 : -1} on:click={() => (kubeconfigInputMode = 'file')} on:keydown={(event) => handleKubeconfigSourceTabKeydown(event, 'file')}><span>▤</span><strong>File</strong><small>One kubeconfig</small></button>
          <button id="kubeconfig-source-tab-folder" type="button" class:source-mode-active={kubeconfigInputMode === 'folder'} role="tab" aria-selected={kubeconfigInputMode === 'folder'} aria-controls="kubeconfig-source-panel" tabindex={kubeconfigInputMode === 'folder' ? 0 : -1} on:click={() => (kubeconfigInputMode = 'folder')} on:keydown={(event) => handleKubeconfigSourceTabKeydown(event, 'folder')}><span>▱</span><strong>Folder</strong><small>Many files</small></button>
          <button id="kubeconfig-source-tab-paste" type="button" class:source-mode-active={kubeconfigInputMode === 'paste'} role="tab" aria-selected={kubeconfigInputMode === 'paste'} aria-controls="kubeconfig-source-panel" tabindex={kubeconfigInputMode === 'paste' ? 0 : -1} on:click={() => (kubeconfigInputMode = 'paste')} on:keydown={(event) => handleKubeconfigSourceTabKeydown(event, 'paste')}><span>⌘</span><strong>Paste YAML</strong><small>From clipboard</small></button>
        </div>

        {#if kubeconfigInputMode === 'paste'}
          <div id="kubeconfig-source-panel" class="kubeconfig-source-panel paste-source-panel" role="tabpanel" aria-labelledby="kubeconfig-source-tab-paste">
            <div class="source-panel-heading"><div><strong>Paste kubeconfig YAML</strong><small>Contexts are saved locally and become reusable after restarting Kuberniva.</small></div><b>YAML</b></div>
            <textarea aria-label="Kubeconfig YAML" bind:value={pastedKubeconfig} spellcheck="false" placeholder={'apiVersion: v1\nkind: Config\nclusters:\n  - cluster: …\ncontexts:\n  - context: …'}></textarea>
            <p class="paste-security-note"><span>✓</span>Your existing sources remain untouched. The pasted configuration is stored only in Kuberniva's local app data.</p>
          </div>
        {:else}
          <div id="kubeconfig-source-panel" class="kubeconfig-source-panel" role="tabpanel" aria-labelledby={`kubeconfig-source-tab-${kubeconfigInputMode}`}>
            <div class="source-panel-heading"><div><strong>{kubeconfigInputMode === 'folder' ? 'Choose a kubeconfig folder' : 'Choose a kubeconfig file'}</strong><small>{kubeconfigInputMode === 'folder' ? 'Every readable kubeconfig in the folder is discovered.' : 'Paths can be absolute, relative to home, or begin with ~/.'}</small></div></div>
            <label>{kubeconfigInputMode === 'folder' ? 'Folder path' : 'File path'}<input bind:value={kubeconfigPath} placeholder={kubeconfigInputMode === 'folder' ? '~/clusters' : 'Default: ~/.kube/config'} /></label>
            <div class="source-picker-actions"><button class="secondary" on:click={() => chooseKubeconfig(kubeconfigInputMode === 'folder')}>{kubeconfigInputMode === 'folder' ? 'Choose folder…' : 'Choose file…'}</button></div>
          </div>
        {/if}

        <div class="kubeconfig-modal-footer"><button class="secondary" disabled={loadingCatalog} on:click={() => closeKubeconfigModal()}>Cancel</button><button class="primary" disabled={loadingCatalog || (kubeconfigInputMode === 'paste' && !pastedKubeconfig.trim())} on:click={() => kubeconfigInputMode === 'paste' ? importPastedKubeconfig() : connectKubeconfig()}>{loadingCatalog ? (kubeconfigInputMode === 'paste' ? 'Importing…' : 'Discovering…') : (kubeconfigInputMode === 'paste' ? 'Import contexts' : 'Add & discover')}</button></div>
      </div>
    </div>
  {/if}

  {#if selectedResource && activeView !== 'Resources'}
      <aside class="resource-detail-tab" aria-label={`${selectedResource.kind} objects`}>
      <div class="resource-drawer">
        <div class="drawer-heading"><div><span class:custom={selectedResource.custom}>{selectedResource.custom ? '◇' : '○'}</span><div><h2>{selectedResource.kind}</h2><p>{selectedResource.apiVersion} · {selectedResource.namespaced ? namespace : 'cluster scope'}</p></div></div><button aria-label="Close resource drawer" on:click={() => (selectedResource = null)}>×</button></div>
        {#if loadingObjects}
          <div class="drawer-state"><i></i>Listing {selectedResource.plural}…</div>
        {:else if loadingRelatedPods}
          <div class="drawer-state"><i></i>Finding Pods for this {selectedResource.kind}…</div>
        {:else if relatedPods !== null}
          <div class="related-pods"><div class="related-heading"><button on:click={() => { relatedPods = null; relatedObject = null }}>← Back</button><span>Pods selected by this {selectedResource.kind}</span>{#if relatedObject}<button class="inline-yaml" on:click={() => selectedResource && relatedObject && openYamlEditor(selectedResource, relatedObject)}>View YAML</button>{/if}</div>{#if relatedPods.length === 0}<div class="drawer-state">No matching Pods found.</div>{:else}<div class="object-list">{#each relatedPods as pod}<button disabled={Boolean(openingLogsTarget)} aria-busy={isOpeningLogs('Pod', pod)} on:click={() => openPodLogs(pod, relatedPods || [], `${selectedResource?.kind || 'Workload'} · ${relatedObject?.name || 'workload'}`)}><span class="object-icon">□</span><div><strong>{pod.name}</strong><small>{pod.namespace || 'namespace unavailable'}</small></div><span>{isOpeningLogs('Pod', pod) ? 'Opening logs…' : 'Logs →'}</span></button>{/each}</div>{/if}</div>
        {:else if resourceObjects.length === 0}
          <div class="drawer-state">No {selectedResource.plural} found in this scope.</div>
        {:else}
          <div class="object-list">{#each resourceObjects as object}<button aria-busy={selectedResource.kind === 'Pod' && isOpeningLogs('Pod', object)} on:click={() => openObject(selectedResource!, object)}><span class="object-icon">□</span><div><strong>{object.name}</strong><small>{object.namespace || 'cluster scoped'} {object.createdAt ? `· ${object.createdAt}` : ''}</small></div><span>{selectedResource.kind === 'Pod' ? (isOpeningLogs('Pod', object) ? 'Opening logs…' : 'Logs →') : selectedResource.category === 'Workloads' ? 'Pods →' : 'Details →'}</span></button>{/each}</div>
        {/if}
        <div class="drawer-footer"><span>Demand-loaded · no background fan-out</span><span>Open an object for its live details.</span></div>
      </div>
      </aside>
  {/if}

  {#if yamlResource && yamlObject}
    <aside class="resource-yaml-tab" aria-label={`${yamlObject.name} YAML`}>
      <div class="yaml-surface">
        <div class="drawer-heading"><div><span>{yamlMode === 'edit' ? '✎' : '⌘'}</span><div><h2>{yamlObject.name}</h2><p>{yamlResource.kind} · {yamlObject.namespace || 'cluster scoped'} · {yamlMode === 'edit' ? 'editing YAML' : 'live YAML'}</p></div></div><button aria-label="Close YAML" on:click={closeYamlEditor}>×</button></div>
        {#if loadingYaml}
          <div class="drawer-state"><i></i>Loading live YAML…</div>
        {:else if yamlMode === 'edit'}
          <textarea class="yaml-editor" bind:value={yamlText} spellcheck="false" aria-label="Resource YAML"></textarea>
        {:else}
          <pre class="yaml-output">{yamlText}</pre>
        {/if}
        <div class="yaml-footer"><span>{yamlMode === 'edit' ? 'Saving replaces this resource using its current resource version.' : 'Loaded from the Kubernetes API.'}</span><div>{#if yamlMode === 'edit'}<button class="secondary" disabled={savingYaml} on:click={() => { yamlText = yamlOriginal; yamlMode = 'view' }}>Discard</button><button class="primary" disabled={savingYaml || yamlText === yamlOriginal} on:click={saveYamlEditor}>{savingYaml ? 'Saving…' : 'Save YAML'}</button>{:else}<button class="primary" on:click={() => (yamlMode = 'edit')}>Edit YAML</button>{/if}</div></div>
      </div>
    </aside>
  {/if}

  {#if editorResource && editorObject && activeView !== 'Resources' && activeView !== 'Workloads'}
    <aside class="resource-editor-tab" aria-label={`${editorResource.kind} editor`}>
      <div class="resource-editor">
        <div class="drawer-heading"><div><span class:custom={editorResource.custom}>{editorResource.kind === 'Secret' ? '◈' : editorResource.kind === 'ConfigMap' ? '◇' : '⌁'}</span><div><h2>{editorObject.name}</h2><p>{editorResource.kind} · {editorObject.namespace || 'cluster scoped'}</p></div></div><button aria-label="Close editor" on:click={() => closeEditor()}>×</button></div>
        {#if loadingEditor}
          <div class="drawer-state"><i></i>Loading live resource data…</div>
        {:else}
          {#if editorCertificate}
            <section class:expired={editorCertificate.expired} class="certificate-card"><div><span>⌁</span><div><strong>{editorCertificate.expired ? 'Certificate expired' : 'TLS certificate'}</strong><p>Expires {editorCertificate.expiresAt}</p></div></div><b>{editorCertificate.expired ? `${Math.abs(editorCertificate.daysRemaining)} days ago` : `${editorCertificate.daysRemaining} days remaining`}</b></section>
          {/if}
          {#if editorResource.kind === 'Secret' || editorResource.kind === 'ConfigMap'}
            <section class="configuration-inspector focus-canvas-editor">
              <div class="configuration-data-heading"><div><span>{editorResource.kind === 'Secret' ? '◈' : '◇'}</span><div><strong>{editorResource.kind === 'Secret' ? 'Secret data' : 'ConfigMap data'}</strong><small>{editorResource.kind === 'Secret' ? (revealSecret ? 'Decoded values are visible locally' : 'Values are base64 encoded') : 'Plain-text values'}</small></div></div><b>{editorEntries.length} {editorEntries.length === 1 ? 'entry' : 'entries'}</b></div>
              <div class="editor-toolbar focus-canvas-toolbar"><div><strong>{editorResource.kind === 'Secret' ? 'Protecting sensitive values' : 'Focus editor'}</strong><small>{editorResource.kind === 'Secret' ? 'Choose a key on the left, then reveal or edit its value.' : 'Choose a key on the left to give its value room to breathe.'}</small></div>{#if editorResource.kind === 'Secret'}<button class="reveal-button" on:click={() => (revealSecret = !revealSecret)}>{revealSecret ? '◉ Hide decoded' : '◌ Reveal decoded'}</button>{/if}</div>
              <div class="focus-canvas-shell">
                {#if editorEntries.length === 0}
                  <div class="drawer-state focus-canvas-empty">This {editorResource!.kind} has no data entries.<button class="focus-canvas-empty-action" on:click={addEditorEntry}>＋ Add key</button></div>
                {:else}
                  <nav class="focus-canvas-rail" aria-label={`${editorResource.kind} keys`}>
                    <div class="focus-canvas-rail-heading"><span>Keys</span><b>{editorEntries.length}</b></div>
                    <div class="focus-canvas-key-list">{#each editorEntries as entry, index}<button type="button" class:focus-canvas-key-active={index === focusedEditorEntry} class="focus-canvas-key" on:click={() => (focusedEditorEntry = index)}><i></i><span>{entry.key || 'Unnamed key'}</span></button>{/each}</div>
                    <div class="focus-canvas-rail-actions"><button type="button" class="focus-canvas-remove focus-canvas-remove-rail" on:click={() => removeEditorEntry(focusedEditorEntry)}>Remove selected</button><button type="button" class="focus-canvas-add" on:click={addEditorEntry}><span>＋</span>Add key</button></div>
                  </nav>
                  {#if focusedEditorEntryData}
                    <div class="focus-canvas-main">
                      <label class="focus-canvas-field focus-canvas-value-field"><span>Value</span><textarea use:autoSizeTextarea={editorResource.kind === 'Secret' && revealSecret ? decodeSecret(focusedEditorEntryData.value) : focusedEditorEntryData.value} value={editorResource.kind === 'Secret' && revealSecret ? decodeSecret(focusedEditorEntryData.value) : focusedEditorEntryData.value} on:input={(event) => updateEditorEntry(focusedEditorEntry, event.currentTarget.value)} spellcheck="false"></textarea></label>
                      <div class="focus-canvas-hint"><span>✓</span>{editorResource.kind === 'Secret' ? (revealSecret ? 'Decoded locally; saving writes base64 back to Kubernetes.' : 'Values remain encoded until you reveal them.') : 'Changes are staged locally until you save.'}</div>
                    </div>
                  {/if}
                {/if}
              </div>
            </section>
          {:else if !editorCertificate}
            <div class="drawer-state">This resource has no editable data view yet.</div>
          {/if}
        {/if}
        <div class="drawer-footer"><span>{editorResource.kind === 'Secret' ? 'Decoded values are never stored by Kuberniva.' : 'Saving uses the cluster resource version.'}</span><div class="editor-footer-actions"><button class="secondary" disabled={loadingEditor} on:click={() => openYamlEditor(editorResource!, editorObject!)}>View YAML</button>{#if editorResource.kind === 'Secret' || editorResource.kind === 'ConfigMap'}<button class="primary" disabled={loadingEditor || savingEditor} on:click={saveEditor}>{savingEditor ? 'Saving…' : 'Save changes'}</button>{:else}<button class="secondary" on:click={() => closeEditor()}>Close</button>{/if}</div></div>
      </div>
    </aside>
  {/if}

  {#if toast}<div class="toast"><span>✓</span>{toast}</div>{/if}
</main>
