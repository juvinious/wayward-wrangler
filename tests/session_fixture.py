"""Disposable Python peer for Rust session protocol proof; no workflow execution."""
from __future__ import annotations

import argparse
import json
import os
import socket
import struct
import threading
import time
from pathlib import Path

MAX_FRAME = 1024 * 1024


def object_no_duplicates(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("duplicate key")
        value[key] = item
    return value


def read_frame(sock):
    data = bytearray()
    while True:
        char = sock.recv(1)
        if not char:
            raise EOFError
        if char == b"\n":
            break
        if len(data) == MAX_FRAME:
            raise ValueError("frame too large")
        data.extend(char)
    return json.loads(data.decode("utf-8"), object_pairs_hook=object_no_duplicates,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite")))


def send(sock, kind, request_id, body):
    sock.sendall(json.dumps({"v": 1, "kind": kind, "request_id": request_id, "body": body},
                            separators=(",", ":"), allow_nan=False).encode() + b"\n")


class Fixture:
    def __init__(self, path, key="fixture", release="proof-v1"):
        self.path = Path(path)
        self.key = key
        self.release = release
        self.commands = {}
        self.events = []
        self.drop_next_command_reply = False
        self.lock = threading.Lock()
        self.closed = threading.Event()
        self.listener = socket.socket(socket.AF_UNIX)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.listener.bind(str(self.path))
        os.chmod(self.path, 0o600)
        self.listener.listen(8)
        self.listener.settimeout(.1)

    def serve(self):
        while not self.closed.is_set():
            try:
                peer, _ = self.listener.accept()
            except socket.timeout:
                continue
            except OSError:
                if self.closed.is_set():
                    break
                raise
            threading.Thread(target=self.handle, args=(peer,), daemon=True).start()

    def handle(self, peer):
        with peer:
            try:
                pid, uid, gid = struct.unpack("3i", peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                if uid != os.geteuid():
                    return
                hello = read_frame(peer)
                body = hello["body"]
                valid = (hello["kind"] == "hello" and body.get("protocol", {}).get("major") == 1
                         and body.get("release_id") == self.release and body.get("project_key") == self.key)
                send(peer, "hello_result" if valid else "error", hello["request_id"],
                     {"accepted": valid, "owner_generation": 1, "state_schema": 1,
                      "code": None if valid else "protocol_mismatch"})
                if not valid:
                    return
                while True:
                    request = read_frame(peer)
                    kind, request_id, payload = request["kind"], request["request_id"], request["body"]
                    if kind == "snapshot":
                        with self.lock:
                            result = {"cursor": len(self.events), "tasks": [], "state_revision": len(self.events)}
                        send(peer, "snapshot", request_id, result)
                    elif kind == "events_read":
                        after = payload["after_seq"]
                        limit = min(64, payload["limit"])
                        with self.lock:
                            selected = self.events[after:after + limit]
                            next_seq = after + len(selected)
                        if not selected:
                            time.sleep(min(payload.get("wait_ms", 0), 1000) / 1000)
                        send(peer, "events_result", request_id, {"events": selected, "next_seq": next_seq,
                                                                  "state_revision": len(self.events)})
                    elif kind == "command":
                        cmd_id = payload["command_id"]
                        with self.lock:
                            previous = self.commands.get(cmd_id)
                            if previous is not None:
                                result = {"status": "accepted" if previous == payload else "rejected",
                                          "code": None if previous == payload else "idempotency_conflict"}
                            elif payload.get("expected_state_revision") != len(self.events):
                                result = {"status": "rejected", "code": "stale_state", "state_revision": len(self.events)}
                            else:
                                self.commands[cmd_id] = payload
                                self.events.append({"seq": len(self.events) + 1, "type": "message.submitted",
                                                    "text": payload.get("payload", {}).get("text")})
                                result = {"status": "accepted", "event_seq": len(self.events),
                                          "state_revision": len(self.events)}
                        if self.drop_next_command_reply:
                            self.drop_next_command_reply = False
                            return
                        send(peer, "command_result", request_id, result)
                    elif kind == "command_get":
                        with self.lock:
                            found = payload["command_id"] in self.commands
                        send(peer, "command_result", request_id, {"status": "accepted" if found else "not_found"})
                    else:
                        send(peer, "error", request_id, {"code": "invalid_request", "message": "unknown request"})
            except (OSError, ValueError, KeyError, EOFError, UnicodeError):
                return

    def close(self):
        self.closed.set()
        self.listener.close()
        self.path.unlink(missing_ok=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("socket")
    args = parser.parse_args()
    fixture = Fixture(args.socket)
    try:
        fixture.serve()
    finally:
        fixture.close()
