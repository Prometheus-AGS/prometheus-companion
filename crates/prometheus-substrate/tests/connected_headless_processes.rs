//! Real two-host acceptance: canonical fixture storage, two headless processes,
//! signed Unix HTTP mutation, selected stdio MCP, fsynced intent/receipt, and
//! remote peer application. No in-process substitute for the transport/consumer.
//! Run only through the scratch wrapper with explicit prebuilt binaries.

#![cfg(unix)]

use anyhow::{bail, ensure, Context, Result};
use kbd_runtime::{
    ensure_device_key_file, Actor, CommandEnvelope, CommandKind, DeviceRecord,
    DeviceStatus, Phase, Runtime, SignedCommandEnvelope, WorkStatus,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const BUDGET: Duration = Duration::from_secs(90);

fn input(name: &str) -> Result<PathBuf> {
    let path = PathBuf::from(std::env::var_os(name).with_context(|| format!("BLOCKED: missing {name}"))?);
    ensure!(path.is_absolute() && path.exists(), "BLOCKED: {name} must exist at an absolute path");
    Ok(path)
}

struct Process(Child);

impl Process {
    fn stop(&mut self) -> Result<()> {
        if self.0.try_wait()?.is_some() { return Ok(()); }
        let status = Command::new("/bin/kill").args(["-TERM", &self.0.id().to_string()]).status()?;
        ensure!(status.success(), "could not signal owned process");
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if let Some(status) = self.0.try_wait()? {
                ensure!(status.success(), "owned process shutdown failed: {status}");
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        bail!("owned process did not shut down within ten seconds")
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // Kill/wait only this scenario's child; the shell wrapper also owns the
        // encompassing process group for interruption/timeouts.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Host {
    project: PathBuf,
    data: PathBuf,
    key: PathBuf,
    skills: PathBuf,
    socket: PathBuf,
    config: PathBuf,
    identity: PathBuf,
    env: BTreeMap<String, String>,
}

impl Host {
    fn new(root: PathBuf, project_id: &str, tools: &Path) -> Result<Self> {
        let project = root.join("project");
        let data = root.join("data");
        let skills = root.join("skills");
        for path in [root.join("home"), root.join("config"), root.join("tmp"), data.clone(), skills.clone(), project.join(".prometheus"), project.join(".kbd-orchestrator")] {
            fs::create_dir_all(path)?;
        }
        fs::write(project.join(".prometheus/project.json"), serde_json::to_vec(&json!({
            "schemaVersion":"1", "projectId":project_id,
            "repositoryFingerprint":"sha256:ldd-connected-scratch-fixture"
        }))?)?;
        let key = root.join("config/device-key.json");
        let socket = root.join("control.sock");
        let config = root.join("config/node.toml");
        let identity = root.join("config/p2p-identity.json");
        let values = [
            ("PATH", tools.to_path_buf()), ("HOME",root.join("home")),
            ("CODEX_HOME",root.join("codex")), ("XDG_CONFIG_HOME",root.join("config")),
            ("XDG_DATA_HOME",data.clone()), ("PROMETHEUS_DATA_DIR",data.clone()),
            ("PROMETHEUS_DEVICE_KEY_FILE",key.clone()), ("PROMETHEUS_SKILLS_DIR",skills.clone()),
            ("SOVEREIGN_SYNC_SOCKET",socket.clone()), ("SOVEREIGN_SYNC_CONFIG",config.clone()),
            ("CLAUDE_CONFIG_DIR",root.join("claude")), ("CORTEX_DATA_DIR",root.join("cortex")),
            ("PROMETHEUS_LEARNING_QUEUE",root.join("queue")), ("PROMETHEUS_LEARNING_LOG_DIR",root.join("log")),
            ("PROMETHEUS_LEARNING_INDEX_DIR",root.join("index")), ("PROMETHEUS_TEAM_DIGEST_DIR",root.join("digest")),
            ("PROMETHEUS_PACK_ROOT",root.join("absent-pack")), ("TMPDIR",root.join("tmp")),
        ];
        let mut env: BTreeMap<String,String> = values.into_iter().map(|(k,v)|(k.into(),v.to_string_lossy().into_owned())).collect();
        env.insert("PROMETHEUS_LEARNING_CORTEX".into(),"0".into());
        env.insert("PROMETHEUS_DEVICE_ID".into(),root.file_name().unwrap().to_string_lossy().into_owned());
        env.insert("GIT_CONFIG_GLOBAL".into(),"/dev/null".into());
        env.insert("GIT_CONFIG_NOSYSTEM".into(),"1".into());
        env.insert("RUST_LOG".into(),"prometheus_substrate=info,sovereign_sync=info".into());
        Ok(Self {project,data,key,skills,socket,config,identity,env})
    }

    fn select_fixture_identity(&self) {
        // Fixture setup runs sequentially under --test-threads=1. Explicit file
        // selection occurs before every real canonical Runtime open/sign call.
        for (key,value) in &self.env { std::env::set_var(key,value); }
    }

    fn configure(&self, enabled: bool, peers: &[String]) -> Result<()> {
        let quote = |path: &Path| serde_json::to_string(&path.to_string_lossy()).unwrap();
        fs::write(&self.config, format!(
            "[node]\nskills_dir={}\np2p_identity_file={}\np2p_enabled={}\n[peers]\nbootstrap={}\n[discovery]\nmdns={}\ndht=false\n",
            quote(&self.skills), quote(&self.identity), enabled, serde_json::to_string(peers)?, enabled
        ))?;
        Ok(())
    }

    fn command(&self, binary: &Path) -> Command {
        let mut command = Command::new(binary);
        command.env_clear().envs(&self.env).current_dir(&self.project);
        command
    }

    fn start(&self, binary: &Path, evidence: &Path, label: &str) -> Result<Process> {
        Ok(Process(self.command(binary).stdin(Stdio::null())
            .stdout(File::create(evidence.join(format!("{label}.stdout")))?)
            .stderr(File::create(evidence.join(format!("{label}.stderr")))?).spawn()?))
    }
}

fn http(socket: &Path, method: &str, route: &str, body: Option<&Value>) -> Result<(u16,Value)> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(Duration::from_secs(12)))?;
    stream.set_write_timeout(Some(Duration::from_secs(12)))?;
    let content = body.map(serde_json::to_vec).transpose()?.unwrap_or_default();
    write!(stream,"{method} {route} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",content.len())?;
    stream.write_all(&content)?;
    let mut bytes=Vec::new(); stream.read_to_end(&mut bytes)?;
    let split=bytes.windows(4).position(|x|x==b"\r\n\r\n").context("HTTP response missing headers")?;
    let headers=std::str::from_utf8(&bytes[..split])?;
    let status=headers.lines().next().context("missing status")?.split_whitespace().nth(1).context("missing status code")?.parse()?;
    let body=&bytes[split+4..];
    let body=if headers.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        let mut offset=0;let mut decoded=Vec::new();
        loop {
            let end=body[offset..].windows(2).position(|x|x==b"\r\n").context("invalid HTTP chunk")?+offset;
            let size=usize::from_str_radix(std::str::from_utf8(&body[offset..end])?.split(';').next().unwrap(),16)?;
            offset=end+2;if size==0 {break;}
            ensure!(offset+size+2<=body.len(),"truncated HTTP chunk");
            decoded.extend_from_slice(&body[offset..offset+size]);offset+=size+2;
        }
        decoded
    } else {body.to_vec()};
    Ok((status,serde_json::from_slice(&body)?))
}

fn wait_ready(host: &Host, process: &mut Process) -> Result<Value> {
    let deadline=Instant::now()+BUDGET;
    while Instant::now()<deadline {
        ensure!(process.0.try_wait()?.is_none(),"headless host exited before authority readiness");
        if let Ok((200,value))=http(&host.socket,"GET","/ready",None) {return Ok(value);}
        std::thread::sleep(Duration::from_millis(100));
    }
    bail!("headless host did not establish actual local authority")
}

struct Mcp {
    process: Process,
    input: ChildStdin,
    responses: Receiver<Value>,
    id: u64,
}

impl Mcp {
    fn start(host:&Host,binary:&Path,evidence:&Path,label:&str)->Result<Self> {
        let mut child=host.command(binary).args(["--mode","mcp","--socket"])
            .arg(&host.socket).stdin(Stdio::piped()).stdout(Stdio::piped())
            .stderr(File::create(evidence.join(format!("{label}.stderr")))?).spawn()?;
        let input=child.stdin.take().context("MCP stdin unavailable")?;
        let output=child.stdout.take().context("MCP stdout unavailable")?;
        let (sender,responses)=mpsc::channel();
        let log=evidence.join(format!("{label}.stdout"));
        std::thread::spawn(move|| {
            let mut log=File::create(log).ok();
            for line in BufReader::new(output).lines().map_while(std::result::Result::ok) {
                if let Some(log)=log.as_mut() {let _=writeln!(log,"{line}");}
                if let Ok(value)=serde_json::from_str(&line) {if sender.send(value).is_err(){break;}}
            }
        });
        let mut client=Self {process:Process(child),input,responses,id:0};
        let initialized=client.request("initialize",json!({"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"ldd-real-process-integration","version":"1"}}))?;
        ensure!(initialized.get("result").is_some(),"actual MCP initialize rejected");
        writeln!(client.input,"{}",json!({"jsonrpc":"2.0","method":"notifications/initialized"}))?;
        client.input.flush()?;
        Ok(client)
    }

    fn request(&mut self,method:&str,params:Value)->Result<Value> {
        self.id+=1;let id=self.id;
        writeln!(self.input,"{}",json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        self.input.flush()?;let deadline=Instant::now()+Duration::from_secs(30);
        loop {
            let remaining=deadline.checked_duration_since(Instant::now()).context("MCP response deadline exceeded")?;
            let value=self.responses.recv_timeout(remaining).context("MCP response unavailable")?;
            if value.get("id")==Some(&json!(id)) {return Ok(value);}
        }
    }

    fn tool(&mut self,name:&str,arguments:Value)->Result<Value> {
        let response=self.request("tools/call",json!({"name":name,"arguments":arguments}))?;
        ensure!(response.get("error").is_none(),"MCP protocol error: {response}");
        let text=response.pointer("/result/content/0/text").and_then(Value::as_str).context("MCP tool omitted text result")?;
        serde_json::from_str(text).with_context(||format!("MCP tool did not return contract JSON: {text}"))
    }
}

fn run() -> Result<()> {
    let scratch=input("LDD_COMPANION_CASE_ROOT")?;
    let evidence=input("LDD_COMPANION_EVIDENCE")?;
    let headless=input("LDD_COMPANION_HEADLESS_BIN")?;
    let sync=input("LDD_COMPANION_SYNC_BIN")?;
    let cli=input("LDD_PROMETHEUS_BIN")?;
    let tools=input("LDD_INTEGRATION_TOOLS")?;
    ensure!(scratch.join(".owned-integration").is_file(),"BLOCKED: fixture lacks wrapper ownership marker");
    let project_id=uuid::Uuid::new_v4().to_string();
    let a=Host::new(scratch.join("a"),&project_id,&tools)?;
    let b=Host::new(scratch.join("b"),&project_id,&tools)?;
    // Real enrolled scratch keys, never OS credential-store access or copied auth.
    let signer_a=ensure_device_key_file(&a.key).context("BLOCKED: scratch file signing backend unavailable")?;
    let signer_b=ensure_device_key_file(&b.key).context("BLOCKED: scratch file signing backend unavailable")?;
    a.select_fixture_identity();
    let runtime_a=Runtime::open_canonical_at(&a.project,&a.data)?;
    let initial=runtime_a.initialize(&project_id,"ldd-connected",Actor::operator("fixture","ldd-integration"))?;
    let enrollment=CommandEnvelope {schema_version:"2".into(),project_id:project_id.clone(),run_id:"ldd-connected".into(),
        command_id:"enroll-b".into(),frontier:Some(initial.frontier.clone()),expected_revision:initial.revision,
        actor:Actor::operator("fixture","ldd-integration"),command:CommandKind::DeviceEnroll {device:DeviceRecord {
            device_id:"b".into(),key_id:signer_b.key_id().into(),public_key:signer_b.public_key().into(),status:DeviceStatus::Active,
            enrolled_at_revision:initial.revision+1,revoked_at_revision:None}}};
    runtime_a.execute_command(enrollment)?;
    let seed=runtime_a.export_project_updates()?;
    b.select_fixture_identity();
    let runtime_b=Runtime::open_canonical_at(&b.project,&b.data)?;
    runtime_b.import_project_updates(&seed)?;
    ensure!(runtime_b.device_signer()?.key_id()==signer_b.key_id(),"scratch B file signer not enrolled");
    let base=runtime_b.replay()?;
    fs::write(evidence.join("canonical-baseline.json"),serde_json::to_vec_pretty(&base)?)?;

    // Actual production pairing CLI shares the group and enrolls both endpoint
    // allowlists. Pairing tickets are private fixture material, never evidence.
    a.configure(true,&[])?;b.configure(true,&[])?;
    let ticket_a=a.command(&sync).args(["--mode","pair-export"]).output()?;
    ensure!(ticket_a.status.success(),"production A pair-export failed");
    let imported=b.command(&sync).args(["--mode","pair-import","--ticket"])
        .arg(String::from_utf8(ticket_a.stdout)?.trim()).output()?;
    ensure!(imported.status.success(),"production B pair-import failed");
    let id_a=serde_json::from_slice::<Value>(&imported.stdout)?["pairedEndpointId"].as_str().context("A endpoint missing")?.to_owned();
    let ticket_b=b.command(&sync).args(["--mode","pair-export"]).output()?;
    ensure!(ticket_b.status.success(),"production B pair-export failed");
    let imported=a.command(&sync).args(["--mode","pair-import","--ticket"])
        .arg(String::from_utf8(ticket_b.stdout)?.trim()).output()?;
    ensure!(imported.status.success(),"production A pair-import failed");
    let id_b=serde_json::from_slice::<Value>(&imported.stdout)?["pairedEndpointId"].as_str().context("B endpoint missing")?.to_owned();
    a.configure(true,std::slice::from_ref(&id_b))?;b.configure(true,std::slice::from_ref(&id_a))?;
    let mut process_a=a.start(&headless,&evidence,"headless-a")?;
    let mut process_b=b.start(&headless,&evidence,"headless-b")?;
    fs::write(evidence.join("ready-a.json"),serde_json::to_vec_pretty(&wait_ready(&a,&mut process_a)?)?)?;
    fs::write(evidence.join("ready-b.json"),serde_json::to_vec_pretty(&wait_ready(&b,&mut process_b)?)?)?;
    let duplicate=a.command(&headless).output()?;
    ensure!(!duplicate.status.success(),"two headless processes acquired the same control socket");
    ensure!(http(&a.socket,"GET","/ready",None)?.0==200,"duplicate socket owner disrupted the original host");
    fs::write(evidence.join("duplicate-owner.stderr"),duplicate.stderr)?;
    let deadline=Instant::now()+BUDGET;
    loop {
        let (_,status_a)=http(&a.socket,"GET","/api/v1/sync/status",None)?;
        let (_,status_b)=http(&b.socket,"GET","/api/v1/sync/status",None)?;
        let has_peer=|status:&Value,id:&str|status["peers"].as_array().is_some_and(|p|p.iter().any(|v|v.as_str()==Some(id)));
        if has_peer(&status_a,&id_b)&&has_peer(&status_b,&id_a) {
            fs::write(evidence.join("transport-a.json"),serde_json::to_vec_pretty(&status_a)?)?;
            fs::write(evidence.join("transport-b.json"),serde_json::to_vec_pretty(&status_b)?)?;
            break;
        }
        ensure!(Instant::now()<deadline,"two real P2P hosts did not connect; readiness alone is not acceptance");
        ensure!(process_a.0.try_wait()?.is_none()&&process_b.0.try_wait()?.is_none(),"P2P host exited");
        std::thread::sleep(Duration::from_millis(150));
    }
    a.select_fixture_identity();
    let before=runtime_a.replay()?;
    let phase=Phase {id:"remote-application".into(),slug:"remote-application".into(),title:"Real signed remote application".into(),
        parent_phase_id:None,status:WorkStatus::Pending,stages:BTreeMap::new(),changes:BTreeMap::new(),legacy_completion_baseline:None,legacy_read_only:false};
    let command=CommandEnvelope {schema_version:"2".into(),project_id:project_id.clone(),run_id:before.run_id.clone(),
        command_id:"real-signed-phase".into(),frontier:Some(before.frontier.clone()),expected_revision:before.revision,
        actor:Actor::operator("operator","ldd-integration"),command:CommandKind::PhaseDefine {phase}};
    let signed=SignedCommandEnvelope::sign(command,&signer_a)?;
    let signed_json=serde_json::to_value(signed)?;
    let mut tampered=signed_json.clone();tampered["signature"]=json!("invalid-signature");
    ensure!(http(&a.socket,"POST",&format!("/api/v1/kbd/projects/{project_id}/commands"),Some(&tampered))?.0>=400,"tampered signed mutation was accepted");
    ensure!(runtime_a.replay()?.revision==before.revision,"rejected mutation changed canonical state");
    let (status,mutation)=http(&a.socket,"POST",&format!("/api/v1/kbd/projects/{project_id}/commands"),Some(&signed_json))?;
    ensure!((200..300).contains(&status),"signed host mutation rejected: {mutation}");
    fs::write(evidence.join("signed-mutation.json"),serde_json::to_vec_pretty(&mutation)?)?;
    let route=format!("/api/v1/kbd/projects/{project_id}/status");
    let (_,state_b)=http(&b.socket,"GET",&route,None)?;
    ensure!(state_b.pointer("/phases/remote-application").is_none(),"negative control: peer already contains the mutation before push");
    let mut mcp=Mcp::start(&a,&sync,&evidence,"mcp-a")?;
    let selected=mcp.tool("sync-status",json!({}))?;
    ensure!(selected["transport"]["nodeId"].as_str()==Some(id_a.as_str()),"MCP reported a transport other than selected headless host");
    let pushed=mcp.tool("sync-push",json!({"domain":format!("kbd-control:{project_id}")}))?;
    fs::write(evidence.join("mcp-push.json"),serde_json::to_vec_pretty(&pushed)?)?;
    ensure!(pushed["status"]!="uncertain"&&pushed["httpStatus"].as_u64().is_some_and(|s|(200..300).contains(&s)),"push did not retain host acceptance: {pushed}");
    let request_id=pushed["requestId"].as_str().context("push ID missing")?;
    let intent_path=PathBuf::from(pushed["intentPath"].as_str().context("pre-POST durable intent missing")?);
    ensure!(intent_path.starts_with(&a.data),"MCP outbox escaped selected data root");
    let intent:Value=serde_json::from_slice(&fs::read(&intent_path)?)?;
    ensure!(intent["signedRequest"]==pushed["signedRequest"]&&intent["endpoint"]==json!(a.socket),"durable intent differs from actual request/endpoint");
    let intent_metadata=fs::symlink_metadata(&intent_path)?;
    ensure!(intent_metadata.is_file()&&!intent_metadata.file_type().is_symlink()&&intent_metadata.nlink()==1&&intent_metadata.permissions().mode()&0o077==0,"pre-POST intent is not a private regular file");
    let receipt=mcp.tool("sync-push-receipt",json!({"request_id":request_id}))?;
    ensure!(receipt["httpStatus"]==200,"original-host durable receipt unavailable");
    fs::write(evidence.join("original-host-receipt.json"),serde_json::to_vec_pretty(&receipt)?)?;
    let deadline=Instant::now()+BUDGET;
    let remote=loop {
        let (_,value)=http(&b.socket,"GET",&route,None)?;
        if value.pointer("/phases/remote-application/title")==Some(&json!("Real signed remote application")) {break value;}
        ensure!(Instant::now()<deadline,"broadcast receipt did not become actual peer application");
        std::thread::sleep(Duration::from_millis(150));
    };
    fs::write(evidence.join("remote-applied-state.json"),serde_json::to_vec_pretty(&remote)?)?;
    b.select_fixture_identity();
    ensure!(runtime_b.replay()?.phases.contains_key("remote-application"),"peer API did not persist canonical application");
    let cli_status=b.command(&cli).args(["kbd","--path"]).arg(&b.project).args(["status","--json"]).output()?;
    ensure!(cli_status.status.success(),"real pack CLI failed on selected Companion socket");
    fs::write(evidence.join("selected-cli.stdout"),cli_status.stdout)?;
    fs::write(evidence.join("selected-cli.stderr"),cli_status.stderr)?;

    // The exact same signed ID/body replays against the real receipt; no new ID.
    let replay=mcp.tool("sync-push",json!({"domain":format!("kbd-control:{project_id}"),"signed_request":pushed["signedRequest"]}))?;
    ensure!(replay["requestId"]==pushed["requestId"]&&replay["httpStatus"]==200,"same-ID replay did not return retained receipt");
    let rejected=mcp.tool("sync-push",json!({"domain":"surreal-memory"}))?;
    ensure!(rejected["httpStatus"].as_u64().is_some_and(|s|s>=400),"local-only memory push was not rejected");
    fs::write(evidence.join("local-memory-negative.json"),serde_json::to_vec_pretty(&rejected)?)?;

    process_a.stop()?;
    let absent=mcp.tool("sync-status",json!({}))?;
    ensure!(absent["status"]=="unavailable","MCP did not report selected-host absence");
    let uncertain=mcp.tool("sync-push",json!({"domain":format!("kbd-control:{project_id}")}))?;
    ensure!(uncertain["status"]=="uncertain"&&uncertain["requestId"].is_string()&&uncertain["intentPath"].is_string(),"absent-host transport failure did not retain durable uncertain request");
    fs::write(evidence.join("absent-host-uncertain.json"),serde_json::to_vec_pretty(&uncertain)?)?;
    ensure!(http(&a.socket,"GET","/health",None).is_err(),"stopped host still accepts work");
    process_b.stop()?;
    ensure!(http(&b.socket,"GET","/health",None).is_err(),"peer shutdown did not stop listener");

    let disabled=Host::new(scratch.join("disabled"),&uuid::Uuid::new_v4().to_string(),&tools)?;
    ensure_device_key_file(&disabled.key)?;
    disabled.select_fixture_identity();
    let runtime=Runtime::open_canonical_at(&disabled.project,&disabled.data)?;
    let id:Value=serde_json::from_slice(&fs::read(disabled.project.join(".prometheus/project.json"))?)?;
    runtime.initialize(id["projectId"].as_str().unwrap(),"disabled",Actor::operator("fixture","ldd-integration"))?;
    disabled.configure(false,&[])?;
    let mut process=disabled.start(&headless,&evidence,"disabled-host")?;
    wait_ready(&disabled,&mut process)?;
    let (_,status)=http(&disabled.socket,"GET","/api/v1/sync/status",None)?;
    ensure!(status["transport"]["state"]=="disabled"&&!disabled.identity.exists(),"disabled P2P created identity/endpoint or reported active transport");
    fs::write(evidence.join("disabled-transport.json"),serde_json::to_vec_pretty(&status)?)?;
    process.stop()?;

    // Standalone pack remains usable with no Companion host/socket at all.
    let offline=disabled.command(&cli).args(["kbd","--path"]).arg(&disabled.project).args(["status","--json"]).output()?;
    ensure!(offline.status.success(),"standalone pack required an optional Companion host");
    fs::write(evidence.join("standalone-pack.stdout"),offline.stdout)?;
    fs::write(evidence.join("standalone-pack.stderr"),offline.stderr)?;
    // Explicitly retain references until scope exit; Drop closes the MCP child.
    ensure!(mcp.process.0.try_wait()?.is_none(),"MCP process exited unexpectedly");
    fs::write(evidence.join("connected-acceptance.json"),serde_json::to_vec_pretty(&json!({
        "acceptance":["14/AC-1"],"projectId":project_id,"endpointIds":[id_a,id_b],"requestId":request_id,
        "remoteApplication":true,"selectedSocket":a.socket,"disabledIdentityAbsent":true,
        "publication":"pending; not established by this scenario"
    }))?)?;
    Ok(())
}

#[test]
fn actual_two_headless_hosts_and_selected_mcp_apply_signed_state() {
    if let Err(error)=run() {panic!("{error:#}");}
}
