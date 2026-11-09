"""Private Herdr high-output fault fixture for the Rust runner; fake child only."""
import argparse
import json
from pathlib import Path

from rust_migration_stage3 import Harness, eventual, load, save


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-root", type=Path, required=True)
    args = parser.parse_args()
    suite = Harness(args.evidence_root.resolve())
    try:
        suite.start()
        state, directory = suite.case("output-flood")
        packet = load(state / "packet-input.json")
        packet["mode"] = "output-flood"
        save(state / "packet-input.json", packet)
        suite.prepare(state, directory)
        first = suite.probe("ensure", state, directory)
        assert first.returncode == 0, first.stderr
        receipt = eventual(lambda: load(directory / "receipt.json")
                           if (directory / "receipt.json").exists() else None, 25)
        assert receipt["status"] == "failed", receipt
        assert "32 MiB" in receipt["error"], receipt
        limit = load(directory / "output-limit.json")
        assert limit["truncated"] is True
        assert limit["limit_bytes"] == 32 * 1024 * 1024
        assert (directory / "transcript.log").stat().st_size == 32 * 1024 * 1024
        assert load(directory / "invocations.json")["count"] == 1
        replay = suite.probe("ensure", state, directory)
        assert replay.returncode == 0, replay.stderr
        assert load(directory / "invocations.json")["count"] == 1
        claim = load(directory / "claim.json")["runner"]
        hwm = Path(f"/proc/{claim['pid']}/status").read_text()
        peak_kib = int(next(line.split()[1] for line in hwm.splitlines()
                            if line.startswith("VmHWM:")))
        save(suite.root / "summary.json", {
            "status": "passed", "receipt": receipt["status"],
            "output_limit": limit, "peak_rss_kib": peak_kib,
            "transcript_bytes": 32 * 1024 * 1024,
            "invocations": 1, "replay_status": json.loads(replay.stdout)["status"],
        })
    finally:
        binding = suite.root / "output-flood/probes/rust-output-flood/binding.json"
        if binding.exists() and suite.server is not None:
            try:
                pane = suite.call("pane.read", {"pane_id": load(binding)["pane_id"],
                                                 "source": "recent", "lines": 100})
                save(suite.root / "output-flood/retained-pane.json", pane)
            except Exception as error:
                save(suite.root / "output-flood/pane-read-error.json", {"error": str(error)})
        suite.cleanup()


if __name__ == "__main__":
    main()
