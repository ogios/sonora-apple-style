#!/usr/bin/env python3
"""Measure isolated release processes during paced, silent local-file playback on Linux."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import statistics
import struct
import subprocess
import tempfile
import time
import wave


class Client:
    """Send ordered protocol requests, discarding notifications after receiving them."""

    def __init__(self, path, version):
        self.socket = socket.socket(socket.AF_UNIX)
        self.socket.settimeout(8)
        self.socket.connect(str(path))
        self.sequence = 0
        self.send({"id": 0, "body": {"type": "hello", "data": {"version": version}}})
        assert self.read()["type"] == "welcome"

    def send(self, value):
        data = json.dumps(value, separators=(",", ":")).encode()
        self.socket.sendall(struct.pack(">I", len(data)) + data)

    def read(self):
        def take(size):
            data = b""
            while len(data) < size:
                part = self.socket.recv(size - len(data))
                assert part, "server disconnected"
                data += part
            return data

        return json.loads(take(struct.unpack(">I", take(4))[0]))

    def request(self, body):
        self.sequence += 1
        self.send({"id": self.sequence, "body": body})
        while True:
            message = self.read()
            if message["type"] == "reply" and message["data"]["id"] == self.sequence:
                result = message["data"]["result"]
                assert "Ok" in result, result
                return result["Ok"]

    def call(self, object_name, method, arguments):
        return self.request({"type": "call", "data": {
            "object": object_name, "method": method, "arguments": arguments,
        }})


def memory(pid):
    """Read resident and proportional memory in KiB, without counting virtual mappings."""
    values = {}
    for line in Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines():
        key = line.split(":")[0]
        if key in ("Rss", "Pss"):
            values[key] = int(line.split()[1])
    children = Path(f"/proc/{pid}/task/{pid}/children").read_text().split()
    values["children"] = {child: memory(child) for child in children}
    return values


def rss_tree(value):
    return value["Rss"] + sum(rss_tree(child) for child in value["children"].values())


def summarize(samples):
    """Report medians and sampled maxima, summing processes at each sample timestamp."""
    roles = list(samples[0]["memory"]) + ["total"]
    result = {}
    for role in roles:
        values = [sum(rss_tree(v) for v in s["memory"].values()) if role == "total"
                  else rss_tree(s["memory"][role]) for s in samples]
        result[role] = {"median_mib": round(statistics.median(values) / 1024, 3),
                        "peak_mib": round(max(values) / 1024, 3)}
    return result


def run(args, root, binaries, label, repetition, with_gui):
    """Launch private processes and sample idle and verified real-time playback phases."""
    directory = root / f"{label}-{repetition}"
    directory.mkdir(mode=0o700)
    endpoint = directory / "ipc/server.sock"
    env = dict(os.environ, SONORA_SOCKET=str(endpoint),
               XDG_CONFIG_HOME=str(directory / "config"),
               XDG_DATA_HOME=str(directory / "data"),
               XDG_CACHE_HOME=str(directory / "cache"),
               XDG_STATE_HOME=str(directory / "state"), XDG_RUNTIME_DIR=str(directory),
               DBUS_SESSION_BUS_ADDRESS=f"unix:path={directory}/no-bus",
               PULSE_SERVER=args.pulse_server)
    env.pop("ALSA_CONFIG_PATH", None)
    server_env = env.copy()
    for key in ("DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"):
        server_env.pop(key, None)
    processes, logs, client = {}, [], None
    try:
        log = open(directory / "server.log", "w")
        logs.append(log)
        processes["server"] = subprocess.Popen([str(binaries[label])], env=server_env,
                                               stdout=log, stderr=log, start_new_session=True)
        deadline = time.monotonic() + 15
        while True:
            assert processes["server"].poll() is None, "server exited"
            try:
                client = Client(endpoint, 2 if label == "before" else 3)
                break
            except (FileNotFoundError, ConnectionRefusedError):
                assert time.monotonic() < deadline, "server readiness timed out"
                time.sleep(0.1)
        client.request({"type": "subscribe"})
        client.call("settings", "set_lyrics_for_local_files", [False])
        client.call("settings", "set_discord_presence", [False])
        if with_gui:
            display = env.get("WAYLAND_DISPLAY")
            if display and not os.path.isabs(display):
                env["WAYLAND_DISPLAY"] = str(Path(os.environ["XDG_RUNTIME_DIR"]) / display)
            log = open(directory / "gui.log", "w")
            logs.append(log)
            processes["gui"] = subprocess.Popen([str(binaries["gui"])], env=env,
                                                stdout=log, stderr=log, start_new_session=True)
        print(f"{directory.name}: warming up; pids " + str({k: p.pid for k, p in processes.items()}), flush=True)
        time.sleep(15)

        def collect(seconds, playback=False):
            samples, clocks = [], []
            start = time.monotonic()
            next_snapshot = 0
            while time.monotonic() - start < seconds:
                elapsed = time.monotonic() - start
                for p in processes.values():
                    assert p.poll() is None, "measured process exited"
                samples.append({"seconds": elapsed, "memory": {
                    role: memory(p.pid) for role, p in processes.items()}})
                if playback and elapsed >= next_snapshot:
                    objects = client.request({"type": "snapshot"})["objects"]
                    state = objects["playback"]
                    position = state["position"]["secs"] + state["position"]["nanos"] / 1e9
                    clocks.append({"seconds": elapsed, "state": state["state"], "position": position})
                    if elapsed >= 3:
                        assert state["state"] == "Playing", clocks[-1]
                    output = objects["outputs"]["active"]
                    assert output and output["id"].startswith("pulseaudio:"), output
                    next_snapshot += 1
                time.sleep(max(0, start + len(samples) * 0.2 - time.monotonic()))
            return samples, clocks

        idle, _ = collect(5)
        print(f"{directory.name}: idle {summarize(idle)}; playing for {args.seconds}s", flush=True)
        client.call("playback", "open_paths", [[str(root / "silence.wav")]])
        active, clocks = collect(args.seconds, playback=True)
        advancement = clocks[-1]["position"] - clocks[3]["position"]
        wall = clocks[-1]["seconds"] - clocks[3]["seconds"]
        assert abs(advancement - wall) < 2, (advancement, wall)
        result = {"label": label, "repetition": repetition, "with_gui": with_gui,
                  "directory": str(directory), "pids": {k: p.pid for k, p in processes.items()},
                  "idle": summarize(idle), "playback": summarize(active),
                  "steady": summarize([s for s in active if s["seconds"] >= args.seconds - 20]),
                  "clocks": clocks, "idle_samples": idle, "playback_samples": active}
        print(f"{directory.name}: done " + json.dumps({k: result[k] for k in ("idle", "playback", "steady")}), flush=True)
        return result
    finally:
        if client:
            client.socket.close()
        for p in reversed(list(processes.values())):
            if p.poll() is None:
                os.killpg(p.pid, signal.SIGTERM)
                try:
                    p.wait(10)
                except subprocess.TimeoutExpired:
                    os.killpg(p.pid, signal.SIGKILL)
                    p.wait()
        for log in logs:
            log.close()
        for log in directory.glob("*.log"):
            assert "panicked at" not in log.read_text(), str(log)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--after", type=Path, default=Path("target/release/sonora-server"))
    parser.add_argument("--gui", type=Path, default=Path("target/release/sonora"))
    parser.add_argument("--pulse-server", default=f"unix:{os.environ.get('XDG_RUNTIME_DIR', '')}/pulse/native")
    parser.add_argument("--seconds", type=int, default=60)
    parser.add_argument("--repetitions", type=int, default=2)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    assert args.seconds >= 30
    root = args.output or Path(tempfile.mkdtemp(prefix="sonora-playback-memory-"))
    root = root.resolve()
    root.mkdir(parents=True, exist_ok=True)
    binaries, metadata = {}, {}
    for label, source in (("before", args.before), ("after", args.after), ("gui", args.gui)):
        target = root / label
        shutil.copy2(source, target)
        binaries[label] = target
        metadata[label] = {"source": str(source.resolve()), "bytes": target.stat().st_size,
                           "sha256": hashlib.sha256(target.read_bytes()).hexdigest()}
    with wave.open(str(root / "silence.wav"), "wb") as audio:
        audio.setnchannels(2)
        audio.setsampwidth(2)
        audio.setframerate(48000)
        audio.writeframes(bytes(48000 * 4 * (args.seconds + 30)))
    print(f"output: {root}", flush=True)
    result = {"binaries": metadata, "kernel": os.uname().release,
              "sample_interval_seconds": 0.2, "playback_seconds": args.seconds,
              "pulse_server": args.pulse_server, "runs": []}
    for repetition in range(args.repetitions):
        configurations = [("before", False), ("after", False), ("after", True)]
        if repetition % 2:
            configurations.reverse()
        for label, gui in configurations:
            result["runs"].append(run(args, root, binaries, label,
                                      f"{repetition}-{'gui' if gui else 'headless'}", gui))
            (root / "results.json").write_text(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
