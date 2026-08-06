<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { Activity, Bell, Boxes, ChevronRight, CircleDot, Command, Container, Database, FileSearch, LayoutDashboard, Menu, RefreshCw, Search, ScrollText, Settings2, SlidersHorizontal, Star, Workflow } from '@lucide/svelte';

  type View = 'Clusters' | 'Favorites' | 'Overview' | 'Events' | 'Resources' | 'Workloads' | 'Explore' | 'Logs' | 'Settings';
  type ResourceCategory = 'Workloads' | 'Configuration' | 'Access Control' | 'Network' | 'Storage' | 'Cluster' | 'Custom Resources';
  type ResourceDescriptor = { group: string; version: string; apiVersion: string; kind: string; plural: string; namespaced: boolean; category: ResourceCategory; custom: boolean; crd: boolean };
  type ClusterCatalog = { context: string; namespaces: string[]; resources: ResourceDescriptor[] };
  type ResourceObject = { name: string; namespace?: string; createdAt?: string };
  type PodPort = { container: string; name?: string; port: number; protocol: string };
  type PodLogResponse = { lines: string[]; containers: string[]; selectedContainer?: string; ports: PodPort[] };
  type PodRuntime = { containers: string[]; ports: PodPort[] };
  type PodExecResponse = { stdout: string; stderr: string };
  type PortForward = { id: string; localAddress: string; localPort: number; remotePort: number; namespace: string; pod: string };
  type LogTarget = { pod: string; namespace: string };
  type CertificateInfo = { expiresAt: string; daysRemaining: number; expired: boolean };
  type ResourceDetail = { manifest: Record<string, unknown>; yaml: string; certificate?: CertificateInfo };
  type EditorEntry = { key: string; value: string };
  type NodeProperty = { key: string; value: string };
  type NodeAddress = { type: string; address: string };
  type NodeCondition = { type: string; status: string; reason?: string; message?: string; lastHeartbeatTime?: string; lastTransitionTime?: string };
  type NodeTaint = { key: string; value?: string; effect: string; timeAdded?: string };
  type NodeOverview = { name: string; ready: boolean; roles: string[]; labels: NodeProperty[]; annotations: NodeProperty[]; addresses: NodeAddress[]; conditions: NodeCondition[]; taints: NodeTaint[]; architecture?: string; operatingSystem?: string; osImage?: string; kernelVersion?: string; kubeletVersion?: string; containerRuntimeVersion?: string; podCidrs: string[]; providerId?: string; unschedulable: boolean; uid?: string; creationTimestamp?: string; capacity: NodeProperty[]; allocatable: NodeProperty[]; cpuCapacity?: string; memoryCapacity?: string; cpuUsage?: string; memoryUsage?: string; cpuUsagePercent?: number; memoryUsagePercent?: number };
  type ClusterOverview = { nodes: NodeOverview[]; metricsAvailable: boolean; observedAt: string };
  type ClusterEvent = { name: string; namespace?: string; eventType: string; reason?: string; message?: string; involvedKind?: string; involvedName?: string; action?: string; count?: number; source?: string; firstObserved?: string; lastObserved?: string };
  type KubeContext = { name: string; cluster: string; namespace: string; authMethod: string; current: boolean; sourcePath?: string };
  type KubeconfigSummary = { contexts: KubeContext[]; currentContext?: string };
  type Cluster = { id: string; name: string; provider: string; status: string; tone: string; authMethod?: string; namespace?: string; kubeconfigPath?: string; sourceId?: string };
  type ClusterSession = { namespace: string; selectedCategory: ResourceCategory | 'All resources'; resourceSearch: string; workloadResource: ResourceDescriptor | null; workloadObjects: ResourceObject[]; workloadSearch: string; clusterOverview: ClusterOverview | null };
  type PersistedWorkspace = { version: 5; sourceConfigured: boolean; kubeconfigPath: string; kubeconfigPaths: string[]; clusters: Cluster[]; sidebarWidth?: number; sidebarHidden?: boolean; clusterNamespaces?: Record<string, string>; favoriteClusterIds?: string[]; favoriteClusterNames?: Record<string, string> };
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
  let kubeconfigOpen = false;
  let kubeconfigPath = '';
  let kubeconfigSources: string[] = [];
  let sourceConfigured = false;
  let restoringWorkspace = true;
  let resourceSearch = '';
  let selectedCategory: ResourceCategory | 'All resources' = 'All resources';
  let loadingCatalog = false;
  let selectedResource: ResourceDescriptor | null = null;
  let resourceObjects: ResourceObject[] = [];
  let loadingObjects = false;
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
  let openingWorkloadLogs = false;
  let logViewport: HTMLPreElement;
  let logRefreshTimer: ReturnType<typeof window.setInterval> | undefined;
  let logRequestGeneration = 0;
  let portForwardOpen = false;
  let portForwarding = false;
  let portForwardRemotePort = '';
  let portForwardLocalPort = '';
  let portForwardTrayOpen = false;
  let portForwards: PortForward[] = [];
  let workloadDetailMode: 'overview' | 'terminal' = 'overview';
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
  let editorResource: ResourceDescriptor | null = null;
  let editorObject: ResourceObject | null = null;
  let editorManifest: Record<string, unknown> | null = null;
  let editorEntries: EditorEntry[] = [];
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
  let overviewRefreshTimer: ReturnType<typeof window.setInterval> | undefined;
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
  const catalogCache = new Map<string, ClusterCatalog>();
  const clusterSessionCache = new Map<string, ClusterSession>();
  const resourceObjectCache = new Map<string, ResourceObject[]>();
  const workspaceStorageKey = 'kuberniva.workspace.v1';

  const resourceCategories: ResourceCategory[] = ['Workloads', 'Configuration', 'Access Control', 'Network', 'Storage', 'Cluster', 'Custom Resources'];
  let clusters: Cluster[] = [];
  let catalog: ClusterCatalog = { context: '', namespaces: [], resources: [] };

  $: favoriteClusters = favoriteClusterIds
    .map((id) => clusters.find((cluster) => cluster.id === id))
    .filter((cluster): cluster is Cluster => Boolean(cluster))
    .slice(0, 10);
  $: favoriteContextCluster = favoriteContextMenu ? clusters.find((cluster) => cluster.id === favoriteContextMenu?.clusterId) : null;
  $: selectedNode = clusterOverview?.nodes.find((node) => node.name === selectedNodeName) || clusterOverview?.nodes[0] || null;
  $: visibleClusterEvents = clusterEvents.filter((event) =>
    (eventTypeFilter === 'All' || event.eventType === eventTypeFilter) &&
    `${event.reason || ''} ${event.message || ''} ${event.involvedKind || ''} ${event.involvedName || ''} ${event.namespace || ''}`.toLowerCase().includes(eventSearch.toLowerCase()),
  );
  $: visibleResources = catalog.resources.filter((resource) =>
    (selectedCategory === 'All resources' || resource.category === selectedCategory) &&
    `${resource.kind} ${resource.plural} ${resource.group}`.toLowerCase().includes(resourceSearch.toLowerCase()),
  ).sort((left, right) => left.kind.localeCompare(right.kind));
  $: categoryCounts = Object.fromEntries(resourceCategories.map((category) => [category, catalog.resources.filter((resource) => resource.category === category).length]));
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

  function notify(message: string) {
    toast = message;
    window.setTimeout(() => (toast = ''), 2600);
  }

  function persistWorkspace() {
    try {
      if (activeClusterId) persistedClusterNamespaces = { ...persistedClusterNamespaces, [activeClusterId]: namespace };
      const workspace: PersistedWorkspace = {
        version: 5,
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
      };
      if ((parsed.version !== 1 && parsed.version !== 2 && parsed.version !== 3 && parsed.version !== 4 && parsed.version !== 5) || typeof parsed.sourceConfigured !== 'boolean' || typeof parsed.kubeconfigPath !== 'string') return null;
      const kubeconfigPaths = (parsed.version === 2 || parsed.version === 3 || parsed.version === 4 || parsed.version === 5) && Array.isArray(parsed.kubeconfigPaths)
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
        version: 5,
        sourceConfigured: parsed.sourceConfigured,
        kubeconfigPath: parsed.kubeconfigPath,
        kubeconfigPaths: kubeconfigPaths.length ? kubeconfigPaths : [''],
        clusters: cachedClusters,
        sidebarWidth: typeof parsed.sidebarWidth === 'number' ? parsed.sidebarWidth : undefined,
        sidebarHidden: typeof parsed.sidebarHidden === 'boolean' ? parsed.sidebarHidden : undefined,
        clusterNamespaces,
        favoriteClusterIds,
        favoriteClusterNames,
      };
      window.localStorage.setItem(workspaceStorageKey, JSON.stringify(workspace));
      return workspace;
    } catch {
      return null;
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
    selectedCategory = session?.selectedCategory || 'All resources';
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

  function startOverviewRefresh() {
    stopOverviewRefresh();
    if (!activeClusterId || activeView !== 'Overview') return;
    overviewRefreshTimer = window.setInterval(() => void loadClusterOverview(), 60_000);
  }

  async function loadClusterOverview() {
    if (!activeClusterId || loadingOverview) return;
    const overviewClusterId = activeClusterId;
    loadingOverview = true;
    overviewError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const response = await invoke<ClusterOverview>('read_cluster_overview', {
        kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
        context: activeCluster,
      });
      if (overviewClusterId === activeClusterId) {
        clusterOverview = response;
        selectedNodeName = response.nodes.some((node) => node.name === selectedNodeName) ? selectedNodeName : response.nodes[0]?.name || '';
      }
    } catch (error) {
      if (overviewClusterId === activeClusterId) overviewError = String(error);
    } finally {
      if (overviewClusterId === activeClusterId) loadingOverview = false;
    }
  }

  async function loadClusterEvents(force = false) {
    if (!activeClusterId || loadingEvents) return;
    if (!force && clusterEvents.length && eventsObservedAt && activeClusterId === eventsClusterId) return;
    const requestClusterId = activeClusterId;
    loadingEvents = true;
    eventsError = '';
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const response = await invoke<ClusterEvent[]>('read_cluster_events', {
        kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
        context: activeCluster,
      });
      if (requestClusterId === activeClusterId) {
        clusterEvents = response;
        eventsClusterId = requestClusterId;
        eventsObservedAt = new Date().toISOString();
      }
    } catch (error) {
      if (requestClusterId === activeClusterId) eventsError = String(error);
    } finally {
      if (requestClusterId === activeClusterId) loadingEvents = false;
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
    }
    rememberActiveClusterSession();
    persistWorkspace();
  }

  function selectResourceCategory(category: ResourceCategory | 'All resources') {
    selectedCategory = category;
    resourceSearch = '';
    clusterPickerOpen = false;
    selectedResource = null;
    resourceObjects = [];
    closeEditor();
    closeYamlEditor();
    const firstResource = catalog.resources
      .filter((resource) => category === 'All resources' || resource.category === category)
      .sort((left, right) => left.kind.localeCompare(right.kind))[0];
    if (firstResource) void openResource(firstResource);
  }

  async function navigateTo(view: View) {
    // Views are self-contained workspaces. Never leave an inspector, YAML tab,
    // stream, or flyout visually attached when the user changes context.
    clusterPickerOpen = false;
    namespaceOpen = false;
    commandOpen = false;
    if (view !== 'Logs') closeLogs();
    closeEditor();
    closeYamlEditor();
    selectedResource = null;
    relatedPods = null;
    relatedObject = null;
    activeView = view;
    if (view === 'Overview' && activeClusterId) {
      void loadClusterOverview();
      startOverviewRefresh();
      return;
    }
    if (view === 'Events' && activeClusterId) {
      void loadClusterEvents();
      return;
    }
    if (view !== 'Overview') stopOverviewRefresh();
    if (view !== 'Workloads' || !activeClusterId || loadingWorkloads) return;
    const preferredResource = workloadResource
      || catalog.resources.find((resource) => resource.kind === 'Deployment')
      || catalog.resources.find((resource) => resource.category === 'Workloads');
    if (!preferredResource) return;
    await loadWorkloadResource(preferredResource);
  }

  async function loadWorkloadResource(resource: ResourceDescriptor) {
    if (loadingWorkloads) return;
    workloadResource = resource;
    const cacheKey = resourceObjectCacheKey(activeClusterId, resource, namespace);
    const cachedObjects = resourceObjectCache.get(cacheKey);
    if (cachedObjects) {
      workloadObjects = cachedObjects;
      return;
    }
    workloadObjects = [];
    loadingWorkloads = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      workloadObjects = await invoke<ResourceObject[]>('list_resource_objects', {
        request: {
          kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
          context: activeCluster,
          group: resource.group,
          version: resource.version,
          kind: resource.kind,
          plural: resource.plural,
          namespaced: resource.namespaced,
          namespace,
        },
      });
      resourceObjectCache.set(cacheKey, workloadObjects);
    } catch (error) {
      notify(`Could not load ${resource.kind}s: ${String(error)}`);
    } finally {
      loadingWorkloads = false;
    }
  }

  function selectWorkloadResource(resource: ResourceDescriptor) {
    workloadSearch = '';
    void loadWorkloadResource(resource);
  }

  function objectNamespace(object: ResourceObject) {
    return object.namespace || (namespace === 'all namespaces' ? '' : namespace);
  }

  function resourceKey(resource: ResourceDescriptor) {
    return `${resource.group}\u0000${resource.version}\u0000${resource.plural}`;
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

  function networkHosts(manifest: Record<string, unknown> | null) {
    const hosts = new Set<string>();
    const resource = asRecord(manifest);
    const spec = asRecord(resource.spec);
    const status = asRecord(resource.status);
    const add = (value: unknown) => { const host = asString(value); if (host) hosts.add(host); };
    add(spec.externalName);
    add(spec.clusterIP);
    asArray(spec.rules).forEach((rule) => add(asRecord(rule).host));
    asArray(spec.tls).forEach((entry) => asArray(asRecord(entry).hosts).forEach(add));
    asArray(spec.listeners).forEach((listener) => add(asRecord(listener).hostname));
    asArray(asRecord(status.loadBalancer).ingress).forEach((entry) => { add(asRecord(entry).hostname); add(asRecord(entry).ip); });
    return [...hosts];
  }

  function networkPorts(manifest: Record<string, unknown> | null) {
    const spec = asRecord(asRecord(manifest).spec);
    const ports = asArray(spec.ports).length ? asArray(spec.ports) : asArray(spec.listeners);
    return ports.map((entry) => {
      const port = asRecord(entry);
      const number = port.port ?? port.targetPort ?? port.containerPort;
      const target = port.targetPort && port.port !== port.targetPort ? ` → ${port.targetPort}` : '';
      return `${port.name ? `${port.name} · ` : ''}${number ?? '—'}${target}${port.protocol ? `/${port.protocol}` : ''}`;
    });
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
    if (openingWorkloadLogs) return;
    openingWorkloadLogs = true;
    await tick();
    try {
      const pods = resource.kind === 'Pod' ? [object] : await listWorkloadPods(resource, object);
      if (!pods.length) {
        notify(`No live Pods match ${resource.kind} ${object.name}`);
        return;
      }
      const podNamespace = objectNamespace(pods[0]);
      const podsInNamespace = podNamespace ? await namespacePodIndex(podNamespace, pods) : pods;
      await openPodLogs(pods[0], podsInNamespace, `${resource.kind} · ${object.name}`);
    } catch (error) {
      notify(`Could not open logs for ${object.name}: ${String(error)}`);
    } finally {
      openingWorkloadLogs = false;
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

  function closeEditor() {
    if (savingEditor) return;
    resetWorkloadTerminal();
    editorResource = null;
    editorObject = null;
    editorManifest = null;
    editorEntries = [];
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

  function closeLogs() {
    if (logRefreshTimer) window.clearInterval(logRefreshTimer);
    logRefreshTimer = undefined;
    logRequestGeneration += 1;
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

  function openPortForwardForm() {
    if (!logTarget) return;
    const suggested = suggestedForwardPorts[0];
    if (!portForwardRemotePort && suggested) portForwardRemotePort = String(suggested);
    if (!portForwardLocalPort && suggested) portForwardLocalPort = String(suggested);
    portForwardOpen = !portForwardOpen;
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
      portForwards = [...portForwards, forward];
      portForwardOpen = false;
      portForwardTrayOpen = true;
      notify(`Forwarding ${logTarget.pod}:${remotePort} on ${forward.localAddress}`);
    } catch (error) {
      notify(`Could not start port forward: ${String(error)}`);
    } finally {
      portForwarding = false;
    }
  }

  async function stopPortForward(id: string) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('stop_port_forward', { request: { id } });
      portForwards = portForwards.filter((forward) => forward.id !== id);
      if (!portForwards.length) portForwardTrayOpen = false;
      notify('Port forward stopped');
    } catch (error) {
      notify(`Could not stop port forward: ${String(error)}`);
    }
  }

  async function openPodLogs(object: ResourceObject, candidates: ResourceObject[] = [], scopeLabel = 'Pod') {
    closeLogs();
    stopOverviewRefresh();
    // Logs are deliberately a focused workspace: no previous object editor,
    // YAML surface, or resource list should travel with the stream.
    closeEditor();
    closeYamlEditor();
    logPods = availableLogPods(object, candidates);
    logScopeLabel = scopeLabel;
    selectedResource = null;
    relatedPods = null;
    relatedObject = null;
    clusterPickerOpen = false;
    namespaceOpen = false;
    activeView = 'Logs';
    await selectLogPod(object);
    logRefreshTimer = window.setInterval(() => loadLogs(), 30_000);
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
  });

  onMount(() => {
    void restoreWorkspace();
    const closeFloatingMenus = (event: PointerEvent) => {
      const target = event.target instanceof Element ? event.target : null;
      if (!target?.closest('.cluster-selector')) clusterPickerOpen = false;
      if (!target?.closest('.namespace-picker, .workload-namespace-picker')) namespaceOpen = false;
      if (!target?.closest('.favorite-context-menu, .favorite-shortcut, .favorite-card-open')) favoriteContextMenu = null;
    };
    window.addEventListener('pointerdown', closeFloatingMenus);
    return () => window.removeEventListener('pointerdown', closeFloatingMenus);
  });

  async function loadCluster(cluster: Cluster, force = false) {
    // A Pod name belongs to exactly one cluster context. Never carry its stream across a switch.
    closeLogs();
    closeEditor();
    closeYamlEditor();
    clusterPickerOpen = false;
    namespaceOpen = false;
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
    if (!cluster.kubeconfigPath) {
      notify('Choose this kubeconfig as a single source file before removing a context');
      return;
    }
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
    if (!cluster.kubeconfigPath) return;
    deletingResource = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('forget_kubeconfig_context', {
        request: { kubeconfigPath: cluster.kubeconfigPath, context: cluster.name },
      });
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
      notify(`Removed ${cluster.name} from its kubeconfig source.`);
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
    const cluster = clusters.find((candidate) => candidate.id === activeClusterId);
    if (cluster) {
      rememberActiveClusterSession();
      clearClusterObjectCache(cluster.id);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('invalidate_cluster_client', { kubeconfigPath: cluster.kubeconfigPath || kubeconfigPath || null, context: cluster.name });
      } catch {
        // Older app binaries do not expose this optional performance reset command.
      }
      await loadCluster(cluster, true);
    }
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
        kubeconfigOpen = false;
        const addedCount = clusters.length - clusterCountBeforeAdd;
        notify(`${addedCount > 0 ? `${addedCount} new context${addedCount === 1 ? '' : 's'} added` : 'Source refreshed'} · ${clusters.length} context${clusters.length === 1 ? '' : 's'} tracked locally.`);
      }
    } catch (error) {
      notify(`Could not connect: ${String(error)}`);
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

  async function openResource(resource: ResourceDescriptor) {
    closeEditor();
    closeYamlEditor();
    selectedResource = resource;
    relatedPods = null;
    relatedObject = null;
    const cacheKey = resourceObjectCacheKey(activeClusterId, resource, namespace);
    const cachedObjects = resourceObjectCache.get(cacheKey);
    if (cachedObjects) {
      resourceObjects = cachedObjects;
      loadingObjects = false;
      return;
    }
    loadingObjects = true;
    resourceObjects = [];
    try {
      if (!('__TAURI_INTERNALS__' in window)) {
        throw new Error('Resource listing is available in the Kuberniva desktop app');
      } else {
        const { invoke } = await import('@tauri-apps/api/core');
        resourceObjects = await invoke<ResourceObject[]>('list_resource_objects', {
          request: {
            kubeconfigPath: activeKubeconfigPath || kubeconfigPath || null,
            context: activeCluster,
            group: resource.group,
            version: resource.version,
            kind: resource.kind,
            plural: resource.plural,
            namespaced: resource.namespaced,
            namespace,
          },
        });
        resourceObjectCache.set(cacheKey, resourceObjects);
      }
    } catch (error) {
      notify(`Could not list ${resource.kind}: ${String(error)}`);
    } finally {
      loadingObjects = false;
    }
  }
</script>

<svelte:head>
  <title>Kuberniva — Kubernetes, in focus</title>
  <meta name="description" content="A calm, fast Kubernetes control surface." />
</svelte:head>

<main class:sidebar-collapsed={sidebarHidden}>
  <aside class:sidebar-hidden={sidebarHidden} class="sidebar" style:width={`${sidebarWidth}px`} style:flex-basis={`${sidebarWidth}px`}>
    <div class="brand">
      <img class="brand-mark" src="/kuberniva-mark.png" alt="" />
      <span class="brand-wordmark"><strong>Kube</strong><span>rniva</span></span>
    </div>

    <nav aria-label="Cluster navigation">
      <p class="eyebrow">Cluster workspace</p>
      {#each ['Overview', 'Events', 'Workloads', 'Resources'] as view}
        <button class:active={activeView === view} class="nav-item" on:click={() => navigateTo(view as View)}>
          <span class="nav-icon">
            {#if view === 'Overview'}<LayoutDashboard size={17} strokeWidth={1.8} />{:else if view === 'Events'}<ScrollText size={17} strokeWidth={1.8} />{:else if view === 'Resources'}<Database size={17} strokeWidth={1.8} />{:else}<Workflow size={17} strokeWidth={1.8} />{/if}
          </span>
          {view}
          {#if view === 'Events' && clusterEvents.length}<span class="count">{clusterEvents.length}</span>{/if}
          {#if view === 'Workloads' && workloadObjects.length}<span class="count">{workloadObjects.length}</span>{/if}
          {#if view === 'Resources'}<span class="count">{activeClusterId ? catalog.resources.length : 0}</span>{/if}
        </button>
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
      <div class="toolbar-leading"><button class="sidebar-toggle" aria-label={sidebarHidden ? 'Show sidebar' : 'Hide sidebar'} title={sidebarHidden ? 'Show sidebar' : 'Hide sidebar'} on:click={toggleSidebar}><Menu size={17} strokeWidth={2} /></button><div class="cluster-selector topbar-cluster-selector"><button class:cluster-picker-open={clusterPickerOpen} class="cluster-picker" aria-expanded={clusterPickerOpen} on:click={() => (clusterPickerOpen = !clusterPickerOpen)}><span class="cluster-dot"></span><span class="cluster-picker-copy"><small>Active cluster</small><span class="cluster-picker-name">{activeCluster}</span></span><span class="chevron">⌄</span></button>{#if clusterPickerOpen}<div class="cluster-selector-menu" role="menu" aria-label="Select cluster"><div class="cluster-selector-heading"><span>Clusters</span><b>{clusters.length}</b></div>{#if clusters.length}<div class="cluster-selector-items">{#each clusters as cluster}<button class:chosen={cluster.id === activeClusterId} title={`${cluster.authMethod || cluster.provider} · ${cluster.status}`} on:click={() => selectCluster(cluster.id)}><span class="status-dot {cluster.tone}"></span><span><strong>{cluster.name}</strong><small>{cluster.authMethod || cluster.provider}</small></span>{#if cluster.id === activeClusterId}<i>✓</i>{/if}</button>{/each}</div>{:else}<p class="cluster-selector-empty">No kubeconfig contexts added yet.</p>{/if}<button class="cluster-selector-manage" on:click={() => { clusterPickerOpen = false; void navigateTo('Clusters') }}>Manage clusters <span>→</span></button></div>{/if}</div><div class="breadcrumbs"><span>Workspace</span><span>/</span><strong>{activeCluster}</strong></div></div>
      <div class="top-actions">
        {#if portForwards.length}<div class="port-forward-tray"><button class:port-forward-tray-open={portForwardTrayOpen} class="port-forward-tray-trigger" on:click={() => (portForwardTrayOpen = !portForwardTrayOpen)}>⇄ {portForwards.length} forward{portForwards.length === 1 ? '' : 's'}</button>{#if portForwardTrayOpen}<div class="port-forward-tray-menu"><div><strong>Active port forwards</strong><small>They stop when Kuberniva quits.</small></div>{#each portForwards as forward}<section><div><b>{forward.localAddress}</b><small>{forward.namespace}/{forward.pod} → {forward.remotePort}</small></div><button on:click={() => stopPortForward(forward.id)}>Stop</button></section>{/each}</div>{/if}</div>{/if}
        <button class="command-button" on:click={() => (commandOpen = true)}><Search size={15} strokeWidth={2} /> Search anything <kbd>⌘ K</kbd></button>
        {#if notifications.length}
          <button class="icon-button" aria-label={`${notifications.length} notification${notifications.length === 1 ? '' : 's'}`} title="Clear notifications" on:click={() => (notifications = [])}><Bell size={18} strokeWidth={1.9} /><i></i></button>
        {/if}
      </div>
    </header>

    <div class="content">
      <div class:cluster-page-heading={activeView === 'Clusters'} class="page-heading">
        <div>
          <div class="title-line"><h1>{activeView}</h1>{#if activeClusterId && !catalogError}<span class="live-pill"><b></b> Live</span>{/if}</div>
          <p>{activeClusterId ? `Browsing ${activeCluster} in real time.` : connectedKubeconfig ? 'Choose a cluster from the sidebar to connect.' : 'Connect a kubeconfig to begin.'}</p>
        </div>
        <div class="heading-actions">
          <button class="secondary" disabled={loadingCatalog} on:click={() => activeClusterId ? refreshActiveCluster() : (kubeconfigOpen = true)}>↻ {loadingCatalog ? 'Connecting…' : activeClusterId ? 'Refresh' : connectedKubeconfig ? 'Manage source' : 'Choose source'}</button>
        </div>
      </div>

      {#if activeView === 'Settings'}
        <section class="settings-panel panel"><div class="panel-heading"><div><h2>Workspace settings</h2><p>Connection and display preferences for this device.</p></div></div><div class="settings-row"><div><strong>Kubeconfig sources</strong><small>{kubeconfigSources.length ? `${kubeconfigSources.length} tracked source${kubeconfigSources.length === 1 ? '' : 's'} · newest: ${kubeconfigPath || 'Default: ~/.kube/config'}` : 'No kubeconfig source added'}</small></div><button class="secondary" on:click={() => (kubeconfigOpen = true)}>+ Add source</button></div><div class="settings-row"><div><strong>Manual source sync</strong><small>Startup uses the local context snapshot. Sync only when you want Kuberniva to rescan saved files and folders.</small></div><button class="secondary" disabled={loadingCatalog || !kubeconfigSources.length} on:click={syncKubeconfigSources}>{loadingCatalog ? 'Syncing…' : 'Sync sources'}</button></div><div class="settings-row"><div><strong>Loaded contexts</strong><small>{clusters.length ? `${clusters.length} available locally; OIDC is requested only when one is selected` : 'No kubeconfig context is currently available'}</small></div><button class="secondary" on:click={() => activeClusterId ? refreshActiveCluster() : (kubeconfigOpen = true)}>{activeClusterId ? 'Refresh current' : 'Add source'}</button></div></section>
      {:else if restoringWorkspace}
        <section class="overview-loading"><i></i><div><h2>Restoring your workspace…</h2><p>Reading your saved kubeconfig source locally. Kuberniva will not connect to a cluster or start OIDC until you select one.</p></div></section>
      {:else if !connectedKubeconfig}
        <section class="empty-view connect-empty"><div class="explore-orbit"><i></i><i></i><b>⌁</b></div><h2>{kubeconfigSources.length ? 'No cached contexts yet' : 'Your workspace is empty'}</h2><p>{kubeconfigSources.length ? 'Your saved sources are not scanned at startup. Sync them only when you want to discover their current contexts.' : 'Add a kubeconfig to discover the contexts and API resources you can access.'}</p><button class="primary" on:click={() => kubeconfigSources.length ? syncKubeconfigSources() : (kubeconfigOpen = true)}>{kubeconfigSources.length ? 'Sync saved sources' : '+ Add kubeconfig'}</button></section>
      {:else if activeView === 'Clusters'}
        <section class="clusters-landing">
          <div class="clusters-landing-heading"><div><p class="eyebrow">Cluster manager</p><h2>{clusters.length} available cluster{clusters.length === 1 ? '' : 's'}</h2><p>Open a cluster to connect. Add kubeconfigs and safely remove retired contexts here; OIDC starts only for the cluster you select.</p></div><button class="secondary" on:click={() => (kubeconfigOpen = true)}>+ Add kubeconfig</button></div>
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
                  <button class="remove-cluster-list" disabled={!cluster.kubeconfigPath} title={cluster.kubeconfigPath ? `Remove ${cluster.name} from its source kubeconfig` : 'Source file is unavailable'} on:click={() => removeCluster(cluster.id)}>Remove</button>
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
            <div><p class="eyebrow">Cluster activity</p><h2>Events</h2><p>Recent Kubernetes events across {activeCluster}. Warnings stay visible until the API server expires them.</p></div>
            <div class="events-actions"><small>{eventsObservedAt ? `Updated ${formatObservedTime(eventsObservedAt)}` : 'Not loaded yet'}</small><button class="secondary" disabled={loadingEvents || !activeClusterId} on:click={() => loadClusterEvents(true)}>↻ {loadingEvents ? 'Fetching…' : 'Refresh events'}</button></div>
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
              <div class="events-filter" role="group" aria-label="Event severity"><button class:active={eventTypeFilter === 'All'} on:click={() => (eventTypeFilter = 'All')}>All <b>{clusterEvents.length}</b></button><button class:active={eventTypeFilter === 'Warning'} on:click={() => (eventTypeFilter = 'Warning')}>Warnings <b>{clusterEvents.filter((event) => event.eventType === 'Warning').length}</b></button><button class:active={eventTypeFilter === 'Normal'} on:click={() => (eventTypeFilter = 'Normal')}>Normal <b>{clusterEvents.filter((event) => event.eventType !== 'Warning').length}</b></button></div>
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
              <div class="events-empty"><ScrollText size={28} /><h3>{clusterEvents.length ? 'No matching events' : 'No recent events'}</h3><p>{clusterEvents.length ? 'Try a different filter or severity.' : 'This cluster has not returned any retained Kubernetes events.'}</p></div>
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
          <section class="overview-error panel"><div><span>!</span><div><h2>Node overview is unavailable</h2><p>{overviewError}</p></div></div><button class="secondary" disabled={loadingOverview} on:click={loadClusterOverview}>Try again</button></section>
        {:else if clusterOverview}
          <section class="cluster-overview-dashboard">
          <section class="cluster-hero">
            <div class="cluster-hero-mark"><img class="brand-mark" src="/kuberniva-mark.png" alt="" /></div>
            <div class="cluster-hero-copy"><p class="eyebrow">Cluster overview</p><h2>{activeCluster}</h2><p>Live node capacity and usage. Metrics refresh at most once a minute while this page is open.</p></div>
            <div class="overview-summary"><div><strong>{clusterOverview.nodes.length}</strong><span>Nodes</span></div><div><strong>{readyNodeCount}</strong><span>Ready</span></div><div><strong>{clusterOverview.metricsAvailable ? 'Live' : 'Unavailable'}</strong><span>Metrics API</span></div></div>
          </section>
          <section class="node-overview panel">
            <div class="panel-heading"><div><p class="eyebrow">Infrastructure</p><h2>Nodes</h2><p>{clusterOverview.metricsAvailable ? 'Current CPU and memory are supplied by the Kubernetes Metrics API.' : 'The cluster does not expose the Metrics API; capacity remains available.'}</p></div><div class="overview-actions"><small>Updated {new Date(clusterOverview.observedAt).toLocaleTimeString()}</small><button class="secondary" disabled={loadingOverview} on:click={loadClusterOverview}>↻ {loadingOverview ? 'Updating…' : 'Refresh metrics'}</button></div></div>
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
          <section class="overview-quick-actions"><button class="overview-workloads-tile" on:click={() => navigateTo('Workloads')}><span>▦</span><div><em>Operate</em><strong>Workloads</strong><small>Deployments, Pods, logs, terminal access, and ports in the selected namespace.</small></div><b>→</b></button><button class="overview-resources-tile" on:click={() => navigateTo('Resources')}><span>▤</span><div><em>Explore</em><strong>Resources</strong><small>{catalog.resources.length} discovered Kubernetes APIs, including configuration and CRDs.</small></div><b>→</b></button></section>
          </section>
        {:else}
          <section class="overview-loading"><i></i><div><h2>Preparing cluster overview…</h2><p>Waiting for the first live node response.</p></div></section>
        {/if}
      {:else if activeView === 'Resources'}
        <section class="resource-workbench panel">
          <div class="resource-workbench-heading">
            <div><p class="eyebrow">Discovered Kubernetes APIs</p><h2>Resources</h2><p>Choose a kind, browse its live objects, then inspect or edit one without leaving the workspace.</p></div>
            <div class="resource-workbench-heading-actions">
              <div class="resource-namespace-control">
                <span>Namespace</span>
                <div class="namespace-picker">
                  <button class="namespace-filter" disabled={!activeClusterId || loadingCatalog} aria-label="Namespace" aria-expanded={namespaceOpen} on:click={() => (namespaceOpen = !namespaceOpen)}><span class="namespace-dot"></span>{namespace === 'all namespaces' ? 'All namespaces' : namespace}<span class="chevron">⌄</span></button>
                  {#if namespaceOpen}<div class="namespace-menu" role="menu"><button class:namespace-selected={namespace === 'all namespaces'} on:click={() => chooseNamespace('all namespaces')}><span>All namespaces</span>{#if namespace === 'all namespaces'}✓{/if}</button>{#each catalog.namespaces as availableNamespace}<button class:namespace-selected={namespace === availableNamespace} on:click={() => chooseNamespace(availableNamespace)}><span>{availableNamespace}</span>{#if namespace === availableNamespace}✓{/if}</button>{/each}</div>{/if}
                </div>
              </div>
              <button class="secondary" disabled={loadingCatalog || !activeClusterId} on:click={refreshActiveCluster}>↻ Refresh API</button>
            </div>
          </div>
          {#if !activeClusterId}
            <div class="connection-error"><strong>Select a cluster to begin</strong><p>Kuberniva has only read your local kubeconfig metadata. No cluster connection or OIDC login has been started.</p></div>
          {:else if catalogError}
            <div class="connection-error"><strong>Unable to connect to {activeCluster}</strong><p>{catalogError}</p><button class="secondary" on:click={refreshActiveCluster}>Try again</button></div>
          {:else if loadingCatalog}
            <div class="connection-error"><strong>Connecting to {activeCluster}…</strong><p>Reading the live API catalog and namespaces.</p></div>
          {:else}
            <div class="resource-category-tabs" aria-label="Resource categories"><button class:resource-category-active={selectedCategory === 'All resources'} on:click={() => selectResourceCategory('All resources')}>All <b>{catalog.resources.length}</b></button>{#each resourceCategories as category}<button class:resource-category-active={selectedCategory === category} on:click={() => selectResourceCategory(category)}>{category === 'Cluster' ? 'Cluster-scoped' : category}<b>{categoryCounts[category] || 0}</b></button>{/each}</div>
            <div class="resource-workbench-body">
              <aside class="resource-kind-browser" aria-label="API resource kinds">
                <div class="resource-kind-browser-heading"><div><strong>Resource kind</strong><small>{visibleResources.length} available in {selectedCategory === 'All resources' ? 'all categories' : selectedCategory}</small></div></div>
                <label class="resource-kind-search"><Search size={15} /><input bind:value={resourceSearch} placeholder="Filter API resources" aria-label="Filter API resources" /></label>
                <div class="resource-kind-list">
                  {#if visibleResources.length}
                    {#each visibleResources as resource}
                      <button class:resource-kind-active={selectedResource !== null && resourceKey(selectedResource) === resourceKey(resource)} title={`${resource.kind} · ${resource.apiVersion}`} on:click={() => openResource(resource)}><span class:custom={resource.crd}>{resource.crd ? '◇' : '○'}</span><div><strong>{resource.kind}</strong><small>{resource.group || 'core'} · {resource.namespaced ? 'namespaced' : 'cluster-wide'}</small></div></button>
                    {/each}
                  {:else}<div class="resource-kind-empty"><strong>No matching API resources</strong><small>Try another category or search term.</small></div>{/if}
                </div>
              </aside>
              <aside class="resource-object-browser" aria-label="Resource objects">
                {#if selectedResource}
                  <div class="resource-object-heading"><div><span class:custom={selectedResource.crd}>{selectedResource.crd ? '◇' : '○'}</span><div><strong>{selectedResource.kind}</strong><small>{selectedResource.namespaced ? (namespace === 'all namespaces' ? 'All namespaces' : namespace) : 'Cluster-wide'} · {selectedResource.plural}</small></div></div><b>{resourceObjects.length}</b></div>
                  {#if loadingObjects}<div class="drawer-state"><i></i>Listing {selectedResource.plural}…</div>{:else if resourceObjects.length === 0}<div class="resource-object-empty"><span>○</span><strong>No {selectedResource.plural} found</strong><p>Try another namespace or refresh this API type.</p></div>{:else}<div class="object-list">{#each resourceObjects as object}<button class:object-selected={editorObject?.name === object.name && editorObject?.namespace === object.namespace} on:click={() => openObject(selectedResource!, object)}><span class="object-icon">□</span><div><strong>{object.name}</strong><small>{object.namespace || 'cluster scoped'} {object.createdAt ? `· ${object.createdAt}` : ''}</small></div><span>{selectedResource.kind === 'Pod' ? 'Logs →' : selectedResource.category === 'Workloads' ? 'Details →' : 'Open →'}</span></button>{/each}</div>{/if}
                {:else}
                  <div class="resource-object-empty"><span>⌘</span><strong>Select a resource kind</strong><p>Choose a kind from the left. Kuberniva loads only that API.</p></div>
                {/if}
              </aside>
              <aside class="resource-inspector" aria-label="Resource details">
                <div class="resource-inspector-surface">
                {#if editorResource && editorObject}
                  <div class="drawer-heading inspector-heading"><div><span class:custom={editorResource.custom}>{editorResource.kind === 'Secret' ? '◈' : editorResource.kind === 'ConfigMap' ? '◇' : '⌁'}</span><div><h2>{editorObject.name}</h2><p>{editorResource.kind} · {editorObject.namespace || 'cluster scoped'}</p></div></div><button aria-label="Back to resource objects" on:click={closeEditor}>×</button></div>
                  {#if loadingEditor}
                    <div class="drawer-state"><i></i>Loading live resource details…</div>
                  {:else}
                    {#if editorCertificate}
                      <section class:expired={editorCertificate.expired} class="certificate-card"><div><span>⌁</span><div><strong>{editorCertificate.expired ? 'Certificate expired' : 'TLS certificate'}</strong><p>Expires {editorCertificate.expiresAt}</p></div></div><b>{editorCertificate.expired ? `${Math.abs(editorCertificate.daysRemaining)} days ago` : `${editorCertificate.daysRemaining} days remaining`}</b></section>
                    {/if}
                    {#if editorResource.kind === 'Secret' || editorResource.kind === 'ConfigMap'}
                      <section class="configuration-inspector">
                        <div class="configuration-data-heading"><div><span>{editorResource.kind === 'Secret' ? '◈' : '◇'}</span><div><strong>{editorResource.kind === 'Secret' ? 'Secret data' : 'ConfigMap data'}</strong><small>{editorResource.kind === 'Secret' ? (revealSecret ? 'Decoded values are visible locally' : 'Values are base64 encoded') : 'Plain-text values loaded from this namespace'}</small></div></div><b>{editorEntries.length} {editorEntries.length === 1 ? 'entry' : 'entries'}</b></div>
                        <div class="editor-toolbar"><div><strong>{editorResource.kind === 'Secret' ? 'Protecting sensitive values' : 'Editable key/value data'}</strong><small>{editorResource.kind === 'Secret' ? 'Reveal only when you need to inspect or change a value.' : 'Changes are saved with the current Kubernetes resource version.'}</small></div>{#if editorResource.kind === 'Secret'}<button class="reveal-button" on:click={() => (revealSecret = !revealSecret)}>{revealSecret ? '◉ Hide decoded' : '◌ Reveal decoded'}</button>{/if}</div>
                        <div class="editor-entries">{#if editorEntries.length === 0}<div class="drawer-state">This {editorResource!.kind} has no data entries.</div>{:else}{#each editorEntries as entry, index}<div class="editor-entry"><label>Key<input value={entry.key} on:input={(event) => editorEntries = editorEntries.map((candidate, candidateIndex) => candidateIndex === index ? { ...candidate, key: event.currentTarget.value } : candidate)} /></label><label>Value<textarea value={editorResource!.kind === 'Secret' && revealSecret ? decodeSecret(entry.value) : entry.value} on:input={(event) => updateEditorEntry(index, event.currentTarget.value)} spellcheck="false"></textarea></label><button aria-label={`Remove ${entry.key}`} on:click={() => (editorEntries = editorEntries.filter((_, entryIndex) => entryIndex !== index))}>×</button></div>{/each}{/if}</div>
                        <button class="add-entry" on:click={() => (editorEntries = [...editorEntries, { key: 'new-key', value: editorResource!.kind === 'Secret' && revealSecret ? encodeSecret('') : '' }])}>+ Add key</button>
                      </section>
                    {:else}
                      <div class="inspector-overview">
                        <section class="inspector-summary-grid"><div><span>Kind</span><strong>{editorResource.kind}</strong></div><div><span>Scope</span><strong>{editorObject.namespace || 'Cluster-wide'}</strong></div><div><span>API</span><strong>{editorResource.apiVersion}</strong></div></section>
                        {#if editorResource.category === 'Network'}
                          <section class="network-inspector-card"><div><strong>Hosts & addresses</strong><small>Resolved from this resource’s spec and status</small></div>{#if networkHosts(editorManifest).length}<div class="inspector-chip-list">{#each networkHosts(editorManifest) as host}<span>{host}</span>{/each}</div>{:else}<p>No host or address is declared on this resource.</p>{/if}</section>
                          <section class="network-inspector-card"><div><strong>Ports & listeners</strong><small>Service ports, target ports, or declared listeners</small></div>{#if networkPorts(editorManifest).length}<div class="inspector-chip-list">{#each networkPorts(editorManifest) as port}<span>{port}</span>{/each}</div>{:else}<p>No ports are declared on this resource.</p>{/if}</section>
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
      {:else if activeView === 'Workloads'}
        {#if !activeClusterId}
          <section class="empty-view"><div class="explore-orbit"><i></i><i></i><b>▦</b></div><h2>Select a cluster first</h2><p>Workload inventory is loaded only for the cluster and namespace you choose.</p></section>
        {:else}
          <section class="workloads-page font-sans">
            <div class="mb-6 flex items-end justify-between gap-6">
              <div>
                <div class="mb-2 flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.16em] text-indigo-300"><Activity size={15} class="text-cyan-300" />Live workload inventory</div>
                <h2 class="m-0 text-4xl font-semibold tracking-tight text-white">Workloads</h2>
                <p class="mb-0 mt-2 text-sm text-slate-400">Choose a workload type, then inspect its live objects without breaking your flow.</p>
              </div>
              <div class="flex items-center gap-2">
                <div class="workload-namespace-picker relative"><button class="workload-namespace-trigger inline-flex h-10 items-center gap-2 rounded-lg border border-white/10 bg-white/[0.05] px-3 text-sm font-medium text-slate-200 shadow-sm transition hover:border-indigo-400/50 hover:bg-white/[0.09]" aria-expanded={namespaceOpen} on:click={() => (namespaceOpen = !namespaceOpen)}><SlidersHorizontal size={15} class="text-slate-400" />{namespace}<span class="text-slate-400">⌄</span></button>{#if namespaceOpen}<div class="workload-namespace-menu absolute right-0 top-12 z-30 max-h-80 w-64 overflow-auto rounded-xl border border-white/10 bg-[#171b27] p-1.5 shadow-2xl shadow-black/40" role="menu"><button class:namespace-selected={namespace === 'all namespaces'} class="flex w-full items-center justify-between rounded-lg px-3 py-2.5 text-left text-sm text-slate-300 hover:bg-white/[0.07]" on:click={() => chooseNamespace('all namespaces')}><span>All namespaces</span>{#if namespace === 'all namespaces'}<span class="text-cyan-300">✓</span>{/if}</button>{#each catalog.namespaces as availableNamespace}<button class:namespace-selected={namespace === availableNamespace} class="flex w-full items-center justify-between rounded-lg px-3 py-2.5 text-left text-sm text-slate-300 hover:bg-white/[0.07]" on:click={() => chooseNamespace(availableNamespace)}><span>{availableNamespace}</span>{#if namespace === availableNamespace}<span class="text-cyan-300">✓</span>{/if}</button>{/each}</div>{/if}</div>
                <button class="inline-flex h-10 items-center gap-2 rounded-lg bg-gradient-to-r from-indigo-500 to-violet-500 px-3.5 text-sm font-semibold text-white shadow-lg shadow-indigo-950/40 transition hover:from-indigo-400 hover:to-violet-400 disabled:cursor-wait disabled:opacity-60" disabled={loadingWorkloads} on:click={() => navigateTo('Workloads')}><RefreshCw size={15} class={loadingWorkloads ? 'animate-spin' : ''} />Refresh</button>
              </div>
            </div>

            <div class:workload-detail-open={editorResource?.category === 'Workloads' && editorObject !== null} class="workload-grid grid min-h-[560px] grid-cols-[230px_minmax(0,1fr)] gap-5">
              <aside class="rounded-2xl border border-white/10 bg-[#151924]/90 p-3 shadow-2xl shadow-black/10"><div class="border-b border-white/10 px-2 pb-3"><p class="m-0 text-sm font-semibold text-white">Resource types</p><p class="mb-0 mt-1 text-xs text-slate-400">Demand-loaded per type</p></div><div class="mt-2 space-y-1">{#if workloadResources.length}{#each workloadResources as resource}<button class={`flex w-full items-center gap-2 rounded-lg px-2.5 py-2.5 text-left text-sm font-medium transition hover:bg-white/[0.06] ${workloadResource?.kind === resource.kind ? 'bg-indigo-500/15 text-indigo-200 ring-1 ring-inset ring-indigo-400/20' : 'text-slate-400'}`} on:click={() => selectWorkloadResource(resource)}><Boxes size={16} class={workloadResource?.kind === resource.kind ? 'text-cyan-300' : 'text-slate-500'} /><span class="min-w-0 flex-1 truncate">{resource.kind}</span>{#if workloadResource?.kind === resource.kind}<span class="rounded-md bg-black/20 px-1.5 py-0.5 font-mono text-[10px] text-indigo-200">{workloadObjects.length}</span>{/if}</button>{/each}{:else}<p class="px-2 py-4 text-xs leading-5 text-slate-500">No workload APIs were discovered.</p>{/if}</div></aside>

              <div class="workload-list-panel overflow-hidden rounded-2xl border border-white/10 bg-[#151924]/90 shadow-2xl shadow-black/10"><div class="flex items-center justify-between gap-4 border-b border-white/10 px-5 py-4"><div><div class="flex items-center gap-2"><Container size={18} class="text-cyan-300" /><h3 class="m-0 text-lg font-semibold text-white">{workloadResource?.kind || 'Select a type'}</h3></div><p class="mb-0 mt-1 text-xs text-slate-400">{namespace} · {workloadResource?.apiVersion || 'Kubernetes API'}</p></div><label class="flex h-10 w-72 items-center gap-2 rounded-lg border border-white/10 bg-black/20 px-3 text-slate-500 focus-within:border-indigo-400 focus-within:bg-black/30 focus-within:ring-2 focus-within:ring-indigo-500/20"><Search size={16} /><input class="min-w-0 flex-1 border-0 bg-transparent text-sm text-slate-100 outline-none placeholder:text-slate-500" bind:value={workloadSearch} placeholder={`Filter ${workloadResource?.plural || 'workloads'}`} /></label></div>
                {#if loadingWorkloads}
                  <div class="grid min-h-96 place-items-center text-sm text-slate-400"><div class="flex items-center gap-3"><RefreshCw size={18} class="animate-spin text-cyan-300" />Loading {workloadResource?.plural || 'workloads'}…</div></div>
                {:else if visibleWorkloadObjects.length === 0}
                  <div class="grid min-h-96 place-items-center px-6 text-center"><div><div class="mx-auto grid h-12 w-12 place-items-center rounded-2xl bg-indigo-500/15 text-cyan-300"><Boxes size={22} /></div><h4 class="mb-0 mt-4 text-base font-semibold text-slate-100">{workloadObjects.length ? 'No matching workloads' : `No ${workloadResource?.plural || 'workloads'} found`}</h4><p class="mb-0 mt-2 text-sm text-slate-400">{workloadObjects.length ? 'Try a different name or namespace filter.' : `Nothing was returned for ${namespace}.`}</p></div></div>
                {:else}
                  <div class="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-3 p-5">{#each visibleWorkloadObjects as workload}<button class="group min-h-40 rounded-xl border border-white/10 bg-white/[0.025] p-4 text-left shadow-sm transition hover:-translate-y-0.5 hover:border-indigo-400/50 hover:bg-indigo-400/[0.06] hover:shadow-lg hover:shadow-black/20" on:click={() => workloadResource && openObject(workloadResource, workload)}><div class="flex items-start justify-between gap-3"><span class="grid h-9 w-9 place-items-center rounded-lg bg-indigo-500/15 text-cyan-300"><Container size={18} /></span><ChevronRight size={18} class="text-slate-600 transition group-hover:translate-x-0.5 group-hover:text-cyan-300" /></div><h4 class="mt-5 truncate text-sm font-semibold text-white">{workload.name}</h4><p class="mt-1 truncate font-mono text-[11px] text-slate-400">{workload.namespace || 'cluster scope'}</p><div class="mt-5 flex items-center justify-between border-t border-white/10 pt-3 text-xs"><span class="inline-flex items-center gap-1.5 text-slate-400"><CircleDot size={12} class="text-cyan-300" />Live object</span><span class="font-semibold text-indigo-200">Open</span></div></button>{/each}</div>
                {/if}
              </div>
              {#if editorResource && editorObject && editorResource.category === 'Workloads'}
                <aside class="workload-inspector" aria-label="Workload details">
                  <div class="workload-inspector-heading"><div><p class="eyebrow">Live workload details</p><h3>{editorObject.name}</h3><p>{editorResource.kind} · {editorObject.namespace || 'cluster scoped'}</p></div><div class="workload-inspector-actions"><button class="workload-delete" disabled={loadingEditor} on:click={() => requestResourceDeletion(editorResource!, editorObject!)}>Delete</button><button aria-label="Close workload details" on:click={closeEditor}>×</button></div></div>
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
                      <section class="workload-action-grid"><button class:workload-action-loading={openingWorkloadLogs} class="workload-action-card workload-logs-action" disabled={openingWorkloadLogs} aria-busy={openingWorkloadLogs} on:click={() => openWorkloadLogs(editorResource!, editorObject!)}><span>{#if openingWorkloadLogs}<RefreshCw size={18} class="workload-action-spinner" />{:else}≡{/if}</span><div><strong>{openingWorkloadLogs ? 'Opening logs…' : 'View logs'}</strong><small>{openingWorkloadLogs ? 'Finding live Pods and preparing the stream' : 'Choose a live Pod and stream its output'}</small></div><b>{openingWorkloadLogs ? '•••' : '→'}</b></button><button class="workload-action-card workload-terminal-action" disabled={loadingTerminalPods} on:click={() => openWorkloadTerminal(editorResource!, editorObject!)}><span>⌘</span><div><strong>Terminal</strong><small>Tunnel into a Pod container with Kubernetes exec</small></div><b>→</b></button></section>
                      <section class="workload-status-grid"><div><span>Replicas</span><strong>{workloadReplicaSummary(editorManifest)}</strong></div><div><span>API</span><strong>{editorResource.apiVersion}</strong></div></section>
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
        <section class="empty-view"><div class="explore-orbit"><i></i><i></i><b>⌕</b></div><h2>Explore without memorizing paths</h2><p>Ask for a resource, filter it, and move between related objects in one place.</p><button class="primary" on:click={() => (commandOpen = true)}>Search resources</button></section>
      {:else}
        {#if logTarget}
          <section class="logs-workspace panel">
            <aside class="log-pod-sidebar" aria-label="Pods with logs">
              <div class="log-pod-sidebar-heading"><div><p class="eyebrow">{logScopeLabel || 'Pod'} stream</p><h2>Pods</h2><p>{logPods.length} available in this view</p></div><div class="log-pod-heading-actions"><button class:port-forward-open={portForwardOpen} class="port-forward-button" on:click={openPortForwardForm}>⇄ Forward</button><span>{logPods.length}</span></div></div>
              <div class="log-pod-list">{#each logPods as pod}<button class:log-pod-selected={logTarget.pod === pod.name && logTarget.namespace === (pod.namespace || namespace)} on:click={() => selectLogPod(pod)}><span class="log-pod-dot"></span><div><strong>{pod.name}</strong><small>{pod.namespace || namespace}</small></div><span class="log-pod-arrow">→</span></button>{/each}</div>
              {#if logPorts.length}<section class="log-port-section"><div><span>Container ports</span><small>{logPorts.length}</small></div>{#each logPorts as port}<span class="log-port-chip" title={`${port.container}${port.name ? ` · ${port.name}` : ''} · ${port.protocol}`}><b>{port.port}/{port.protocol}</b><small>{port.container}{port.name ? ` · ${port.name}` : ''}</small></span>{/each}</section>{/if}
              {#if portForwardOpen}<section class="port-forward-form"><div><strong>Port forward</strong><button aria-label="Close port forward" on:click={() => (portForwardOpen = false)}>×</button></div><p>Expose {logTarget.pod} only on this Mac.</p><label>Remote port<input list="kuberniva-pod-ports" type="number" min="1" max="65535" bind:value={portForwardRemotePort} placeholder="e.g. 8080" /></label><datalist id="kuberniva-pod-ports">{#each suggestedForwardPorts as port}<option value={port}></option>{/each}</datalist><label>Local port<input type="number" min="1" max="65535" bind:value={portForwardLocalPort} placeholder="e.g. 8080" /></label><button class="primary" disabled={portForwarding} on:click={startPortForward}>{portForwarding ? 'Starting…' : 'Start forward'}</button></section>{/if}
              <div class="log-pod-sidebar-footer">Switch Pods without leaving the log stream. Port forwards remain active until you stop them or quit Kuberniva.</div>
            </aside>
            <div class="log-stream-panel">
              <div class="log-stream-heading"><div><p class="eyebrow">Streaming output</p><h2>{logTarget.pod}</h2><p>{activeCluster} · {logScopeLabel || 'Pod'} · {logTarget.namespace}</p></div><div class="table-actions"><button class="secondary" on:click={() => loadLogs(true)}>↻ Refresh now</button><button class="secondary" on:click={() => { closeLogs(); void navigateTo('Workloads') }}>← Back to workloads</button></div></div>
              <div class="log-toolbar"><div><strong>Live logs</strong><small>{loadingLogs ? 'Refreshing…' : 'Refreshes every 30 seconds · scroll up to hold your place'}</small></div>{#if logContainers.length > 1}<label>Container <select bind:value={selectedLogContainer} on:change={() => loadLogs(true)}>{#each logContainers as container}<option value={container}>{container}</option>{/each}</select></label>{/if}</div>
              <pre class="live-log-output" bind:this={logViewport}>{#if logLines.length}{logLines.join('\n')}{:else}No log lines returned yet.{/if}</pre>
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
          <p class="deletion-intro">{deletionTarget.type === 'resource' ? 'This sends a Kubernetes DELETE request only to the cluster and namespace shown below.' : 'This changes only the local kubeconfig source shown below; it does not delete the remote Kubernetes cluster.'}</p>
          <dl class="deletion-target-summary">
            <div><dt>Target</dt><dd>{deletionTargetName()}</dd></div>
            {#if deletionTarget.type === 'resource'}
              <div><dt>Cluster</dt><dd>{activeCluster}</dd></div><div><dt>Namespace</dt><dd>{deletionTarget.object.namespace || 'cluster-scoped'}</dd></div>
            {:else}
              <div><dt>Source</dt><dd title={deletionTarget.cluster.kubeconfigPath}>{deletionTarget.cluster.kubeconfigPath}</dd></div>
            {/if}
          </dl>
          <div class="deletion-warning">{deletionTarget.type === 'resource' ? 'Deletion cannot be undone. Kubernetes may handle dependents according to the resource’s configured deletion policy.' : 'This changes the source file immediately. Only this context and unreferenced credentials or cluster entries are removed.'}</div>
          <div class="deletion-actions"><button class="secondary" disabled={deletingResource} on:click={cancelDeletion}>Cancel</button><button class="destructive" disabled={deletingResource} on:click={continueDeletion}>Continue</button></div>
        {:else}
          <p class="eyebrow">Final confirmation</p>
          <h2>Type the name to confirm</h2>
          <p class="deletion-intro">To complete this deletion, type <strong>{deletionTargetName()}</strong> exactly. This prevents an accidental delete from a fast click.</p>
          <label class="deletion-name-input">{deletionTarget.type === 'resource' ? 'Resource name' : 'Context name'}<input bind:value={deletionName} autocomplete="off" spellcheck="false" placeholder={deletionTargetName()} /></label>
          <div class="deletion-actions"><button class="secondary" disabled={deletingResource} on:click={() => (deletionStep = 1)}>Back</button><button class="destructive" disabled={deletingResource || deletionName !== deletionTargetName()} on:click={confirmDeletion}>{deletingResource ? 'Deleting…' : deletionTarget.type === 'resource' ? 'Delete resource' : 'Remove context'}</button></div>
        {/if}
      </div>
    </div>
  {/if}

  {#if commandOpen}
    <div class="modal-backdrop" role="presentation" on:click={() => (commandOpen = false)}>
      <div class="command-modal" role="dialog" aria-modal="true" aria-label="Search Kuberniva" tabindex="-1" on:click|stopPropagation on:keydown|stopPropagation>
        <div class="command-input"><span>⌕</span><input placeholder="Search workloads, pods, namespaces…" /><kbd>esc</kbd></div>
        <p class="eyebrow">Navigate</p>
        <button on:click={() => { void navigateTo('Resources'); commandOpen = false }}>▤ <span>Browse discovered resources</span><kbd>↵</kbd></button>
        <button on:click={() => { kubeconfigOpen = true; commandOpen = false }}>⌁ <span>Add a kubeconfig</span></button>
      </div>
    </div>
  {/if}

  {#if kubeconfigOpen}
    <div class="modal-backdrop" role="presentation" on:click={() => !loadingCatalog && (kubeconfigOpen = false)}>
      <div class="kubeconfig-modal" role="dialog" aria-modal="true" aria-label="Add kubeconfig" tabindex="-1" on:click|stopPropagation on:keydown|stopPropagation>
        <div class="modal-kube-mark">⌁</div><h2>Add kubeconfig source</h2><p>Choose a kubeconfig file or a folder containing multiple files. This always adds to the contexts already tracked in Kuberniva; it never replaces them. Paths beginning with <code>~/</code>, relative paths from your home folder, and absolute paths are all accepted.</p>
        <label>kubeconfig file or directory <input bind:value={kubeconfigPath} placeholder="Default: ~/.kube/config" /></label>
        <div class="source-picker-actions"><button class="secondary" on:click={() => chooseKubeconfig(false)}>Choose file…</button><button class="secondary" on:click={() => chooseKubeconfig(true)}>Choose folder…</button></div>
        <div class="kubeconfig-modal-footer"><button class="secondary" on:click={() => (kubeconfigOpen = false)}>Cancel</button><button class="primary" disabled={loadingCatalog} on:click={connectKubeconfig}>{loadingCatalog ? 'Discovering…' : 'Add & discover'}</button></div>
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
          <div class="related-pods"><div class="related-heading"><button on:click={() => { relatedPods = null; relatedObject = null }}>← Back</button><span>Pods selected by this {selectedResource.kind}</span>{#if relatedObject}<button class="inline-yaml" on:click={() => selectedResource && relatedObject && openYamlEditor(selectedResource, relatedObject)}>View YAML</button>{/if}</div>{#if relatedPods.length === 0}<div class="drawer-state">No matching Pods found.</div>{:else}<div class="object-list">{#each relatedPods as pod}<button on:click={() => openPodLogs(pod, relatedPods || [], `${selectedResource?.kind || 'Workload'} · ${relatedObject?.name || 'workload'}`)}><span class="object-icon">□</span><div><strong>{pod.name}</strong><small>{pod.namespace || 'namespace unavailable'}</small></div><span>Logs →</span></button>{/each}</div>{/if}</div>
        {:else if resourceObjects.length === 0}
          <div class="drawer-state">No {selectedResource.plural} found in this scope.</div>
        {:else}
          <div class="object-list">{#each resourceObjects as object}<button on:click={() => openObject(selectedResource!, object)}><span class="object-icon">□</span><div><strong>{object.name}</strong><small>{object.namespace || 'cluster scoped'} {object.createdAt ? `· ${object.createdAt}` : ''}</small></div><span>{selectedResource.kind === 'Pod' ? 'Logs →' : selectedResource.category === 'Workloads' ? 'Pods →' : 'Details →'}</span></button>{/each}</div>
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
        <div class="drawer-heading"><div><span class:custom={editorResource.custom}>{editorResource.kind === 'Secret' ? '◈' : editorResource.kind === 'ConfigMap' ? '◇' : '⌁'}</span><div><h2>{editorObject.name}</h2><p>{editorResource.kind} · {editorObject.namespace || 'cluster scoped'}</p></div></div><button aria-label="Close editor" on:click={closeEditor}>×</button></div>
        {#if loadingEditor}
          <div class="drawer-state"><i></i>Loading live resource data…</div>
        {:else}
          {#if editorCertificate}
            <section class:expired={editorCertificate.expired} class="certificate-card"><div><span>⌁</span><div><strong>{editorCertificate.expired ? 'Certificate expired' : 'TLS certificate'}</strong><p>Expires {editorCertificate.expiresAt}</p></div></div><b>{editorCertificate.expired ? `${Math.abs(editorCertificate.daysRemaining)} days ago` : `${editorCertificate.daysRemaining} days remaining`}</b></section>
          {/if}
          {#if editorResource.kind === 'Secret' || editorResource.kind === 'ConfigMap'}
            <div class="editor-toolbar"><div><strong>{editorResource.kind === 'Secret' ? 'Secret data' : 'ConfigMap data'}</strong><small>{editorResource.kind === 'Secret' ? (revealSecret ? 'Decoded values are visible locally' : 'Values are base64 encoded') : 'Plain-text values'}</small></div>{#if editorResource.kind === 'Secret'}<button class="reveal-button" on:click={() => (revealSecret = !revealSecret)}>{revealSecret ? '◉ Hide decoded' : '◌ Reveal decoded'}</button>{/if}</div>
            <div class="editor-entries">{#if editorEntries.length === 0}<div class="drawer-state">This {editorResource!.kind} has no data entries.</div>{:else}{#each editorEntries as entry, index}<div class="editor-entry"><label>Key<input value={entry.key} on:input={(event) => editorEntries = editorEntries.map((candidate, candidateIndex) => candidateIndex === index ? { ...candidate, key: event.currentTarget.value } : candidate)} /></label><label>Value<textarea value={editorResource!.kind === 'Secret' && revealSecret ? decodeSecret(entry.value) : entry.value} on:input={(event) => updateEditorEntry(index, event.currentTarget.value)} spellcheck="false"></textarea></label><button aria-label={`Remove ${entry.key}`} on:click={() => (editorEntries = editorEntries.filter((_, entryIndex) => entryIndex !== index))}>×</button></div>{/each}{/if}</div>
            <button class="add-entry" on:click={() => (editorEntries = [...editorEntries, { key: 'new-key', value: editorResource!.kind === 'Secret' && revealSecret ? encodeSecret('') : '' }])}>+ Add key</button>
          {:else if !editorCertificate}
            <div class="drawer-state">This resource has no editable data view yet.</div>
          {/if}
        {/if}
        <div class="drawer-footer"><span>{editorResource.kind === 'Secret' ? 'Decoded values are never stored by Kuberniva.' : 'Saving uses the cluster resource version.'}</span><div class="editor-footer-actions"><button class="secondary" disabled={loadingEditor} on:click={() => openYamlEditor(editorResource!, editorObject!)}>View YAML</button>{#if editorResource.kind === 'Secret' || editorResource.kind === 'ConfigMap'}<button class="primary" disabled={loadingEditor || savingEditor} on:click={saveEditor}>{savingEditor ? 'Saving…' : 'Save changes'}</button>{:else}<button class="secondary" on:click={closeEditor}>Close</button>{/if}</div></div>
      </div>
    </aside>
  {/if}

  {#if toast}<div class="toast"><span>✓</span>{toast}</div>{/if}
</main>
