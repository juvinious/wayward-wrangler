"""Disposable PTY proof. Requires WAYWR_TEST_BINARY pointing to a built Rust binary."""
import fcntl
import os
import pty
import select
import struct
import subprocess
import tempfile
import termios
import threading
import time
import unittest
from pathlib import Path

from session_fixture import Fixture


class RustPtyTest(unittest.TestCase):
    def test_paste_submission_resize_and_terminal_restore(self):
        binary = os.environ.get("WAYWR_TEST_BINARY")
        if not binary:
            self.skipTest("Set WAYWR_TEST_BINARY to built Rust client")
        with tempfile.TemporaryDirectory() as temp:
            fixture = Fixture(Path(temp) / "session.sock")
            server = threading.Thread(target=fixture.serve, daemon=True)
            server.start()
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
            process = subprocess.Popen([binary, "--socket", str(fixture.path), "--project-key", "fixture",
                                        "--release-id", "proof-v1", "--outbox-dir", str(Path(temp) / "outbox")],
                                       stdin=slave, stdout=slave, stderr=slave)
            os.close(slave)
            output = bytearray()
            stop_reading = threading.Event()
            def drain():
                while not stop_reading.is_set():
                    if select.select([master], [], [], .05)[0]:
                        try:
                            output.extend(os.read(master, 65536))
                        except OSError:
                            break
            reader = threading.Thread(target=drain, daemon=True)
            reader.start()
            try:
                time.sleep(.3)
                os.write(master, b"\x1b[200~first\nsecond\x1b[201~")
                time.sleep(.15)
                self.assertFalse(fixture.commands, "paste submitted without Ctrl-S")
                os.write(master, b"\x13")
                deadline = time.monotonic() + 3
                while len(fixture.commands) < 1 and time.monotonic() < deadline:
                    time.sleep(.02)
                self.assertEqual(len(fixture.commands), 1)
                self.assertEqual(next(iter(fixture.commands.values()))["payload"]["text"], "first\nsecond")
                fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 15, 50, 0, 0))
                os.write(master, b"\x1b")
                time.sleep(.12)
                os.write(master, b"q")
                process.wait(timeout=5)
                self.assertEqual(process.returncode, 0)
                self.assertIn(b"\x1b[?1049l", output, "alternate screen was not restored")
            finally:
                if process.poll() is None:
                    process.terminate()
                    try: process.wait(timeout=2)
                    except subprocess.TimeoutExpired: process.kill(); process.wait(timeout=2)
                stop_reading.set()
                reader.join(timeout=1)
                os.close(master)
                fixture.close()
                server.join(timeout=1)

    def test_bounded_event_flood_keeps_input_live(self):
        if os.environ.get("WAYWR_RUN_STRESS") != "1":
            self.skipTest("Set WAYWR_RUN_STRESS=1 for the 60-second proof")
        binary = os.environ["WAYWR_TEST_BINARY"]
        with tempfile.TemporaryDirectory() as temp:
            fixture = Fixture(Path(temp) / "session.sock")
            server = threading.Thread(target=fixture.serve, daemon=True); server.start()
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
            process = subprocess.Popen([binary, "--socket", str(fixture.path), "--project-key", "fixture",
                                        "--release-id", "proof-v1", "--outbox-dir", str(Path(temp) / "outbox")],
                                       stdin=slave, stdout=slave, stderr=slave)
            os.close(slave)
            stop = threading.Event()
            bytes_seen = [0]
            def drain():
                while not stop.is_set():
                    if select.select([master], [], [], .05)[0]:
                        try: bytes_seen[0] += len(os.read(master, 65536))
                        except OSError: return
            reader = threading.Thread(target=drain, daemon=True); reader.start()
            try:
                time.sleep(.3)
                with fixture.lock:
                    fixture.events.extend({"seq": n, "type": "activity", "text": "x" * 80}
                                          for n in range(1, 10001))
                deadline = time.monotonic() + 60
                submitted = False
                while time.monotonic() < deadline:
                    if not submitted and time.monotonic() > deadline - 30:
                        os.write(master, b"responsive\x13")
                        submitted = True
                        ack_deadline = time.monotonic() + 1
                        while not fixture.commands and time.monotonic() < ack_deadline:
                            time.sleep(.01)
                        self.assertTrue(fixture.commands, "submission was not acknowledged within one second")
                    time.sleep(.1)
                self.assertTrue(process.poll() is None, "client quit during flood")
                self.assertGreater(bytes_seen[0], 1000, "no live render")
                os.write(master, b"\x1b"); time.sleep(.12); os.write(master, b"q")
                process.wait(timeout=5)
                self.assertEqual(process.returncode, 0)
            finally:
                if process.poll() is None:
                    process.terminate(); process.wait(timeout=2)
                stop.set(); reader.join(timeout=1); os.close(master)
                fixture.close(); server.join(timeout=1)


if __name__ == "__main__":
    unittest.main()
