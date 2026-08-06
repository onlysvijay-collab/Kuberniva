<p align="center">
  <img src="public/kuberniva-mark.png" alt="Kuberniva logo" width="160">
</p>

<h1 align="center">Kuberniva</h1>

<p align="center"><strong>Kubernetes, in focus.</strong><br>Fast, native cluster operations for people who work across many Kubernetes environments.</p>

Kuberniva is a fast local Kubernetes workspace for moving between clusters, resources, logs, and safe operations without loading sample data.

It discovers contexts from a file or folder of kubeconfigs, connects lazily when you select a cluster, and works with OIDC `exec` authentication. It supports Kubernetes API resources and CRDs, namespace-aware resource browsing, live node CPU/memory, pod logs, ConfigMap and Secret editing, YAML editing, certificate expiry, and native Pod port forwarding.

## Run from source

```bash
npm install
npm run tauri dev
```

For the browser-only UI preview:

```bash
npm run dev
```

## Build for Apple Silicon

```bash
npm run tauri build -- --bundles app
```

The resulting app is in `src-tauri/target/release/bundle/macos/Kuberniva.app`. For sharing an ad-hoc signed build, copy the app into a ZIP and send it. On the recipient Mac, move **Kuberniva.app** to Applications, then run once:

```bash
xattr -cr /Applications/Kuberniva.app
```

An Apple Developer ID certificate and notarization are required to avoid the Gatekeeper workaround for public distribution.

## Architecture

- `src/` — Svelte + TypeScript workspace
- `src-tauri/` — Rust/Tauri desktop host and Kubernetes client
