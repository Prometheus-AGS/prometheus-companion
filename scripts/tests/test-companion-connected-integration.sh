#!/usr/bin/env bash
# Executes one explicitly built real-process Rust integration target. Never Cargo.
# 0 PASS, 1 FAIL, 2 BLOCKED; root owns the consolidated gate and publication.
set -euo pipefail
command -v python3 >/dev/null 2>&1 || { echo 'BLOCKED: Python 3 is required' >&2; exit 2; }
exec python3 -B - "$@" <<'PY'
import argparse, datetime, hashlib, json, os, pathlib, signal, shutil, subprocess, sys, tempfile
p=argparse.ArgumentParser(description='Actual two headless Companion hosts and selected MCP integration')
for name in ('companion','evidence','scratch','headless-bin','sync-bin','prometheus-bin','companion-integration-bin'):
    p.add_argument('--'+name,required=True)
p.add_argument('--timeout-seconds',type=int,default=600)
a=p.parse_args();evidence=pathlib.Path(a.evidence);owned=None;child=None;code=1;reason=None
started=datetime.datetime.now(datetime.timezone.utc).isoformat();inputs={}
class Blocked(Exception):pass
def absolute(value):
    path=pathlib.Path(value)
    if not path.is_absolute():raise Blocked('absolute path required: '+value)
    try:return path.resolve(strict=True)
    except OSError:raise Blocked('unavailable explicit input: '+value)
def interrupted(signum,frame):raise KeyboardInterrupt('coordinator interrupted Companion scenario')
for sig in (signal.SIGINT,signal.SIGTERM):signal.signal(sig,interrupted)
try:
    if os.name!='posix':raise Blocked('production Unix socket scenario requires a Unix host')
    if a.timeout_seconds<30:raise Blocked('scenario timeout must allow actual process startup')
    scratch=absolute(a.scratch);companion=absolute(a.companion)
    if scratch==pathlib.Path.home().resolve():raise Blocked('real HOME is not scratch')
    if not evidence.is_absolute():raise Blocked('evidence must be absolute')
    evidence.mkdir(parents=True,exist_ok=True)
    for name,value in (('headless',a.headless_bin),('sync',a.sync_bin),('prometheus',a.prometheus_bin),('integration',a.companion_integration_bin)):
        path=absolute(value)
        if not path.is_file() or not os.access(path,os.X_OK):raise Blocked('explicit binary is not executable: '+str(path))
        inputs[name]=dict(path=str(path),sha256=hashlib.sha256(path.read_bytes()).hexdigest())
    owned=pathlib.Path(tempfile.mkdtemp(prefix='companion-connected-',dir=scratch));(owned/'.owned-integration').write_text('companion-connected\n')
    tools=owned/'tools';tools.mkdir()
    for name in ('git','kill','sh','bash','uname'):
        tool=shutil.which(name)
        if tool:(tools/name).symlink_to(pathlib.Path(tool).resolve())
    env=dict(PATH=str(tools),HOME=str(owned/'home'),CODEX_HOME=str(owned/'codex'),XDG_CONFIG_HOME=str(owned/'config'),
        XDG_DATA_HOME=str(owned/'data'),PROMETHEUS_DATA_DIR=str(owned/'data'),CORTEX_DATA_DIR=str(owned/'cortex'),
        CLAUDE_CONFIG_DIR=str(owned/'claude'),PROMETHEUS_PLUGIN_ROOT=str(owned/'plugin'),
        PROMETHEUS_LEARNING_QUEUE=str(owned/'queue'),PROMETHEUS_LEARNING_LOG_DIR=str(owned/'log'),
        PROMETHEUS_LEARNING_INDEX_DIR=str(owned/'index'),PROMETHEUS_TEAM_DIGEST_DIR=str(owned/'digest'),
        PROMETHEUS_LEARNING_CORTEX='0',GIT_CONFIG_GLOBAL=os.devnull,GIT_CONFIG_NOSYSTEM='1',
        LDD_COMPANION_CASE_ROOT=str(owned),LDD_COMPANION_EVIDENCE=str(evidence),LDD_INTEGRATION_TOOLS=str(tools),
        LDD_COMPANION_HEADLESS_BIN=inputs['headless']['path'],LDD_COMPANION_SYNC_BIN=inputs['sync']['path'],
        LDD_PROMETHEUS_BIN=inputs['prometheus']['path'],LANG='C',LC_ALL='C')
    argv=[inputs['integration']['path'],'--exact','actual_two_headless_hosts_and_selected_mcp_apply_signed_state','--test-threads=1','--nocapture']
    with (evidence/'companion-integration.stdout').open('w') as out,(evidence/'companion-integration.stderr').open('w') as err:
        child=subprocess.Popen(argv,env=env,cwd=companion,stdout=out,stderr=err,start_new_session=True)
        try:exit_code=child.wait(timeout=a.timeout_seconds)
        except subprocess.TimeoutExpired:raise AssertionError('actual Companion process scenario exceeded declared deadline')
    output=(evidence/'companion-integration.stdout').read_text()+(evidence/'companion-integration.stderr').read_text()
    # Prerequisites are explicit failures, never passing Rust-test skips. The
    # wrapper alone maps the documented BLOCKED prerequisite marker to exit 2.
    if exit_code!=0:
        code=2 if 'BLOCKED:' in output else 1;reason='real integration target exited '+str(exit_code)
    elif not (evidence/'connected-acceptance.json').is_file():
        code=2;reason='target did not produce the real process receipt (unsupported platform/feature or filtered target)'
    else:code=0
except Blocked as error:code,reason=2,str(error)
except BaseException as error:code,reason=1,type(error).__name__+': '+str(error)
finally:
    if child is not None:
        # Includes both scenario-owned hosts/MCP if the target panics, times out
        # or the coordinator interrupts it. Never signal an unrelated PID/group.
        try:os.killpg(child.pid,signal.SIGTERM)
        except ProcessLookupError:pass
        if child.poll() is None:
            try:child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                try:os.killpg(child.pid,signal.SIGKILL)
                except ProcessLookupError:pass
                child.wait()
        try:os.killpg(child.pid,signal.SIGKILL)
        except ProcessLookupError:pass
    if evidence.is_absolute():
        evidence.mkdir(parents=True,exist_ok=True)
        (evidence/'companion-connected-result.json').write_text(json.dumps(dict(status=['PASS','FAIL','BLOCKED'][code],exitCode=code,reason=reason,
            start=started,end=datetime.datetime.now(datetime.timezone.utc).isoformat(),sourceRoot=a.companion,inputs=inputs,
            isolatedRoot=str(owned) if owned else None,acceptance=['14/AC-1'],publication='pending; not exercised'),indent=2))
    if owned is not None and (owned/'.owned-integration').is_file():shutil.rmtree(owned)
print(['PASS','FAIL','BLOCKED'][code]+': Companion real-process integration'+(': '+reason if reason else ''));sys.exit(code)
PY
