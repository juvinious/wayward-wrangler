"""Project-local lifecycle races and deterministic handoff; no model/Herdr calls."""
import json
import asyncio
import socket
import tempfile
import threading
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from agent_workflow.runtime import RuntimePaths, bind, unbind
from agent_workflow.session_lifecycle import dispatch_guard
from agent_workflow.session_service import Service, Store, encode
from agent_workflow.storage import OperationalError, read
from agent_workflow.herdr_backend import ControllerDetached
from agent_workflow import session_bootstrap


class LifecycleTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        root = Path(self.temp.name)
        self.a = RuntimePaths(root / "a", root / "a/state", root / "registry", "a" * 32)
        self.b = RuntimePaths(root / "b", root / "b/state", root / "registry", "b" * 32)
        self.a.project.mkdir()
        self.b.project.mkdir()

    def tearDown(self):
        self.temp.cleanup()

    def command(self, store, action, ident):
        return {"command_id": ident, "action": action,
                "expected_state_revision": store.snapshot()["state_revision"], "payload": {}}

    def test_exit_resume_races_and_replay_are_project_scoped(self):
        a, b = Store(self.a), Store(self.b)
        stop = self.command(a, "stop_session", "exit-1")
        first = a.lifecycle_command(stop)
        self.assertEqual(first["session"]["phase"], "exit-requested")
        self.assertEqual(a.lifecycle_command(stop), first)
        self.assertEqual(a.lifecycle_command({**stop, "payload": {"changed": True}})["code"], "idempotency_conflict")
        self.assertEqual(b.phase()["phase"], "running")
        old = first["session"]
        resume = self.command(a, "resume_session", "resume-1")
        resumed = a.lifecycle_command(resume)
        self.assertEqual(resumed["session"]["generation"], old["generation"] + 1)
        self.assertIsNone(a.update_exit("exit-1", old["generation"], "paused"))
        self.assertEqual(a.phase()["phase"], "running")
        self.assertEqual(a.lifecycle_command(resume), resumed)
        self.assertEqual(a.lifecycle_command({**resume, "command_id": "stale"})["code"], "stale_state")

    def test_pause_first_then_resume_and_dispatch_fence(self):
        store = Store(self.a)
        first = store.lifecycle_command(self.command(store, "stop_session", "exit-2"))["session"]
        self.assertEqual(store.update_exit("exit-2", first["generation"], "draining")["phase"], "draining")
        self.assertEqual(store.update_exit("exit-2", first["generation"], "paused")["phase"], "paused")
        service = Service(self.a, self.a.state / "ipc.sock", "/tmp/no-herdr", "test", "/bin/false")
        self.assertEqual(service.store.phase()["phase"], "paused")
        token = bind(self.a)
        try:
            with self.assertRaises(ControllerDetached):
                with dispatch_guard(): pass
        finally:
            unbind(token)
        store.lifecycle_command(self.command(store, "resume_session", "resume-2"))
        token = bind(self.a)
        try:
            with dispatch_guard(): pass
        finally:
            unbind(token)

    def test_checkpoint_rebuild_keeps_decision_provenance(self):
        service = Service(self.a, self.a.state / "ipc.sock", "/tmp/no-herdr", "test", "/bin/false")
        store = service.store
        with store.open() as db:
            db.execute("CREATE TABLE app_responses(run_id TEXT,request_id TEXT,payload TEXT)")
            db.execute("INSERT INTO app_responses VALUES (?,?,?)", ("run-1", "A1", json.dumps({
                "outcome": "approved", "answer": "Approved with unresolved findings", "artifact_sha256": "a" * 64})))
            db.execute("INSERT INTO session_admissions VALUES (?,?,?,?)",("task-1","admit-1","plan-sha",
                json.dumps({"context":".wayward-wrangler/context/selection.toml",
                            "context_snapshot_sha256":"selected-sha",
                            "cross_imports":[{"grant_id":"grant-1","item_sha256":"item-sha"}],
                            "max_worker_invocations":5,"review_limit":2})))
        checkpoint = service.checkpoint()
        content=read(checkpoint["path"])
        self.assertEqual(content["decision_refs"][0]["artifact_sha256"], "a" * 64)
        self.assertEqual(content["context_refs"][0]["grants"][0]["grant_id"],"grant-1")
        self.assertEqual(content["budget_refs"][0]["review_limit"],2)
        (self.a.state / "session/checkpoints/latest.json").unlink()
        rebuilt = Service(self.a, self.a.state / "ipc.sock", "/tmp/no-herdr", "test", "/bin/false").checkpoint()
        self.assertEqual(checkpoint["sha256"], rebuilt["sha256"])

    def test_concurrent_clients_get_one_authoritative_revision(self):
        store = Store(self.a)
        stop = self.command(store, "stop_session", "exit-race")
        outcomes = []
        threads = [threading.Thread(target=lambda: outcomes.append(store.lifecycle_command(stop))) for _ in range(2)]
        for thread in threads: thread.start()
        for thread in threads: thread.join()
        self.assertEqual(outcomes[0], outcomes[1])
        self.assertEqual(store.snapshot()["state_revision"], 1)

    def test_idle_service_exits_after_durable_checkpoint(self):
        service=Service(self.a,self.a.state/"session/ipc.sock","/tmp/no-herdr","test","/bin/false")
        thread=threading.Thread(target=lambda:asyncio.run(service.run()),daemon=True)
        thread.start()
        until=time.monotonic()+3
        while not service.ipc.exists() and time.monotonic()<until: time.sleep(.02)
        self.assertTrue(service.ipc.exists())
        client=socket.socket(socket.AF_UNIX)
        client.connect(str(service.ipc))
        client.sendall(encode("hello","held-client",{"protocol":{"major":1},"release_id":"test",
                            "project_key":self.a.key}))
        self.assertIn(b"hello_result",client.recv(4096))
        result=service.lifecycle_command(self.command(service.store,"stop_session","exit-idle"))
        self.assertEqual(result["status"],"accepted")
        thread.join(timeout=4)
        self.assertFalse(thread.is_alive(),"service stayed running after its idle exit checkpoint")
        self.assertEqual(service.store.phase()["phase"],"paused")
        self.assertTrue((self.a.state/"session/shutdown-completions/1-exit-idle.json").exists())
        client.close()

    def test_resume_during_drain_supersedes_old_exit_and_keeps_service(self):
        service=Service(self.a,self.a.state/"session/ipc.sock","/tmp/no-herdr","test","/bin/false")
        thread=threading.Thread(target=lambda:asyncio.run(service.run()),daemon=True)
        thread.start()
        until=time.monotonic()+3
        while not service.ipc.exists() and time.monotonic()<until: time.sleep(.02)
        stopped=service.lifecycle_command(self.command(service.store,"stop_session","old-exit"))
        resumed=service.lifecycle_command(self.command(service.store,"resume_session","new-resume"))
        self.assertEqual(resumed["status"],"accepted")
        self.assertEqual(resumed["session"]["generation"],stopped["session"]["generation"]+1)
        time.sleep(1.3)
        self.assertTrue(thread.is_alive())
        self.assertEqual(service.store.phase()["phase"],"running")
        service.lifecycle_command(self.command(service.store,"stop_session","final-exit"))
        thread.join(timeout=4)
        self.assertFalse(thread.is_alive())

    def test_stalled_and_uncertain_notices_do_not_fabricate_completion(self):
        for suffix, report, kind in (("live",{"operation":"worker-1","status":"active","runner_alive":True},"attention-required"),
                                     ("ambiguous",{"operation":"worker-2","status":"uncertain","detail":"claim missing"},"execution-uncertain")):
            paths=RuntimePaths(self.a.project,self.a.state/suffix,self.a.registry,self.a.key)
            service=Service(paths,paths.state/"ipc.sock","/tmp/no-herdr","test","/bin/false")
            service.store.lifecycle_command(self.command(service.store,"stop_session","exit-"+suffix))
            with service.store.open() as db:
                phase=service.store._phase(db)
                phase["deadline"]=time.time()-1
                db.execute("UPDATE session_metadata SET value=? WHERE name='lifecycle_v1'",(json.dumps(phase),))
            service._inflight=lambda:[report]
            worker=threading.Thread(target=service._drain_loop,daemon=True)
            worker.start()
            until=time.monotonic()+3
            while not service.store.phase().get("notice") and time.monotonic()<until: time.sleep(.02)
            self.assertEqual(service.store.phase()["notice"]["kind"],kind)
            self.assertEqual(service.store.phase()["phase"],"draining")
            if kind=="attention-required":
                service.store.lifecycle_command(self.command(service.store,"resume_session","resume-"+suffix))
            worker.join(timeout=2)
            self.assertFalse(worker.is_alive())

    def test_lost_cli_reply_reuses_saved_command_id(self):
        paths=self.a
        ready={"status":"ready","project":str(paths.project),"project_key":paths.key,
               "state":str(paths.state),"unit":"test","ipc":"/tmp/fake"}
        args=SimpleNamespace(project=str(paths.project),state_dir=str(paths.state),registry_dir=str(paths.registry),
                             action="stop_session",release_id="test")
        calls=[]
        def fake_request(_ipc,_key,_release,kind,body):
            calls.append((kind,body))
            if kind=="snapshot":return {"state_revision":0}
            if kind=="command_get":
                return {"status":"not_found"} if len([item for item in calls if item[0]=="command_get"])==1 else {"status":"accepted"}
            if kind=="command":raise TimeoutError("reply lost after commit")
            raise AssertionError(kind)
        with patch.object(session_bootstrap,"prepare",return_value=ready),patch.object(session_bootstrap,"load",return_value=paths),patch.object(session_bootstrap,"_request",side_effect=fake_request):
            first=session_bootstrap.lifecycle(args)
            self.assertEqual(first["status"],"delivery_uncertain")
            saved=read(paths.state/"session/cli-lifecycle-pending.json")
            second=session_bootstrap.lifecycle(args)
        self.assertEqual(second["status"],"accepted")
        self.assertEqual(saved["command_id"],first["command_id"])
        self.assertEqual(calls[-1][1]["command_id"],first["command_id"])
        self.assertFalse((paths.state/"session/cli-lifecycle-pending.json").exists())

    def test_accepted_resume_replay_survives_new_ambiguous_evidence(self):
        service=Service(self.a,self.a.state/"ipc.sock","/tmp/no-herdr","test","/bin/false")
        service.store.lifecycle_command(self.command(service.store,"stop_session","old-exit"))
        body=self.command(service.store,"resume_session","saved-resume")
        first=service.lifecycle_command(body)
        self.assertEqual(first["status"],"accepted")
        with patch.object(service,"reconcile_before_resume",return_value="new ambiguous evidence"):
            self.assertEqual(service.lifecycle_command(body),first)
            self.assertEqual(service.lifecycle_command({**body,"payload":{"changed":True}})["code"],"idempotency_conflict")
        service.shutting_down=True
        self.assertEqual(service.lifecycle_command(body),first)


if __name__ == "__main__":
    unittest.main()
