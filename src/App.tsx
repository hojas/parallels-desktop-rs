import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import "./App.css";

interface VmConfig {
  id: string;
  name: string;
  cpu_count: number;
  memory_mb: number;
  disk_path: string;
  iso_path: string | null;
}

interface SnapshotInfo {
  tag: string;
  description: string;
  created_at: number;
}

interface SerialPayload { vm_id: string; line: string }
interface StatusPayload { vm_id: string; state: string }
interface ErrorPayload { vm_id: string; message: string }

function App() {
  const [serialOutput, setSerialOutput] = useState("");
  const [vmRunning, setVmRunning] = useState(false);
  const [activeVmId, setActiveVmId] = useState<string | null>(null);
  const [vms, setVms] = useState<VmConfig[]>([]);
  const [snapshots, setSnapshots] = useState<SnapshotInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [snapshotTag, setSnapshotTag] = useState("");
  const [snapshotDesc, setSnapshotDesc] = useState("");
  const [config, setConfig] = useState({
    name: "Debian ARM64", cpu_count: 4, memory_mb: 4096, disk_path: "", iso_path: "",
  });
  const outputRef = useRef<HTMLPreElement>(null);
  const unlistenRef = useRef<UnlistenFn[]>([]);

  useEffect(() => { loadVms(); return () => unlistenRef.current.forEach((f) => f()); }, []);
  useEffect(() => { if (outputRef.current) outputRef.current.scrollTop = outputRef.current.scrollHeight; }, [serialOutput]);

  const loadVms = async () => { try { setVms(await invoke<VmConfig[]>("list_vms")); } catch (e) { showError(String(e)); } };
  const loadSnapshots = async (vmId: string) => { try { setSnapshots(await invoke<SnapshotInfo[]>("snapshot_list", { vmId })); } catch { setSnapshots([]); } };
  const showError = (msg: string) => { setError(msg); setTimeout(() => setError(null), 8000); };

  const setupListeners = async () => {
    unlistenRef.current.forEach((f) => f()); unlistenRef.current = [];
    const u1 = await listen<SerialPayload>("vm:serial", (e) => setSerialOutput((p) => p + e.payload.line + "\n"));
    const u2 = await listen<StatusPayload>("vm:status", (e) => {
      if (e.payload.state === "running") setVmRunning(true);
      else if (e.payload.state === "stopped") setVmRunning(false);
    });
    const u3 = await listen<ErrorPayload>("vm:error", (e) => showError(`VM ${e.payload.vm_id}: ${e.payload.message}`));
    unlistenRef.current = [u1, u2, u3];
  };

  const handleCreateVm = async () => {
    if (!config.disk_path) { showError("Disk image path is required"); return; }
    try {
      const created = await invoke<VmConfig>("create_vm", { config: { ...config, iso_path: config.iso_path || null } });
      setActiveVmId(created.id); await loadVms();
      setSerialOutput((p) => p + `[app] Created: ${created.id}\n`);
    } catch (e) { showError(String(e)); }
  };

  const handleDeleteVm = async () => {
    if (!activeVmId) return;
    if (!confirm("Delete this VM and all its data?")) return;
    try { await invoke("delete_vm", { vmId: activeVmId }); setActiveVmId(null); setSerialOutput(""); await loadVms(); } catch (e) { showError(String(e)); }
  };

  const handleStartVm = useCallback(async () => {
    if (!activeVmId) { showError("No VM selected"); return; }
    try { await setupListeners(); setSerialOutput("[app] Starting...\n"); await invoke("start_vm", { vmId: activeVmId }); } catch (e) { showError(`Start: ${e}`); }
  }, [activeVmId]);

  const handleStopVm = async () => {
    if (!activeVmId) return;
    try { setSerialOutput((p) => p + "[app] Stopping...\n"); await invoke("stop_vm", { vmId: activeVmId }); } catch (e) { showError(String(e)); }
  };

  const handleSelectVm = (vmId: string) => {
    setActiveVmId(vmId); setSerialOutput(""); setSnapshots([]);
    const vm = vms.find((v) => v.id === vmId);
    if (vm) setConfig({ name: vm.name, cpu_count: vm.cpu_count, memory_mb: vm.memory_mb, disk_path: vm.disk_path, iso_path: vm.iso_path || "" });
    loadSnapshots(vmId);
  };

  const handleSnapshot = async () => {
    if (!activeVmId || !snapshotTag) return;
    try {
      const info = await invoke<SnapshotInfo>("snapshot_create", { vmId: activeVmId, tag: snapshotTag, description: snapshotDesc });
      setSnapshots((p) => [...p.filter((s) => s.tag !== info.tag), info]);
      setSnapshotTag(""); setSnapshotDesc("");
    } catch (e) { showError(String(e)); }
  };

  const handleSnapshotRestore = async (tag: string) => { if (!activeVmId) return; try { await invoke("snapshot_restore", { vmId: activeVmId, tag }); } catch (e) { showError(String(e)); } };
  const handleSnapshotDelete = async (tag: string) => {
    if (!activeVmId) return;
    try { await invoke("snapshot_delete", { vmId: activeVmId, tag }); setSnapshots((p) => p.filter((s) => s.tag !== tag)); } catch (e) { showError(String(e)); }
  };

  const fmt = (ts: number) => new Date(ts * 1000).toLocaleString();

  return (
    <div className="app">
      {error && <div className="error-banner" onClick={() => setError(null)}>{error}</div>}
      <header className="toolbar">
        <h1>Parallels Desktop RS</h1>
        <div className="toolbar-actions">
          {!vmRunning ? <button className="btn-start" onClick={handleStartVm}>Start VM</button>
            : <button className="btn-stop" onClick={handleStopVm}>Stop VM</button>}
          {activeVmId && <button className="btn-delete" onClick={handleDeleteVm}>Delete</button>}
        </div>
      </header>
      <main className="main">
        <aside className="sidebar">
          <h2>Saved VMs</h2>
          <ul className="vm-list">
            {vms.length === 0 && <li className="vm-empty">No VMs</li>}
            {vms.map((vm) => (
              <li key={vm.id} className={`vm-item ${activeVmId === vm.id ? "active" : ""}`} onClick={() => handleSelectVm(vm.id)}>
                <span className="vm-name">{vm.name}</span>
                <span className="vm-detail">{vm.cpu_count} CPU · {vm.memory_mb}MB</span>
              </li>
            ))}
          </ul>

          <h2>Config</h2>
          <label>Name <input type="text" value={config.name} onChange={(e) => setConfig({...config, name: e.target.value})} /></label>
          <label>CPU <input type="number" min={1} max={16} value={config.cpu_count} onChange={(e) => setConfig({...config, cpu_count: Number(e.target.value)})} /></label>
          <label>RAM MB <input type="number" min={512} max={65536} step={512} value={config.memory_mb} onChange={(e) => setConfig({...config, memory_mb: Number(e.target.value)})} /></label>
          <label>Disk <input type="text" placeholder="~/VMs/debian.qcow2" value={config.disk_path} onChange={(e) => setConfig({...config, disk_path: e.target.value})} /></label>
          <label>ISO <input type="text" placeholder="~/Downloads/debian.iso" value={config.iso_path} onChange={(e) => setConfig({...config, iso_path: e.target.value})} /></label>
          <button className="btn-create" onClick={handleCreateVm}>Save VM</button>

          {activeVmId && vmRunning && (
            <>
              <h2>Snapshots</h2>
              <div className="snapshot-form">
                <input type="text" placeholder="tag" value={snapshotTag} onChange={(e) => setSnapshotTag(e.target.value)} />
                <input type="text" placeholder="desc" value={snapshotDesc} onChange={(e) => setSnapshotDesc(e.target.value)} />
                <button className="btn-snapshot" onClick={handleSnapshot} disabled={!snapshotTag}>Save</button>
              </div>
              <ul className="snapshot-list">
                {snapshots.map((s) => (
                  <li key={s.tag} className="snapshot-item">
                    <span className="snap-tag">{s.tag}</span>
                    <span className="snap-time">{fmt(s.created_at)}</span>
                    <div className="snap-actions">
                      <button onClick={() => handleSnapshotRestore(s.tag)}>Restore</button>
                      <button className="btn-danger" onClick={() => handleSnapshotDelete(s.tag)}>Del</button>
                    </div>
                  </li>
                ))}
              </ul>
            </>
          )}
        </aside>
        <section className="content">
          <div className="terminal">
            <div className="terminal-header">
              <span>Serial Console</span>
              <span className={`status ${vmRunning ? "running" : "stopped"}`}>{vmRunning ? "Running" : "Stopped"}</span>
            </div>
            <pre className="terminal-output" ref={outputRef}>{serialOutput || "VM output will appear here..."}</pre>
          </div>
        </section>
      </main>
    </div>
  );
}

export default App;
