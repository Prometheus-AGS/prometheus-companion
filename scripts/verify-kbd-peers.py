#!/usr/bin/env python3
"""Exercise signed project replication using installed binaries, local mDNS and Unix sockets.

Python 3.9+. Run with --help for binary paths. The report contains public identities
and observations. Disposable pairing tickets and private key files stay in the fixture.
"""
import argparse
import errno
from concurrent.futures import ThreadPoolExecutor
from http.server import BaseHTTPRequestHandler
import queue
import re
import socketserver
import sys
import threading
from contextlib import ExitStack, closing
import hashlib
import http.client
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import tempfile
import time
import uuid


ASSERTIONS = 0
PROCESSES = []
INTERRUPTED_PIDS = set()
PUSHES = "/api/v2/sync/pushes"
PEER_CASES = ("PEER-001", "PEER-002", "PEER-003", "PEER-004")
FULL_RELEASE = "acd029107533c07a661a7e3ad90eb28834688afa"
HISTORICAL_JOURNAL = "substrate/kbd-runtime/tests/fixtures/pre-192-journal/events.jsonl"


class Blocked(RuntimeError):
    pass


def require(condition, message):
    global ASSERTIONS
    ASSERTIONS += 1
    if not condition:
        raise RuntimeError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    # The controlled fixture uses ASCII strings, integers and booleans only.
    return json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


class UnixHTTP(http.client.HTTPConnection):
    def __init__(self, path, timeout):
        super().__init__("localhost", timeout=timeout)
        self.path = path

    def connect(self):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.settimeout(self.timeout)
        self.sock.connect(str(self.path))


class Device:
    def __init__(self, root, name, project_id, args):
        self.name = name
        self.root = root / name
        self.checkout = self.root / "project"
        self.project_id = project_id
        self.route = f"/api/v1/kbd/projects/{project_id}"
        self.socket = self.root / "control.sock"
        self.snapshot = self.root / "data/prometheus/kbd/projects" / project_id / "project.loro"
        self.args = args
        self.process = None
        self.receiver_file_check = None
        self.pids = []
        self.log = self.root / "headless.log"
        for directory in ("home", "config", "data", "state", "cache", "tmp", "skills",
                          "project/.prometheus", "project/.kbd-orchestrator"):
            (self.root / directory).mkdir(parents=True, exist_ok=True)
        config = self.root / "config/config.toml"
        config.write_text(f'[node]\np2p_identity_file = {json.dumps(str(self.root / "config/p2p.json"))}\n'
                          '[peers]\nlocal_only = true\n[discovery]\nmdns = true\ndht = false\n')
        self.env = {"HOME": str(self.root / "home"), "PATH": os.defpath, "NO_PROXY": "*",
                    "SOVEREIGN_SYNC_SOCKET": str(self.socket), "SOVEREIGN_SYNC_CONFIG": str(config),
                    "PROMETHEUS_DEVICE_KEY_FILE": str(config.parent / "device-key.json"),
                    "PROMETHEUS_SKILLS_DIR": str(self.root / "skills"),
                    "PROMETHEUS_DATA_DIR": str(self.root / "data"), "TMPDIR": str(self.root / "tmp"),
                    "RUST_LOG": "sovereign_sync=info,prometheus_substrate=info", "NO_COLOR": "1"}
        self.env.update({f"XDG_{d.upper()}_HOME": str(self.root / d)
                         for d in ("config", "data", "state", "cache")})
        manifest = {"schemaVersion": "1", "projectId": project_id,
                    "repositoryFingerprint": "sha256:" + hashlib.sha256(project_id.encode()).hexdigest()}
        (self.checkout / ".prometheus/project.json").write_text(json.dumps(manifest))
        identities = [json.loads(line) for line in self.sync("init").splitlines()]
        self.key = next(row["keyId"] for row in identities if "keyId" in row)
        self.endpoint = next(row["endpointId"] for row in identities if "endpointId" in row)
        self.replica = json.loads(self.kbd("register", "."))["registration"]["replicaId"]

    def run(self, program, argv, env=None):
        result = subprocess.run([program, *argv], cwd=self.checkout, env=self.env | (env or {}),
                                capture_output=True, timeout=self.args.timeout, stdin=subprocess.DEVNULL)
        if result.returncode:
            # pair-import accepts its secret in argv. Never echo that argv or its diagnostics.
            detail = "" if "--ticket" in argv else result.stderr.decode(errors="replace").strip()[-700:]
            raise RuntimeError(f"{self.name} {program.name} {argv[:2]} exited {result.returncode}: {detail}")
        return result.stdout

    def kbd(self, *args, control=None):
        remote = {"PROMETHEUS_KBD_CONTROL_PLANE": "1", "SOVEREIGN_SYNC_SOCKET": str(control)} if control else {}
        return self.run(self.args.pack_cli, ["kbd", *args], remote)

    def sync(self, mode, *args):
        return self.run(self.args.sync_cli, ["--mode", mode, *args])

    def fixture(self, *argv, data=b""):
        if not self.args.fixture_tool:
            raise Blocked("the locked-build kbd-peer-fixture executable is missing")
        result = subprocess.run([self.args.fixture_tool, *argv], cwd=self.checkout,
                                env=self.env, input=data, capture_output=True, timeout=self.args.timeout)
        if result.returncode:
            raise RuntimeError(f"{self.name} fixture {argv[0]} exited {result.returncode}: "
                               + result.stderr.decode(errors="replace").strip()[-700:])
        return result.stdout

    def start(self, stack):
        with self.log.open("ab") as logfile:
            self.process = subprocess.Popen([self.receiver_file_check or self.args.companion],
                                            cwd=self.checkout, env=self.env,
                                            stdin=subprocess.PIPE if self.receiver_file_check else subprocess.DEVNULL,
                                            stdout=logfile, stderr=subprocess.STDOUT)
        process = self.process
        if process.stdin is not None:
            stack.callback(process.stdin.close)
        self.pids.append(process.pid)
        PROCESSES.append((self.name, process))
        stack.callback(self.stop_process, process)

    @staticmethod
    def stop_process(process):
        if process.poll() is None:
            process.send_signal(signal.SIGINT)
            try:
                process.wait(timeout=6)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=4)

    def stop(self):
        self.stop_process(self.process)

    def kill(self):
        require(self.process.poll() is None, "interruption target is no longer live")
        INTERRUPTED_PIDS.add(self.process.pid)
        self.process.kill()
        self.process.wait(timeout=4)

    def logs(self):
        return self.log.read_text(errors="replace") if self.log.exists() else ""

    def describe(self):
        return {"name": self.name, "keyId": self.key, "endpointId": self.endpoint,
                "projectId": self.project_id, "replicaId": self.replica,
                "checkout": str(self.checkout), "authority": str(self.snapshot.parent),
                "socket": str(self.socket), "companionPids": self.pids}

    def journal(self):
        return self.snapshot.parent / "replicas" / self.replica / "events.jsonl"

    def waypoint(self):
        return self.checkout / ".kbd-orchestrator/current-waypoint.json"

    def all_audit(self):
        return [json.loads(line) for line in self.request(self.route + "/audit").splitlines() if line.strip()]

    def signed_push(self, endpoint):
        return self.sync("sign-kbd-push", "--project-id", self.project_id, "--peer", endpoint)

    def status(self):
        return json.loads(self.request("/api/v1/sync/status"))

    def exchange(self, route, body=None):
        with closing(UnixHTTP(self.socket, self.args.timeout)) as connection:
            connection.request("GET" if body is None else "POST", route, body,
                               {"Content-Type": "application/json"} if body is not None else {})
            response = connection.getresponse()
            return response.status, response.read()

    def request(self, route, body=None, expect=200):
        status, raw = self.exchange(route, body)
        require(status == expect, f"{self.name} {route} returned {status}, expected {expect}: "
                + raw[:400].decode(errors="replace"))
        return raw

    def ready(self):
        with closing(UnixHTTP(self.socket, self.args.timeout)) as connection:
            connection.request("GET", "/ready")
            return connection.getresponse().status == 200

    def state(self):
        return json.loads(self.request(self.route + "/status"))

    def audit(self, command):
        return [row for row in self.all_audit() if row.get("commandId") == command]

    def peers(self):
        return json.loads(self.request("/api/v1/sync/peers"))["peers"]


def wait_for(description, predicate, nodes, timeout):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        for node in nodes:
            if node.process.poll() is not None:
                # Logs stay private in scratch. Include a bounded diagnostic from the executable.
                lines = (node.root / "headless.log").read_text(errors="replace").splitlines()
                raise RuntimeError(f"{node.name} exited {node.process.returncode}: " + " ".join(lines[-2:])[-500:])
        try:
            value = predicate()
            if value:
                return value
        except (OSError, http.client.HTTPException) as error:
            last = str(error)
        time.sleep(0.2)
    raise TimeoutError(f"timed out waiting for {description}" + (f": {last}" if last else ""))


def pair(source, target):
    ticket = source.sync("pair-export").decode().strip()
    imported = json.loads(target.sync("pair-import", "--ticket", ticket))
    require(imported["pairedEndpointId"] == source.endpoint, "imported endpoint differs from its ticket issuer")


def observe_stream(response, command, timeout):
    deadline = time.monotonic() + timeout
    event_type = ""
    while time.monotonic() < deadline:
        line = response.readline()
        require(line, "peer event stream closed before the signed event")
        if line.startswith(b"event:"):
            event_type = line[6:].strip().decode()
        if line.startswith(b"data:") and event_type == "kbd.events":
            for event in json.loads(line[5:]):
                if event["commandId"] == command:
                    return event
    raise TimeoutError("peer event stream never reported the command")


def exercise(root, args, report, stack):
    project = str(uuid.uuid4())
    a, b, c = nodes = [Device(root, label, pid, args) for label, pid in
                       (("A", project), ("B", project), ("C", str(uuid.uuid4())))]
    report["devices"] = {n.name: {"endpointId": n.endpoint, "keyId": n.key, "projectId": n.project_id,
                                  "replicaId": n.replica} for n in nodes}
    for field in ("endpoint", "key", "replica"):
        require(len({getattr(n, field) for n in nodes}) == 3, f"fixture reuses a {field}")

    # Initialize one signed history once. Only the enrolled, common baseline is provisioned offline.
    # Each registered checkout has an independent canonical store, replica ID and signing key.
    b_key = json.loads(b.kbd("device", "identity", "--json"))
    a.kbd("device", "enroll", "--command-id", "enroll-peer-b", "--device-id", "peer-b",
          "--public-key", b_key["publicKey"], "--operator")
    b.snapshot.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(a.snapshot, b.snapshot)
    c.kbd("phase", "create", "--id", "private-c", "--command-id", "initialize-c", "--title", "C only")
    pair(a, b)
    pair(b, a)
    # C knows A's ticket but A has never enrolled C. C owns a different project.
    pair(a, c)
    for node in nodes:
        node.start(stack)
    wait_for("all three control sockets", lambda: all(n.ready() for n in nodes), nodes, args.timeout)
    wait_for("reciprocal mDNS discovery", lambda: b.endpoint in a.peers() and a.endpoint in b.peers(),
             nodes, args.timeout)
    require(c.endpoint not in a.peers() and c.endpoint not in b.peers(), "unpaired endpoint entered peer list")
    report["mutualMdnsPeers"] = True
    a_seed, b_seed, c_seed = (n.state() for n in nodes)
    require(a_seed["frontier"] and a_seed["frontier"] == b_seed["frontier"], "initial frontiers differ")

    command = str(uuid.uuid4())
    stream = stack.enter_context(closing(UnixHTTP(b.socket, args.timeout)))
    stream.request("GET", b.route + "/events/stream", headers={"Last-Event-ID": str(b_seed["revision"])})
    response = stream.getresponse()
    require(response.status == 200 and "text/event-stream" in response.getheader("Content-Type", ""),
            "expected a live KBD event stream")
    # Execute with B's enrolled operator identity and local data, addressing A's actual control socket.
    # A changing while B stays at its seed rules out a successful local-only fallback on the caller.
    b.kbd("phase", "create", "--id", "peer-verification", "--command-id", command,
          "--title", "Peer verification", control=a.socket)
    accepted = a.audit(command)
    require(len(accepted) == 1 and accepted[0].get("signature"), "A did not record one signed CLI command")
    require(not b.audit(command) and b.state()["frontier"] == b_seed["frontier"], "B changed before the push")
    report["command"] = {"commandId": command, "eventId": accepted[0]["eventId"],
                         "integrityHash": accepted[0]["integrityHash"], "signerKeyId": accepted[0]["signerKeyId"],
                         "cliOrigin": "B", "controlSocketOwner": "A", "absentFromBBeforePush": True}
    signed = a.sync("sign-kbd-push", "--project-id", project, "--peer", b.endpoint)
    request = json.loads(signed)
    require(request["targetEndpointIds"] == [b.endpoint] and request["signerKeyId"] == a.key,
            "signed request lost target or used unexpected operator key")
    pushes = "/api/v2/sync/pushes"
    receipt = json.loads(a.request(pushes, signed, expect=201))
    require(receipt["localState"] == "broadcast" and receipt["perPeer"][b.endpoint]["state"] == "applied",
            "push did not reach the paired transport")
    report["signedRequest"] = request
    report["deliveryReceipt"] = receipt
    event = observe_stream(response, command, args.timeout)
    require(event == accepted[0], "SSE event differs from signed sender event")
    require(b.audit(command) == accepted, "B audit differs from A after push")
    sender, receiver = a.state(), b.state()
    require(sender["revision"] > a_seed["revision"] and sender["revision"] == receiver["revision"]
            and sender["frontier"] and sender["frontier"] == receiver["frontier"], "frontiers did not converge")
    waypoint = json.loads((b.checkout / ".kbd-orchestrator/current-waypoint.json").read_text())
    require(waypoint["sourceRevision"] == receiver["revision"] and waypoint["frontier"] == receiver["frontier"],
            "B's on-disk compatibility projection did not advance")
    replay = json.loads(a.request(pushes, signed, expect=200))
    require(replay == receipt and a.audit(command) == b.audit(command) == accepted,
            "identical request did not replay the original result exactly once")
    rejected_request = a.sync("sign-kbd-push", "--project-id", project, "--peer", c.endpoint)
    rejected = json.loads(a.request(pushes, rejected_request, expect=503))
    failed_id = json.loads(rejected_request)["requestId"]
    failed = json.loads(a.request(pushes + "/" + failed_id))
    require(failed["localState"] == "failed" and failed["perPeer"][c.endpoint]["state"] == "failed",
            "unpaired destination did not produce a failed delivery receipt")
    report["rejectedDelivery"] = {"http": 503, "response": rejected, "receipt": failed}
    require(c.state()["frontier"] == c_seed["frontier"] and not c.audit(command),
            "unpaired peer's independent project changed")
    c.request(a.route + "/audit", expect=404)
    require(c.endpoint not in a.peers() and c.endpoint not in b.peers(), "unpaired peer was admitted")
    report["push"] = {"pushId": receipt["pushId"], "target": b.endpoint, "initialHttp": 201, "retryHttp": 200,
                      "receiptReplayed": True, "eventCountAtEachPeer": len(accepted)}
    report["convergence"] = {"revision": receiver["revision"], "frontier": receiver["frontier"],
                             "signedEventMatchedAcrossAuditAndSse": True, "projectionMatched": True}
    report["unpaired"] = {"projectAuditHttp": 404, "ownFrontierUnchanged": True, "absentFromPairedLists": True}



def enroll(operator, peer, command=None, pack=None):
    identity = json.loads(peer.kbd("device", "identity", "--json"))
    require(identity["keyId"] == peer.key, "fixture CLI resolved the wrong device key")
    argv = ["kbd", "device", "enroll", "--command-id", command or uuid.uuid4().hex,
            "--device-id", f"peer-{peer.name}", "--public-key", identity["publicKey"], "--operator"]
    return operator.run(pack or operator.args.pack_cli, argv)


def provision(source, receiver):
    require(source.project_id == receiver.project_id, "trusted baseline project IDs differ")
    receiver.snapshot.parent.mkdir(parents=True, exist_ok=True)
    require(not receiver.snapshot.exists(), "bootstrap would overwrite receiving authority")
    with source.snapshot.open("rb") as source_file, receiver.snapshot.open("xb") as destination:
        os.chmod(receiver.snapshot, 0o600)
        shutil.copyfileobj(source_file, destination)
    original, installed = sha256(source.snapshot), sha256(receiver.snapshot)
    require(original == installed, "signed bootstrap snapshot changed during transfer")
    return {"senderSha256": original, "receiverSha256": installed}


def fixtures(root, args):
    project = str(uuid.uuid4())
    return Device(root, "A", project, args), Device(root, "B", project, args)


def pair_both(a, b):
    pair(a, b)
    pair(b, a)


def record_nodes(report, *nodes):
    report.setdefault("authorities", {}).update({node.name: node.describe() for node in nodes})
    for field in ("endpoint", "key", "replica", "socket"):
        require(len({getattr(node, field) for node in nodes}) == len(nodes), f"fixture shares {field}")


def ready_nodes(stack, *nodes):
    for node in nodes:
        node.start(stack)
    try:
        wait_for("independent node readiness", lambda: all(node.ready() for node in nodes),
                 nodes, max(node.args.timeout for node in nodes))
    except TimeoutError as error:
        logs = "\n".join(node.logs()[-1000:] for node in nodes)
        raise RuntimeError(f"{error}: {logs}") from error


def require_paired(a, b):
    try:
        wait_for("paired same-host transport", lambda: b.endpoint in a.peers() and a.endpoint in b.peers(),
                 (a, b), max(a.args.timeout, b.args.timeout))
    except TimeoutError as error:
        raise Blocked(f"{error}: local mDNS or paired endpoint is unavailable") from error


def start_pair(a, b, report, stack):
    report["baselineCopy"] = provision(a, b)
    pair_both(a, b)
    ready_nodes(stack, a, b)
    record_nodes(report, a, b)
    require_paired(a, b)
    a_seed, b_seed = a.state(), b.state()
    require(a_seed["frontier"] and a_seed["frontier"] == b_seed["frontier"], "enrolled baseline frontiers differ")
    require(a.all_audit() == b.all_audit(), "trusted baseline audit differs between authorities")
    report["baseline"] = {"frontier": b_seed["frontier"], "eventHashes": event_hashes(b.all_audit())}
    return a_seed, b_seed


def event_hashes(events):
    return [event["integrityHash"] for event in events]


def phase_args(command_id, name, title=None):
    return ["phase", "create", "--id", name, "--command-id", command_id, "--title", title or name]


def assert_waypoint(node, state):
    waypoint_path = node.waypoint()
    waypoint = json.loads(waypoint_path.read_bytes())
    require(waypoint["sourceRevision"] == state["revision"] and waypoint["frontier"] == state["frontier"],
            f"{node.name}'s checkout waypoint is stale")
    return {"sha256": sha256(waypoint_path), "sourceRevision": waypoint["sourceRevision"],
            "frontier": waypoint["frontier"]}


def assert_applied(sender, receiver, command_id, signed):
    receipt = json.loads(sender.request(PUSHES, signed, expect=201))
    delivery = receipt["perPeer"][receiver.endpoint]
    require(receipt["localState"] == "broadcast" and delivery["state"] == "applied",
            "addressed push did not record receiver application")
    require(receipt["targets"] == [receiver.endpoint], "receipt does not identify the addressed receiver")
    accepted, observed = sender.audit(command_id), receiver.audit(command_id)
    require(len(accepted) == 1 and accepted == observed,
            "independent audits do not show exactly one matching signed event")
    require(accepted[0].get("signature") and accepted[0].get("integrityHash"), "event is missing signed integrity")
    return receipt, accepted[0]


def check_readiness_diagnostic(node, expected):
    def failed():
        body = json.loads(node.request("/ready", expect=503))
        if body.get("startup", {}).get("stage") == "failed":
            return body
        return None

    ready = wait_for("candidate reader rejection", failed, (node,), node.args.timeout)
    json.loads(node.request("/health"))
    diagnostic = " ".join(line for line in node.logs().splitlines() if expected in line)
    require(diagnostic, f"candidate diagnostic did not identify the unsupported event {expected}")
    return ready, diagnostic[-1600:]


def future_reader_case(a, b, baseline, b_before, report, stack):
    # Produce the exact next causal event through the real writer, then
    # substitute a future kind and sign its canonical preimage.
    a.kbd(*phase_args("future-event-fixture", "future-event-fixture"))
    event = json.loads(a.journal().read_bytes().splitlines()[-1])
    future_kind = "future_peer_boundary_unsupported"
    require(event["previousHash"] == baseline[-1]["integrityHash"]
            and event["lamport"] == baseline[-1]["lamport"] + 1,
            "fixture event does not extend the verified baseline chain")
    require(event["signerKeyId"] == a.key and event["signerPublicKey"], "fixture signer is not enrolled")
    event["kind"]["type"] = future_kind
    event.pop("signature")
    event["integrityHash"] = ""
    preimage = canonical(event)
    signature = a.fixture("sign-bytes", data=preimage).decode().strip()
    event["integrityHash"] = digest(preimage)
    event["signature"] = signature
    b.fixture("verify-bytes", event["signerPublicKey"], signature, data=preimage)
    previous = None
    for original in baseline:
        unsigned = dict(original, integrityHash="")
        b.fixture("verify-bytes", original["signerPublicKey"], unsigned.pop("signature"), data=canonical(unsigned))
        require(digest(canonical(unsigned)) == original["integrityHash"]
                and original["previousHash"] == previous, "baseline chain is invalid")
        previous = original["integrityHash"]
    b.journal().parent.mkdir(parents=True, exist_ok=True)
    require(not b.journal().exists(), "future fixture journal would overwrite receiver history")
    # Complete JSON without a final newline exercises the candidate's
    # non-destructive branch ahead of interrupted-append recovery.
    b.journal().write_bytes(canonical(event))
    os.chmod(b.journal(), 0o600)
    future = b.journal().read_bytes()
    b.start(stack)
    readiness, diagnostic = check_readiness_diagnostic(b, future_kind)
    b.request(b.route + "/audit", expect=503)
    require(b.journal().read_bytes() == future and sha256(b.snapshot) == b_before,
            "reader changed signed history while rejecting a future event")
    report["futureEvent"] = {"kind": future_kind, "signerKeyId": a.key,
                             "eventHash": event["integrityHash"], "recordSha256": digest(future),
                             "baselineHashes": event_hashes(baseline), "eventCount": len(baseline) + 1,
                             "bytesPreserved": True, "readiness": readiness, "diagnostic": diagnostic}


def peer_reader(root, args, report, stack):
    a, b = fixtures(root, args)
    if args.installed_pack_cli is None:
        raise Blocked("KBD_INTERSESSION_INSTALLED_CLI is required for the installed writer compatibility case")
    enroll(a, b, "enroll-co-operator-b", pack=args.installed_pack_cli)
    baseline_raw = a.journal().read_bytes()
    baseline = [json.loads(line) for line in baseline_raw.splitlines() if line.strip()]
    grants = [row for row in baseline if row.get("kind", {}).get("type") == "operator_granted"]
    require(len(grants) == 1 and grants[0]["signerKeyId"] == a.key and grants[0]["signature"]
            and grants[0]["kind"]["payload"]["key_id"] == b.key,
            "the supported installed writer did not produce B's signed co-operator grant")
    report["baselineCopy"] = provision(a, b)
    report["fixtureInputSha256"] = digest(baseline_raw)
    record_nodes(report, a, b)
    a_before, b_before = sha256(a.snapshot), sha256(b.snapshot)
    if args.negative_only:
        future_reader_case(a, b, baseline, b_before, report, stack)
        return

    ready_nodes(stack, a, b)
    require(sha256(a.snapshot) == a_before and sha256(b.snapshot) == b_before
            and a.journal().read_bytes() == baseline_raw, "read-only startup changed a signed baseline")
    a_state, b_state = a.state(), b.state()
    require(a.all_audit() == b.all_audit() == baseline, "enrolled audit lost a signed event")
    require(a_state["frontier"] and a_state["frontier"] == b_state["frontier"], "reader frontiers differ")
    for state in (a_state, b_state):
        require(all(state["devices"][key]["status"] == "active" for key in (a.key, b.key)),
                "reader did not retain both enrolled keys")
    report["baseline"] = {"frontier": b_state["frontier"], "eventHashes": event_hashes(baseline),
                          "grant": grants[0]["integrityHash"], "snapshotBytesPreserved": True}
    source = f"{FULL_RELEASE}:{HISTORICAL_JOURNAL}"
    result = subprocess.run(["git", "-C", str(args.pack_root), "show", source],
                            capture_output=True, timeout=args.timeout)
    if result.returncode or not result.stdout.strip():
        raise Blocked("authentic legacy journal from the Full v1.17.0 source is unavailable")
    history = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    status_path = str(Path(HISTORICAL_JOURNAL).with_name("status-prometheus-1.13.0.json"))
    status_bytes = subprocess.check_output(["git", "-C", str(args.pack_root), "show",
                                            f"{FULL_RELEASE}:{status_path}"], timeout=args.timeout)
    historical_status = json.loads(status_bytes)
    old = Device(root, "H", historical_status["projectId"], args)
    require(history[0]["kind"]["type"] == "run_initialized" and len(history) == historical_status["revision"],
            "historical journal does not span initialization through the recorded final revision")
    frontier = {}
    previous = None
    for revision, event in enumerate(history, 1):
        replica = event["replicaId"]
        require(event["projectId"] == old.project_id and event["runId"] == historical_status["runId"]
                and event["revision"] == revision and event["previousHash"] == previous
                and event["lamport"] == frontier.get(replica, 0) + 1
                and event.get("frontier", {}) == frontier, "historical journal has missing ancestry")
        unsigned = dict(event, integrityHash="")
        signature = unsigned.pop("signature")
        encoded = canonical(unsigned)
        require(digest(encoded) == event["integrityHash"], "historical event hash mismatch")
        old.fixture("verify-bytes", event["signerPublicKey"], signature, data=encoded)
        previous = event["integrityHash"]
        frontier[replica] = event["lamport"]
    require(frontier == historical_status["frontier"], "historical journal ends at an unexpected frontier")
    old.journal().parent.mkdir(parents=True, exist_ok=True)
    old.journal().write_bytes(result.stdout)
    os.chmod(old.journal(), 0o600)
    ready_nodes(stack, old)
    record_nodes(report, old)
    require(old.all_audit() == history and old.journal().read_bytes() == result.stdout,
            "candidate did not preserve authentic legacy signed events")
    state = old.state()
    require(all(state[field] == historical_status[field] for field in ("projectId", "runId", "revision", "frontier")),
            "candidate replay differs from the historical writer's recorded state")
    report["historical"] = {"sourceRelease": FULL_RELEASE, "path": HISTORICAL_JOURNAL,
                            "fixtureInputSha256": digest(result.stdout), "eventHashes": event_hashes(history),
                            "writerStatusPath": status_path, "writerStatusSha256": digest(status_bytes),
                            "verifiedSignatures": len(history), "revision": state["revision"], "frontier": state["frontier"]}


class ForwardedPost(socketserver.UnixStreamServer):
    def __init__(self, path, device, drop_response=False):
        self.device = device
        self.drop_response = drop_response
        self.suppressed = False
        self.observations = queue.Queue()
        super().__init__(str(path), PostRelayHandler)

    def handle_error(self, request, client_address):
        self.observations.put({"exception": "local socket relay failed"})


class PostRelayHandler(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_GET(self):
        if self.server.suppressed and self.path.endswith("/audit"):
            self.send_error(503, "audit deliberately disconnected after accepted submission")
            return
        self.forward()

    def do_POST(self):
        self.forward()

    def forward(self):
        routed = self.server
        raw = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        with closing(UnixHTTP(routed.device.socket, routed.device.args.timeout)) as connection:
            headers = {"Content-Type": "application/json"} if self.command == "POST" else {}
            connection.request(self.command, self.path, raw if self.command == "POST" else None, headers)
            response = connection.getresponse()
            result = response.read()
            if self.command == "POST":
                routed.observations.put({
                    "method": "POST", "path": self.path, "acceptedHttp": response.status,
                    "targetSocket": str(routed.device.socket), "requestSha256": digest(raw),
                    "responseSha256": digest(result), "acceptedAtNs": time.monotonic_ns(),
                    "signedEnvelope": json.loads(raw), "responseSuppressed": routed.drop_response,
                })
                if routed.drop_response:
                    routed.suppressed = True
                    self.close_connection = True
                    return
            self.send_response(response.status)
            self.send_header("Content-Type", response.getheader("Content-Type", "application/json"))
            self.send_header("Content-Length", str(len(result)))
            self.end_headers()
            self.wfile.write(result)


def start_relay(root, stack, device, drop_response=False):
    path = root / ("lost.sock" if drop_response else "forward.sock")
    relay = stack.enter_context(ForwardedPost(path, device, drop_response))
    thread = threading.Thread(target=relay.serve_forever, daemon=True)
    thread.start()

    def close():
        relay.shutdown()
        thread.join(timeout=3)
        require(not thread.is_alive(), "socket relay did not stop")
        path.unlink(missing_ok=True)

    stack.callback(close)
    return path, relay


def present_post(node, observation, command_id):
    require(observation.get("acceptedHttp") == 200 and observation.get("method") == "POST"
            and observation.get("path") == node.route + "/commands",
            "originating command did not receive an accepted response on A's actual control socket")
    signed = observation.pop("signedEnvelope")
    require(signed["command"]["commandId"] == command_id and signed["signerKeyId"] == node.key,
            "accepted request does not bind the originating session signer and command ID")
    public = json.loads(node.kbd("device", "identity", "--json"))["publicKey"]
    node.fixture("verify-bytes", public, signed["signature"],
                 data=canonical(signed["command"]) + bytes(1) + node.key.encode())
    return observation


def emit(value):
    print(json.dumps(value), flush=True)


def session_worker():
    init = json.loads(sys.stdin.readline())
    pid = os.getpid()
    if init["role"] == "sender":
        emit({"ready": True, "pid": pid, "atNs": time.monotonic_ns()})
        for line in sys.stdin:
            request = json.loads(line)
            env = os.environ.copy()
            if request.get("socket"):
                env.update({"PROMETHEUS_KBD_CONTROL_PLANE": "1", "SOVEREIGN_SYNC_SOCKET": request["socket"]})
            with subprocess.Popen([init["pack"], "kbd", *request["argv"]], env=env,
                                  stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as cli:
                out, err = cli.communicate()
                emit({"pid": pid, "cliPid": cli.pid, "exit": cli.returncode,
                      "outputSha256": digest(out), "diagnostic": err.decode(errors="replace").strip()[-900:],
                      "atNs": time.monotonic_ns()})
        return
    with closing(UnixHTTP(init["socket"], init["timeout"])) as connection:
        connection.request("GET", init["route"] + "/events/stream",
                           headers={"Last-Event-ID": str(init["seedRevision"])})
        response = connection.getresponse()
        require(response.status == 200 and "text/event-stream" in response.getheader("Content-Type", ""),
                "receiver session failed to subscribe to its own authority")
        event_type, cursor, announced = "", None, False
        while line := response.readline():
            if line.startswith(b"event:"):
                event_type = line[6:].strip().decode()
            elif line.startswith(b"id:"):
                cursor = line[3:].strip().decode()
            elif line.startswith(b"data:") and event_type == "kbd.events":
                events = json.loads(line[5:])
                if not announced:
                    emit({"ready": True, "pid": pid, "socket": init["socket"], "seedCursor": cursor,
                          "initialEventCount": len(events), "atNs": time.monotonic_ns()})
                    announced = True
                for event in events:
                    if event.get("commandId") == init["commandId"]:
                        emit({"pid": pid, "socket": init["socket"], "signedEvent": event,
                              "cursor": cursor, "atNs": time.monotonic_ns()})


class LiveSession:
    def __init__(self, role, device, stack, **options):
        self.role = role
        self.device = device
        self.messages = queue.Queue()
        self.closed = False
        self.process = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "--session-worker"],
                                        cwd=device.checkout, env=device.env, stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1,
                                        start_new_session=True)
        self.send({"role": role, "pack": str(device.args.pack_cli), "socket": str(device.socket),
                   "route": device.route, "timeout": device.args.timeout, **options})
        PROCESSES.append((f"{device.name}-{role}-session", self.process))
        self.reader = threading.Thread(target=self.read_messages, daemon=True)
        self.reader.start()
        stack.callback(self.stop)
        self.ready = self.next()
        require(self.ready["ready"] and self.ready["pid"] == self.process.pid,
                f"session {role} did not confirm an independent running PID")
        self.session_id = uuid.uuid4().hex

    def send(self, request):
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()

    def read_messages(self):
        try:
            for line in self.process.stdout:
                self.messages.put(json.loads(line))
        finally:
            self.messages.put(None)

    def next(self, timeout=None):
        try:
            value = self.messages.get(timeout=timeout or self.device.args.timeout)
        except queue.Empty as error:
            raise RuntimeError(f"live {self.role} session exceeded its observation deadline") from error
        if value is None:
            detail = self.process.stderr.read().strip()[-800:]
            raise RuntimeError(f"live {self.role} session exited early: {detail}")
        return value

    def describe(self):
        return {"sessionId": self.session_id, **self.ready, "checkout": str(self.device.checkout),
                "authority": str(self.device.snapshot.parent)}

    def stop(self):
        if self.closed:
            return
        if self.process.poll() is None:
            os.killpg(self.process.pid, signal.SIGTERM)
            try:
                self.process.wait(timeout=4)
            except subprocess.TimeoutExpired:
                os.killpg(self.process.pid, signal.SIGKILL)
                self.process.wait(timeout=4)
        self.reader.join(timeout=3)
        require(not self.reader.is_alive(), f"live {self.role} session did not close")
        for stream in (self.process.stdin, self.process.stdout, self.process.stderr):
            stream.close()
        self.closed = True


def peer_delivery(root, args, report, stack):
    a, b = fixtures(root, args)
    enroll(a, b)
    _, seed = start_pair(a, b, report, stack)
    command = uuid.uuid4().hex
    origin = LiveSession("sender", a, stack)
    observer = LiveSession("receiver", b, stack, commandId=command, seedRevision=seed["revision"])
    report["sessions"] = {"A": origin.describe(), "B": observer.describe()}
    require(origin.process.pid != observer.process.pid and origin.process.poll() is None
            and observer.process.poll() is None, "the two original sessions were not concurrently alive")
    require(observer.ready["seedCursor"] == str(seed["revision"]) and observer.ready["initialEventCount"] == 0,
            "B's independent listener did not acknowledge the seed frontier")
    forward_socket, relay = start_relay(root, stack, a)
    origin.send({"argv": phase_args(command, "through-a-actual-control"), "socket": str(forward_socket)})
    cli = origin.next()
    require(cli["exit"] == 0 and cli["pid"] == origin.process.pid
            and cli["cliPid"] != origin.process.pid, "normal writer failed in session A")
    observed_post = present_post(a, relay.observations.get(timeout=args.timeout), command)
    require(observed_post["acceptedAtNs"] > observer.ready["atNs"], "B subscribe did not precede A's accepted POST")
    accepted = a.audit(command)
    require(len(accepted) == 1 and accepted[0]["signerKeyId"] == a.key, "A's signed event differs from the originating key")
    require(not b.audit(command) and b.state()["frontier"] == seed["frontier"]
            and observer.process.poll() is None, "B changed without an explicit signed push")
    report["origin"] = {**observed_post, "cli": cli, "event": accepted[0]["eventId"],
                        "hash": accepted[0]["integrityHash"], "bAbsentBeforePush": True}
    if args.negative_only:
        try:
            unexpected = observer.messages.get(timeout=2.5)
        except queue.Empty:
            unexpected = "absent"
        require(unexpected == "absent" and observer.process.poll() is None and not b.audit(command)
                and b.state()["frontier"] == seed["frontier"], "withheld event reached B")
        report["withheld"] = {"receiverPid": observer.process.pid, "receiverFrontier": seed["frontier"]}
    signed = a.signed_push(b.endpoint)
    receipt, event = assert_applied(a, b, command, signed)
    streaming = observer.next(timeout=30)
    require(streaming["pid"] == observer.process.pid and streaming["signedEvent"] == event
            and streaming["socket"] == str(b.socket) and streaming["atNs"] > observed_post["acceptedAtNs"],
            "the original B process did not independently observe the identical signed event")
    require(origin.process.poll() is None and observer.process.poll() is None,
            "one of the original sessions exited before B observed the event")
    report["receiver"] = streaming
    a_state, b_state = a.state(), b.state()
    require(a_state["frontier"] == b_state["frontier"] and a_state["revision"] == b_state["revision"],
            "independent project authorities did not converge")
    report["projection"] = assert_waypoint(b, b_state)
    require(json.loads(a.request(PUSHES, signed)) == receipt and b.audit(command) == [event],
            "same request bytes did not replay the receipt exactly once")
    if args.negative_only:
        changed = json.loads(signed)
        changed["issuedAtMs"] += 1
        changed = a.fixture("resign-push", data=canonical(changed))
        require(json.loads(changed)["requestId"] == receipt["pushId"] and changed != signed,
                "collision fixture must retain the request ID and change signed content")
        conflict = json.loads(a.request(PUSHES, changed, expect=409))
        require(conflict["error"]["code"] == "request_id_conflict"
                and json.loads(a.request(PUSHES + "/" + receipt["pushId"])) == receipt
                and b.state()["frontier"] == b_state["frontier"] and b.audit(command) == [event],
                "altered same-ID signed request modified the original receipt or receiver")
        report["collision"] = {"http": 409, "receiptPreserved": True, "receiverEventCount": 1}
    observer.stop()
    old_pid = b.process.pid
    b.stop()
    require(b.process.returncode == 0, "B did not shut down cleanly")
    ready_nodes(stack, b)
    require(b.process.pid != old_pid and b.audit(command) == [event]
            and b.state()["frontier"] == b_state["frontier"], "B did not retain the event after restart")
    report["restarted"] = {"previousPid": old_pid, "currentPid": b.process.pid,
                           "projection": assert_waypoint(b, b.state())}
    report["push"] = receipt


def blocker_args(command, blocker, summary):
    return ["blocker", "record", "--command-id", command, "--id", blocker, "--summary", summary]


def expect_failed_push(a, target, signed, reason=None):
    receipt = json.loads(a.request(PUSHES, signed, expect=503))
    require(receipt["localState"] == "failed" and receipt["perPeer"][target]["state"] == "failed",
            "push was not recorded as a failed delivery")
    if reason:
        require(reason in receipt["perPeer"][target]["failure"], f"push missed the expected denial: {reason}")
    push_id = json.loads(signed)["requestId"]
    require(receipt == json.loads(a.request(PUSHES + "/" + push_id)), "failed receipt was not durably retrievable")
    return receipt


def peer_recovery(root, args, report, stack):
    if args.negative_only:
        peer_denials(root, args, report, stack)
        return
    a, b = fixtures(root, args)
    enroll(a, b)
    _, baseline = start_pair(a, b, report, stack)
    origin = LiveSession("sender", a, stack)
    report["session"] = origin.describe()
    a.stop()
    require(a.process.returncode == 0, f"A did not shut down cleanly: exit {a.process.returncode}")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as probe:
        probe.settimeout(args.timeout)
        connect_errno = probe.connect_ex(str(a.socket))
    report["shutdownProbe"] = {"pid": a.process.pid, "exit": a.process.returncode,
                               "socketPathExists": a.socket.exists(), "connectErrno": connect_errno,
                               "result": errno.errorcode.get(connect_errno, str(connect_errno))}
    require(connect_errno in (errno.ENOENT, errno.ECONNREFUSED),
            f"A's socket probe expected a missing or refused listener, got {os.strerror(connect_errno)}")
    first = uuid.uuid4().hex
    origin.send({"argv": blocker_args(first, "offline-blocker", "Signed during a local outage"),
                 "socket": str(a.socket)})
    local = origin.next()
    require(local["exit"] == 0 and "committing command" in local["diagnostic"],
            "originating session did not report explicit standalone fallback")
    require(not b.audit(first) and b.state()["frontier"] == baseline["frontier"],
            "offline local command appeared at B without a push")
    ready_nodes(stack, a)
    require_paired(a, b)
    require(len(a.audit(first)) == 1 and not b.audit(first), "offline work was not independently observed at A")
    receipt, event = assert_applied(a, b, first, a.signed_push(b.endpoint))
    require(a.state()["frontier"] == b.state()["frontier"] and event["signerKeyId"] == a.key,
            "offline origin was not reconciled after reconnect")
    report["offline"] = {"originCli": local, "commandId": first, "eventHash": event["integrityHash"],
                         "receipt": receipt, "absentAtReceiverUntilPush": True}

    lost_socket, relay = start_relay(root, stack, a, drop_response=True)
    command = uuid.uuid4().hex
    argv = blocker_args(command, "lost-response-blocker", "Commit survives a lost response")
    origin.send({"argv": argv, "socket": str(lost_socket)})
    ambiguous = origin.next()
    accepted = present_post(a, relay.observations.get(timeout=args.timeout), command)
    require(accepted["responseSuppressed"] and "ambiguous" in ambiguous["diagnostic"]
            and command in ambiguous["diagnostic"], "response loss was not visible for the original command ID")
    before = a.audit(command)
    require(len(before) == 1 and before[0]["signerKeyId"] == a.key and not b.audit(command),
            "direct audits could not reconcile the uncertain command")
    origin.send({"argv": argv, "socket": str(a.socket)})
    same_command = origin.next()
    require(same_command["exit"] == 0 and a.audit(command) == before and not b.audit(command),
            "retry of the original command ID changed its durable effect")

    b.stop()
    failed_request = a.signed_push(b.endpoint)
    failed = expect_failed_push(a, b.endpoint, failed_request)
    ready_nodes(stack, b)
    require_paired(a, b)
    require(not b.audit(command), "failed push applied the new event")
    require(json.loads(a.request(PUSHES, failed_request)) == failed and not b.audit(command),
            "replaying a failed request retried its delivery")
    retry_request = a.signed_push(b.endpoint)
    require(json.loads(retry_request)["requestId"] != failed["pushId"], "new retry reused old push ID")
    retried, event = assert_applied(a, b, command, retry_request)
    require([event] == before and a.state()["frontier"] == b.state()["frontier"],
            "reconnect did not preserve the original signed logical effect")
    report["uncertainty"] = {"commandId": command, "originatingPost": accepted,
                             "ambiguousCli": ambiguous, "sameCommandRetry": same_command,
                             "failedReceipt": failed, "failedReceiptReplayed": True,
                             "newReceipt": retried, "eventHash": event["integrityHash"]}


def denied_attempt(sender, receiver, command, denial):
    baseline = receiver.state()["frontier"]
    history = receiver.all_audit()
    require(not receiver.audit(command), "denial baseline already contains the test command")
    signed = sender.signed_push(receiver.endpoint)
    result = expect_failed_push(sender, receiver.endpoint, signed, denial)
    require(receiver.state()["frontier"] == baseline and receiver.all_audit() == history,
            "denied request changed the receiving project's frontier or signed audit")
    return {"commandId": command, "frontier": baseline, "eventHashes": event_hashes(history),
            "receiverPid": receiver.process.pid, "receiverSocket": str(receiver.socket),
            "receipt": result, "auditUnchanged": True}


def peer_denials(root, args, report, stack):
    a, b = fixtures(root, args)
    c = Device(root, "C", a.project_id, args)
    d = Device(root, "D", str(uuid.uuid4()), args)
    enroll(a, b)
    report["baselineCopy"] = provision(a, b)
    enroll(a, c)
    report["unjoinedBranchCopy"] = provision(a, c)
    pair_both(a, b)
    pair_both(b, c)
    wrong = uuid.uuid4().hex
    d.kbd(*phase_args(wrong, "different-project"))
    ready_nodes(stack, a, b, c, d)
    record_nodes(report, a, b, c, d)
    require_paired(a, b)
    require_paired(b, c)
    def ready_address():
        transport = d.status()["transport"]
        return transport["state"] == "ready" and transport["nodeId"] == d.endpoint
    wait_for("independent unpaired endpoint address", ready_address, (d,), args.timeout)

    unpaired = uuid.uuid4().hex
    a.kbd(*phase_args(unpaired, "unpaired-denial"))
    require(len(a.audit(unpaired)) == 1 and d.endpoint not in a.peers(),
            "unpaired control did not use a live independent endpoint")
    report["unpaired"] = denied_attempt(a, d, unpaired, "target endpoint is not paired")

    unenrolled = uuid.uuid4().hex
    c.kbd(*phase_args(unenrolled, "unenrolled-signer-denial"))
    require(c.state()["devices"][c.key]["status"] == "active" and c.key not in b.state()["devices"],
            "paired but unenrolled fixture does not separate endpoint pairing and project authority")
    require(c.audit(unenrolled)[0]["signerKeyId"] == c.key,
            "unenrolled control did not generate real signed work")
    report["unenrolled"] = denied_attempt(c, b, unenrolled, "signer is not enrolled")

    b.kbd("device", "revoke", "--command-id", uuid.uuid4().hex,
          "--key-id", a.key, "--reason", "Independent revoked signer control")
    revoked = uuid.uuid4().hex
    a.kbd(*phase_args(revoked, "revoked-signer-denial"))
    require(b.state()["devices"][a.key]["status"] == "revoked"
            and a.state()["devices"][a.key]["status"] == "active",
            "revocation fixture did not differ only in receiving authority")
    report["revoked"] = denied_attempt(a, b, revoked, "signer is revoked")

    b.stop()
    d.stop()
    pair_both(b, d)
    ready_nodes(stack, b, d)
    require_paired(b, d)
    require(d.state()["devices"][d.key]["status"] == "active" and d.audit(wrong)[0]["signerKeyId"] == d.key,
            "wrong-project control did not use a valid signer in its actual project")
    b.request(d.route + "/audit", expect=404)
    report["wrongProject"] = denied_attempt(d, b, wrong, "receiving KBD project is unavailable")
    b.request(d.route + "/audit", expect=404)


def accepted_wire_frames(node):
    logs = re.sub(r"\x1b\[[0-9;]*m", "", node.logs())
    return [int(size) for line in logs.splitlines()
            if "direct peer frame accepted by project authority" in line
            for size in re.findall(r"frame_bytes=(\d+)", line)]


def last_bound_udp_ports(node):
    logs = re.sub(r"\x1b\[[0-9;]*m", "", node.logs())
    for line in reversed(logs.splitlines()):
        if "local peer UDP ports bound" in line:
            ports = {family: int(port) for family, port in re.findall(r"(ipv[46])_port=(\d+)", line)}
            require(set(ports) == {"ipv4", "ipv6"} and any(ports.values()),
                    "receiver did not record its bound UDP ports")
            return ports
    raise RuntimeError("receiver did not report its local peer UDP bindings")


def peer_catchup(root, args, report, stack):
    a, b = fixtures(root, args)
    if args.negative_only:
        b.receiver_file_check = args.receiver_file_check
    enroll(a, b)
    _, baseline = start_pair(a, b, report, stack)
    if args.negative_only:
        peer_frame_and_projection_controls(a, b, baseline, report, stack)
        return

    commands = []
    for index in range(40):
        command = uuid.uuid4().hex
        # Repeatable, incompressible test data rather than production content.
        summary = hashlib.shake_256(command.encode()).hexdigest(48 * 1024)
        a.kbd(*blocker_args(command, f"backlog-{index:03d}", summary))
        commands.append(command)
    expected = a.all_audit()
    require(set(commands).issubset({event.get("commandId") for event in expected})
            and b.state()["frontier"] == baseline["frontier"], "fixture backlog did not remain private before push")
    naive = json.loads(a.fixture("legacy-frame-size", a.project_id, b.endpoint))
    maximum = naive["maximumBytes"]
    require(maximum == 16 * 1024 * 1024 and naive["frameBytes"] > maximum,
            "naive legacy full-history encoding does not exceed the real frame bound")
    report["sourceBacklog"] = {**naive, "commandCount": len(commands),
                               "fixtureInputSha256": digest(canonical(commands)),
                               "sourceEventHashes": event_hashes(expected)}
    report["budgets"] = {"deliveryAttempts": 2, "maximumFrameBytes": maximum,
                         "operationSeconds": args.timeout, "progressPollSeconds": 0.025}
    source = a.state()
    request = a.signed_push(b.endpoint)
    original_ports = last_bound_udp_ports(b)

    with ThreadPoolExecutor(max_workers=1) as background:
        push = background.submit(expect_failed_push, a, b.endpoint, request)
        deadline = time.monotonic() + args.timeout
        while time.monotonic() < deadline and not push.done():
            partial = b.state()
            if baseline["revision"] < partial["revision"] < source["revision"]:
                interrupted_pid = b.process.pid
                b.kill()
                break
            time.sleep(0.025)
        else:
            raise RuntimeError("could not interrupt catch-up after observed partial receiver progress")
        interrupted = push.result(timeout=args.timeout)
    ready_nodes(stack, b)
    # Addressed sync uses the authenticated endpoint directly. Gossip
    # neighbor events can lag the receiver restart and use a separate ALPN.
    def receiver_transport_ready():
        transport = b.status()["transport"]
        if transport["state"] == "ready" and transport["nodeId"] == b.endpoint:
            return transport
        return None

    transport = wait_for("restarted addressed receiver", receiver_transport_ready, (a, b), args.timeout)
    resumed_ports = last_bound_udp_ports(b)
    report["restartPorts"] = {"before": original_ports, "after": resumed_ports,
                              "stoppedPid": interrupted_pid, "restartedPid": b.process.pid}
    report["gossipAtRestart"] = {"senderNeighbors": a.peers(), "receiverNeighbors": transport["peers"]}
    require(original_ports == resumed_ports, "the restarted receiver lost its available UDP bindings")
    before_resume = b.all_audit()
    before_resume_state = b.state()
    require(b.process.pid != interrupted_pid and baseline["revision"] < before_resume_state["revision"] < source["revision"],
            "restarted receiver did not retain exactly a partial history")
    require(set(event_hashes(before_resume)).issubset(event_hashes(expected)),
            "interrupted receiver retained unexpected signed events")
    require(json.loads(a.request(PUSHES, request)) == interrupted and b.all_audit() == before_resume,
            "original interrupted request retried instead of replaying its failure receipt")
    prior_frames = accepted_wire_frames(b)
    report["interruption"] = {"stoppedPid": interrupted_pid, "restartedPid": b.process.pid,
                              "beforeRestart": partial["frontier"], "persistedFrontier": before_resume_state["frontier"],
                              "persistedEventHashes": event_hashes(before_resume), "failedReceipt": interrupted}
    status, raw = a.exchange(PUSHES, a.signed_push(b.endpoint))
    resumed = json.loads(raw)
    report["resumedPush"] = {"httpStatus": status, "receipt": resumed}
    if status != 201:
        report["receiverFrameLog"] = [line for line in b.logs().splitlines()
                                      if "direct peer frame" in line]
    require(status == 201, f"resumed push returned HTTP {status}: " + json.dumps(resumed.get("perPeer", resumed)))
    peer = resumed["perPeer"][b.endpoint]
    final = b.state()
    observed = b.all_audit()
    require(peer["state"] == "applied" and peer["insertedEvents"] == source["revision"] - before_resume_state["revision"],
            "new request did not resume from the receiver's verified position")
    require(final["frontier"] == source["frontier"] and observed == expected
            and len({event["eventId"] for event in observed}) == len(observed),
            "receiver did not durably converge to the complete signed backlog without duplicates")
    frames_after_resume = accepted_wire_frames(b)[len(prior_frames):]
    all_frames = accepted_wire_frames(b)
    require(frames_after_resume == peer["acceptedFrameBytes"] and len(all_frames) > 2
            and all(0 < size <= maximum for size in all_frames),
            "receiver's measured encoded frames disagree with the sender receipt or exceed the limit")
    report["catchup"] = {"acceptedWireFrameBytes": all_frames, "frameCount": len(all_frames),
                         "acceptedWireBytes": sum(all_frames), "receipt": resumed,
                         "receiverEventHashes": event_hashes(observed), "frontier": final["frontier"],
                         "projection": assert_waypoint(b, final)}


def peer_frame_and_projection_controls(a, b, baseline, report, stack):
    refresh_route = b.route + "/projections/refresh"
    restored = json.loads(b.request(refresh_route, b""))
    require(restored["current"], "could not establish a current baseline waypoint")
    current = b.waypoint()
    def receiver_snapshot():
        state = b.state()
        return {"revision": state["revision"], "frontier": state["frontier"],
                "eventHashes": event_hashes(b.all_audit()), "documentSha256": sha256(b.snapshot),
                "waypointSha256": sha256(current)}

    before = receiver_snapshot()
    require(before["frontier"] == baseline["frontier"] and before["eventHashes"],
            "receiver byte-limit control must start from its live signed baseline")
    marker = "KBD_FRAME_SIZE_RECEIPT "
    def receiver_receipt():
        for line in b.logs().splitlines():
            if line.startswith(marker):
                return json.loads(line[len(marker):])
        return None

    require(receiver_receipt() is None, "receiver already produced a byte-limit receipt")
    b.process.stdin.write(b"check-size\n")
    b.process.stdin.flush()
    limit_receipt = wait_for("receiver's file-input admission receipt", receiver_receipt, (b,), b.args.timeout)
    maximum_bytes = 16 * 1024 * 1024
    require(limit_receipt["pid"] == b.process.pid and limit_receipt["maximumBytes"] == maximum_bytes
            and limit_receipt["acceptedBytes"] == maximum_bytes
            and limit_receipt["rejectedBytes"] == maximum_bytes + 1
            and limit_receipt["rejection"] == "too_large", "receiver did not enforce its exact byte limit")
    after = receiver_snapshot()
    require(before == after, "byte-limit rejection changed receiver history, revision, or waypoint")
    report["oversize"] = {"result": "PASS", "input": "receiver-local files", "receipt": limit_receipt,
                          "before": before, "after": after}
    backup = current.with_suffix(".test-saved")
    current.rename(backup)
    current.mkdir()
    saved = sha256(backup)

    command = uuid.uuid4().hex
    a.kbd(*phase_args(command, "durable-before-projection"))
    signed = a.signed_push(b.endpoint)
    receipt, event = assert_applied(a, b, command, signed)
    delivery = receipt["perPeer"][b.endpoint]
    require(delivery["projectionCurrent"] is False and delivery["projectionError"],
            "receiver receipt did not expose the stale checkout")
    committed, committed_history = b.state(), b.all_audit()
    diagnostics = json.loads(b.request(b.route + "/diagnostics"))
    require(not diagnostics["projection"]["matchesRuntime"] and diagnostics["runtime"]["frontier"] == committed["frontier"]
            and current.is_dir() and sha256(backup) == saved, "filesystem failure did not isolate the checkout waypoint")
    require(b.audit(command) == [event] and committed["frontier"] == a.state()["frontier"],
            "projection failure lost or duplicated the signed import")
    replay = json.loads(a.request(PUSHES, signed))
    require(replay == receipt and b.all_audit() == committed_history
            and b.state()["frontier"] == committed["frontier"],
            "replaying the signed push changed its receipt or the receiver's durable history")

    current.rmdir()
    backup.rename(current)
    recovery = json.loads(b.request(refresh_route, b""))
    require(recovery["current"] and recovery["sourceRevision"] == committed["revision"]
            and recovery["frontier"] == committed["frontier"], "explicit recovery did not use the durable authority")
    require(b.all_audit() == committed_history and b.audit(command) == [event]
            and b.state()["frontier"] == committed["frontier"], "projection recovery created a logical command")
    report["projectionRecovery"] = {"commandId": command, "eventHash": event["integrityHash"], "receipt": receipt,
                                    "stale": diagnostics["projection"], "refresh": recovery,
                                    "waypoint": assert_waypoint(b, committed), "auditPreserved": True,
                                    "identicalRequestReplayed": replay == receipt}


def executable(value):
    path = shutil.which(str(value))
    if not path:
        raise Blocked(f"required executable is missing: {value}")
    return Path(path).resolve()


def binary_identity(path, timeout, show_version=False):
    info = {"path": str(path), "sha256": sha256(path)}
    if show_version:
        version = subprocess.run([path, "--version"], capture_output=True, timeout=timeout)
        if version.returncode:
            raise Blocked(f"could not identify required executable: {path}")
        info["version"] = version.stdout.decode(errors="replace").strip()
    return info


def source_identity(root, paths, timeout):
    def git(*argv):
        return subprocess.run(["git", "-C", str(root), *argv], capture_output=True,
                              timeout=timeout, check=True).stdout

    revision = git("rev-parse", "HEAD").decode().strip()
    dirty = {}
    for relative in sorted(set(git("ls-files", "-m", "-o", "--exclude-standard", "-z", "--", *paths).split(bytes(1)))):
        if not relative:
            continue
        name = os.fsdecode(relative)
        path = root / name
        if path.is_symlink():
            dirty[name] = "symlink:" + os.readlink(path)
        elif path.is_file():
            dirty[name] = sha256(path)
        else:
            dirty[name] = "absent"
    return {"path": str(root), "revision": revision, "dirtySourceSha256": digest(canonical(dirty)),
            "dirtySourceCount": len(dirty)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", choices=PEER_CASES)
    parser.add_argument("--negative-only", action="store_true")
    parser.add_argument("--companion", "--headless", dest="companion", default="prometheus-companion-headless")
    parser.add_argument("--sync-cli", default="sovereign-sync")
    parser.add_argument("--pack-cli", default="prometheus")
    parser.add_argument("--installed-pack-cli", default=os.environ.get("KBD_INTERSESSION_INSTALLED_CLI"))
    parser.add_argument("--fixture-tool")
    parser.add_argument("--receiver-file-check")
    parser.add_argument("--pack-root", type=Path, default=Path.cwd())
    parser.add_argument("--timeout", type=int, default=45, help="deadline per operation in seconds")
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    cases = {"PEER-001": peer_reader, "PEER-002": peer_delivery,
             "PEER-003": peer_recovery, "PEER-004": peer_catchup}
    report = {"schemaVersion": 1, "case": args.case or "legacy", "negativeOnly": args.negative_only,
              "passed": False, "result": "FAIL"}
    scratch = None
    try:
        if args.timeout < 35 and args.case in {"PEER-003", "PEER-004"}:
            raise Blocked("timeout is below the production transport deadline")
        args.companion = executable(args.companion)
        args.sync_cli = executable(args.sync_cli)
        args.pack_cli = executable(args.pack_cli)
        if args.installed_pack_cli:
            args.installed_pack_cli = executable(args.installed_pack_cli)
        if args.case:
            default_fixture = args.companion.parent / "examples/kbd-peer-fixture"
            args.fixture_tool = executable(args.fixture_tool or default_fixture)
        report["binaries"] = {
            "headless": binary_identity(args.companion, args.timeout),
            "sovereignSync": binary_identity(args.sync_cli, args.timeout),
            "packCandidate": binary_identity(args.pack_cli, args.timeout, show_version=True),
        }
        if args.installed_pack_cli:
            report["binaries"]["installedWriter"] = binary_identity(args.installed_pack_cli, args.timeout, show_version=True)
        if args.fixture_tool:
            report["binaries"]["fixtureTool"] = binary_identity(args.fixture_tool, args.timeout)
        if args.case == "PEER-004" and args.negative_only:
            default_receiver = args.companion.parent / "examples/kbd-receiver-file-check"
            args.receiver_file_check = executable(args.receiver_file_check or default_receiver)
            report["binaries"]["receiverFileCheck"] = binary_identity(args.receiver_file_check, args.timeout)
        if args.case:
            companion_root = Path(__file__).resolve().parent.parent
            report["companionSource"] = source_identity(companion_root,
                ["Cargo.toml", "Cargo.lock", "crates", "scripts/verify-kbd-peers.py", "skills/sync-push"], args.timeout)
            report["fullSource"] = source_identity(args.pack_root,
                ["tools/prometheus-cli", "substrate", "scripts/verify-kbd-intersession.sh"], args.timeout)
            report["cargoLockSha256"] = sha256(companion_root / "Cargo.lock")
            report["runnerSha256"] = sha256(Path(__file__))
        # Short private sockets, keys and authority stores are disposable.
        with tempfile.TemporaryDirectory(prefix="kbd-peers-", dir="/tmp") as scratch, ExitStack() as stack:
            require(Path(scratch).stat().st_mode & 0o077 == 0, "fixture root permissions are too broad")
            cases.get(args.case, exercise)(Path(scratch), args, report, stack)
        report["result"] = "PASS"
        report["passed"] = True
    except Blocked as error:
        report["result"] = "BLOCKED"
        report["error"] = str(error)
    except (OSError, RuntimeError, KeyError, ValueError, http.client.HTTPException,
            subprocess.SubprocessError, queue.Empty) as error:
        # TimeoutExpired contains argv, including disposable pairing tickets.
        report["error"] = "subprocess exceeded its deadline" if isinstance(error, subprocess.TimeoutExpired) else str(error)
    report["cleanup"] = {"scratchRemoved": scratch is None or not Path(scratch).exists(),
                         "processes": [{"role": role, "pid": proc.pid, "exit": proc.poll(),
                                        "intentionalInterruption": proc.pid in INTERRUPTED_PIDS}
                                       for role, proc in PROCESSES]}
    cleanup = report["cleanup"]
    if report["passed"] and (not cleanup["scratchRemoved"] or any(
            row["exit"] is None or (row["exit"] == -signal.SIGKILL and not row["intentionalInterruption"])
            for row in cleanup["processes"])):
        report.update({"passed": False, "result": "FAIL", "error": "fixture cleanup was incomplete"})
    report["assertionCount"] = ASSERTIONS
    encoded = json.dumps(report, indent=2) + "\n"
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(encoded)
    print(encoded, end="")
    return {"PASS": 0, "FAIL": 1, "BLOCKED": 2}[report["result"]]


if __name__ == "__main__":
    if sys.argv[1:] == ["--session-worker"]:
        session_worker()
    else:
        raise SystemExit(main())
