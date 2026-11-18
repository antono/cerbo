#!/usr/bin/env python3
"""Runtime check of cerboid under a fake systemd (the Home Manager service).

Usage: check-socket-activation.py /path/to/cerboid /path/to/core/src/rpc/mod.rs

The protocol version is read from the RPC source, so the check follows
every bump of PROTOCOL_VERSION.

Plays the part of systemd's socket unit: binds the daemon's socket, then
starts cerboid with the socket as descriptor 3 and LISTEN_PID / LISTEN_FDS set
(sd_listen_fds(3)). Verifies that
  1. a client connects through the socket and gets the handshake and a ping,
  2. a second systemd-started instance fails visibly (non-zero, message)
     instead of exiting 0 and leaving the socket unserved,
  3. the socket file survives the daemon's stop (it is systemd's),
  4. a daemon started without systemd refuses to unlink a socket that
     something still listens on.
Everything lives under a temporary XDG tree; nothing outside it is touched.
"""
import json
import re
import os
import signal
import socket
import subprocess
import sys
import tempfile
import time

cerboid = sys.argv[1]
with open(sys.argv[2]) as f:
    PROTOCOL = int(re.search(r"pub const PROTOCOL_VERSION: u32 = (\d+);", f.read()).group(1))
root = tempfile.mkdtemp(prefix="cerboid-check-")
env = {
    "PATH": os.environ.get("PATH", ""),
    "HOME": os.path.join(root, "home"),
    "XDG_RUNTIME_DIR": os.path.join(root, "run"),
    "XDG_CONFIG_HOME": os.path.join(root, "config"),
    "XDG_DATA_HOME": os.path.join(root, "data"),
    "XDG_CACHE_HOME": os.path.join(root, "cache"),
    "XDG_STATE_HOME": os.path.join(root, "state"),
}
for k in ("ORT_DYLIB_PATH",):
    if k in os.environ:
        env[k] = os.environ[k]
for d in env.values():
    if d.startswith(root):
        os.makedirs(d, mode=0o700, exist_ok=True)

run_dir = os.path.join(env["XDG_RUNTIME_DIR"], "cerbo")
os.makedirs(run_dir, mode=0o700, exist_ok=True)
sock_path = os.path.join(run_dir, "cerboid.sock")

# systemd binds and listens before it starts the service.
listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
listener.bind(sock_path)
os.chmod(sock_path, 0o600)
listener.listen(16)


def fail(message):
    print(f"FAIL: {message}", file=sys.stderr)
    sys.exit(1)


def start(activated):
    """Run cerboid; with `activated`, as systemd would for its socket."""
    child_env = dict(env)
    err_path = os.path.join(root, f"stderr-{time.monotonic_ns()}")
    err = open(err_path, "wb")
    pid = os.fork()
    if pid == 0:
        try:
            if activated:
                if listener.fileno() != 3:
                    os.dup2(listener.fileno(), 3)
                # Same descriptor (dup2 would be a no-op): drop close-on-exec.
                os.set_inheritable(3, True)
                child_env["LISTEN_PID"] = str(os.getpid())
                child_env["LISTEN_FDS"] = "1"
            os.dup2(err.fileno(), 2)
            os.execve(cerboid, [cerboid], child_env)
        finally:
            os._exit(127)
    err.close()
    return pid, err_path


def wait(pid, seconds):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        done, status = os.waitpid(pid, os.WNOHANG)
        if done:
            return os.waitstatus_to_exitcode(status)
        time.sleep(0.05)
    os.kill(pid, signal.SIGKILL)
    os.waitpid(pid, 0)
    fail(f"process {pid} did not exit in {seconds}s")


def stderr_of(path):
    with open(path, "rb") as f:
        return f.read().decode(errors="replace")


def talk(lines):
    """Connect through the socket, send lines, return one parsed answer each."""
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(30)
    client.connect(sock_path)
    reader = client.makefile("rwb")
    answers = []
    for line in lines:
        reader.write((json.dumps(line) + "\n").encode())
        reader.flush()
        answers.append(json.loads(reader.readline()))
    client.close()
    return answers


# 1. The systemd-started daemon serves a client on the inherited socket.
daemon, daemon_err = start(activated=True)
welcome, pong = talk([
    {"hello": {"protocol": PROTOCOL, "client": "cli", "name": "check"}},
    {"id": 1, "method": "ping"},
])
if welcome.get("type") != "welcome" or welcome.get("protocol") != PROTOCOL:
    fail(f"no handshake: {welcome}; stderr: {stderr_of(daemon_err)}")
if welcome.get("pid") != daemon:
    fail(f"handshake came from pid {welcome.get('pid')}, not {daemon}")
if pong.get("result") != "pong":
    fail(f"ping not answered: {pong}")
print("ok: client connected through the inherited socket and got the handshake")

# 2. A second systemd-started instance finds the lock taken and must fail.
second, second_err = start(activated=True)
code = wait(second, 30)
message = stderr_of(second_err)
if code == 0 or "systemd" not in message:
    fail(f"second activated instance: exit {code}, stderr {message!r}")
print("ok: second activated instance failed visibly:", message.strip())

# The first one is still serving.
if talk([{"hello": {"protocol": PROTOCOL, "client": "cli"}}])[0].get("type") != "welcome":
    fail("first daemon stopped answering")

# 3. Stopping the daemon leaves the socket to systemd.
os.kill(daemon, signal.SIGTERM)
if wait(daemon, 30) != 0:
    fail(f"daemon did not stop cleanly: {stderr_of(daemon_err)}")
if not os.path.exists(sock_path):
    fail("the socket file went away with the daemon")
print("ok: socket survived the stop")

# 4. Without systemd, a socket that something still listens on is not stolen.
manual, manual_err = start(activated=False)
code = wait(manual, 30)
message = stderr_of(manual_err)
if code == 0 or "already listens" not in message:
    fail(f"manual start beside a live socket: exit {code}, stderr {message!r}")
if not os.path.exists(sock_path):
    fail("the live socket was unlinked")
print("ok: a live socket was left alone:", message.strip())
