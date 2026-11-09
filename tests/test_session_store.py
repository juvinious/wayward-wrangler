import tempfile
import threading
import unittest
from pathlib import Path
from unittest.mock import patch
from types import SimpleNamespace

from agent_workflow import ownership
from agent_workflow.runtime import RuntimePaths, bind, unbind
from agent_workflow.session_service import Service, Store
from agent_workflow.storage import OperationalError, atomic, digest


class SessionStoreTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        root = Path(self.temp.name)
        self.a = RuntimePaths(root / "a", root / "a/state", root / "registry", "a" * 32)
        self.b = RuntimePaths(root / "b", root / "b/state", root / "registry", "b" * 32)
        self.a.project.mkdir()
        self.b.project.mkdir()

    def tearDown(self): self.temp.cleanup()

    def test_identical_ids_are_project_local(self):
        a, b = Store(self.a), Store(self.b)
        command = {"command_id": "same-id", "action": "submit_message",
                   "expected_state_revision": 0, "payload": {"text": "A task"}}
        a_result = a.command(command)
        b_result = b.command({**command, "payload": {"text": "B task"}})
        self.assertEqual(a_result["status"], "accepted")
        self.assertEqual(b_result["status"], "accepted")
        self.assertEqual(a_result["task_id"], b_result["task_id"])
        self.assertEqual(a.command(command), a_result)
        self.assertEqual(a.command({**command, "payload": {"text": "changed"}})["code"], "idempotency_conflict")
        for store, task_id in ((a, a_result["task_id"]), (b, b_result["task_id"])):
            task = store.task(task_id)
            call = store.reserve_coordinator(task, "packet")
            store.publish_proposal(task_id, call["call_id"], {"proposal": {
                "task_id": task_id, "task_version": 1,
                "message_sha256": digest(task["body"]), "brief": task["body"]}}, "result")
            result = store.admit({"command_id": "admit-same", "action": "admit_task",
                                  "expected_task_version": 1, "expected_state_revision": 2,
                                  "payload": {"task_id": task_id}}, task_id,
                                 {"run_id": "run-same", "project_key": store.paths.key})
            self.assertEqual(result["status"], "accepted")
            self.assertEqual(store.admit({"command_id":"admit-same","action":"admit_task",
                "expected_task_version":1,"expected_state_revision":2,
                "payload":{"task_id":"other"}},task_id,{"run_id":"run-same","project_key":store.paths.key})["code"],
                "idempotency_conflict")
            self.assertEqual(store.next_ready()["task_id"], task_id)
            store.activate(task_id, "run-same")
        a.transition(a_result["task_id"], "run-same", "awaiting-input")
        b.transition(b_result["task_id"], "run-same", "approved")
        self.assertEqual(a.active()["status"], "awaiting-input")
        self.assertIsNone(b.active())
        self.assertEqual(b.task(b_result["task_id"])["status"], "approved")

    def test_coordinator_inspection_distinguishes_call_from_workflow_dispatch(self):
        store = Store(self.a)
        accepted = store.command({"command_id": "task-1", "action": "submit_message",
                                  "expected_state_revision": 0, "payload": {"text": "Draft a decision log"}})
        task_id = accepted["task_id"]
        task = store.task(task_id)
        call = store.reserve_coordinator(task, "frozen-packet")
        store.stop_coordinator(task_id, call["call_id"], "uncertain", "runner exited")
        detail = Service(self.a, self.a.state / "ipc.sock", "/tmp/none", "test", "/bin/false").task_detail(task_id)
        self.assertIsNone(detail["task"]["run_id"])
        self.assertEqual(detail["coordinator"]["call"]["status"], "uncertain")
        self.assertFalse(detail["coordinator"]["permit_recorded"])
        self.assertFalse(detail["coordinator"]["child_recorded"])

    def test_old_service_generation_cannot_dispatch(self):
        token = bind(self.a)
        owner = ownership.bind_service(1)
        try:
            atomic(self.a.state / "session/service-generation.json", {"generation": 2})
            with self.assertRaisesRegex(OperationalError, "Stale project service generation"):
                ownership.check()
        finally:
            ownership.unbind_service(owner)
            unbind(token)

    def test_next_driver_waits_for_previous_thread(self):
        service = Service(self.a, self.a.state / "session/ipc.sock", "/tmp/unused-herdr.sock", "test", "/bin/false")
        first_started, first_release, second_started = threading.Event(), threading.Event(), threading.Event()
        seen = []
        def drive(task_id, run_id):
            seen.append((task_id, run_id))
            if task_id == "one":
                first_started.set()
                first_release.wait(2)
            else:
                second_started.set()
        service._drive = drive
        self.assertTrue(service.start_driver("one", "r1"))
        self.assertTrue(first_started.wait(1))
        self.assertFalse(service.start_driver("two", "r2"))
        self.assertFalse(second_started.is_set())
        # A retired task must not be dispatched even if it was scheduled.
        service.store.task = lambda task_id: {"run_id": "r2", "status": "active"} if task_id == "two" else None
        first_release.set()
        self.assertTrue(second_started.wait(2))
        self.assertEqual(seen, [("one", "r1"), ("two", "r2")])

    def test_authorized_second_task_waits_for_first_terminal_state(self):
        store = Store(self.a)
        for index in (1, 2):
            command = {"command_id": f"submit-{index}", "action": "submit_message",
                       "expected_state_revision": store.snapshot()["state_revision"],
                       "payload": {"text": f"Task {index}"}}
            task_id = store.command(command)["task_id"]
            task = store.task(task_id)
            call = store.reserve_coordinator(task, f"packet-{index}")
            store.publish_proposal(task_id, call["call_id"], {"proposal": {
                "task_id": task_id, "task_version": 1,
                "message_sha256": digest(task["body"]), "brief": task["body"]}}, f"result-{index}")
            plan = {"run_id": f"same-run-{index}", "project_key": self.a.key}
            admitted = store.admit({"command_id": f"admit-{index}", "action": "admit_task",
                                    "expected_task_version": 1,
                                    "expected_state_revision": store.task(task_id)["state_revision"],
                                    "payload": {"task_id": task_id}}, task_id, plan)
            self.assertEqual(admitted["task_status"], "ready")
            if index == 1:
                first = task_id
                store.activate(first, plan["run_id"])
            else:
                second = task_id
                self.assertEqual(store.next_ready()["task_id"], second)
                with self.assertRaisesRegex(OperationalError, "unresolved"):
                    store.activate(second, plan["run_id"])
        store.transition(first, "same-run-1", "approved")
        store.activate(second, "same-run-2")
        self.assertEqual(store.active()["task_id"], second)

    def test_queued_steering_versions_input_and_refuses_reserved_coordinator(self):
        store=Store(self.a)
        task_id=store.command({"command_id":"initial","action":"submit_message","expected_state_revision":0,
                               "payload":{"text":"old"}})["task_id"]
        change={"command_id":"steer","action":"steer_queued","expected_task_version":1,
                "expected_state_revision":1,"payload":{"task_id":task_id,"text":"new"}}
        self.assertEqual(store.modify_queued(change)["task_version"],2)
        self.assertEqual(store.modify_queued(change)["task_version"],2)
        self.assertEqual(store.task(task_id)["body"],"new")
        stale=store.task(task_id)
        store.reserve_coordinator(stale,"frozen-packet")
        self.assertEqual(store.modify_queued({**change,"command_id":"too-late","expected_task_version":2,
                                             "expected_state_revision":2})["code"],"stale_or_started")
        with self.assertRaisesRegex(OperationalError,"stale"):
            store.reserve_coordinator({**stale,"version":1},"old-packet")

    def test_cancel_undispatched_does_not_touch_other_project(self):
        a,b=Store(self.a),Store(self.b)
        command={"command_id":"same","action":"submit_message","expected_state_revision":0,
                 "payload":{"text":"work"}}
        task_id=a.command(command)["task_id"]
        b.command(command)
        cancelled=a.cancel_undispatched({"command_id":"cancel","action":"cancel_task",
                                         "expected_task_version":1,"expected_state_revision":1,
                                         "payload":{"task_id":task_id}})
        self.assertEqual(cancelled["task_status"],"cancelled")
        self.assertEqual(a.task(task_id)["status"],"cancelled")
        self.assertEqual(b.task(task_id)["status"],"queued")

    def test_missing_workspace_waits_for_verified_gate_rebinding(self):
        service=Service(self.a,self.a.state/'session/ipc.sock','/tmp/missing-herdr.sock','test','/bin/false')
        store=service.store
        task_id=store.command({'command_id':'submit','action':'submit_message','expected_state_revision':0,
                               'payload':{'text':'task'}})['task_id']
        task=store.task(task_id)
        call=store.reserve_coordinator(task,'packet')
        store.publish_proposal(task_id,call['call_id'],{'proposal':{'task_id':task_id,'task_version':1,
             'message_sha256':digest(task['body']),'brief':task['body']}},'result')
        store.admit({'command_id':'admit','expected_task_version':1,'expected_state_revision':2,
                     'payload':{'task_id':task_id}},task_id,{'run_id':'run','project_key':self.a.key})
        store.activate(task_id,'run')
        store.transition(task_id,'run','awaiting-input')
        with patch('agent_workflow.records.get_run',return_value={'status':'awaiting-input','detail':None}), \
             patch('agent_workflow.records.effective_target',return_value={'socket':'/tmp/missing','workspace':'w-old'}), \
             patch('agent_workflow.herdr_backend.Herdr.call',side_effect=OperationalError('workspace_not_found')), \
             patch.object(service,'start_driver') as driver:
            service.resume_active()
            driver.assert_not_called()
            self.assertEqual(store.task(task_id)['status'],'recovering')
        with patch('agent_workflow.records.get_run',return_value={'status':'awaiting-input','detail':None}), \
             patch('agent_workflow.records.effective_target',return_value={'socket':'/tmp/missing','workspace':'w-old'}), \
             patch('agent_workflow.routing.rebind',return_value={'status':'rebound'}) as rebind, \
             patch.object(service,'resume_active') as resume:
            service.reconcile_active_workspace('w-new')
            rebind.assert_called_once_with('run','w-new',allow_answered=True)
            resume.assert_called_once()
            self.assertEqual(store.task(task_id)['status'],'awaiting-input')

    def test_human_answer_is_command_idempotent_and_scoped(self):
        service = Service(self.a, self.a.state / "session/ipc.sock", "/tmp/unused-herdr.sock", "test", "/bin/false")
        store = service.store
        task_id = store.command({"command_id": "submit", "action": "submit_message",
                                 "expected_state_revision": 0, "payload": {"text": "task"}})["task_id"]
        task = store.task(task_id)
        call = store.reserve_coordinator(task, "packet")
        store.publish_proposal(task_id, call["call_id"], {"proposal": {
            "task_id": task_id, "task_version": 1,
            "message_sha256": digest(task["body"]), "brief": task["body"]}}, "result")
        store.admit({"command_id": "admit", "expected_task_version": 1,
                     "expected_state_revision": 2, "payload": {"task_id": task_id}},
                    task_id, {"run_id": "run", "project_key": self.a.key})
        store.activate(task_id, "run")
        store.transition(task_id, "run", "awaiting-input")
        body = {"command_id": "answer-1", "action": "answer_task", "expected_task_version": 1,
                "expected_state_revision": store.task(task_id)["state_revision"],
                "payload": {"task_id": task_id, "request_id": "D-one", "outcome": "answered",
                            "answer": "A", "artifact_sha256": "artifact"}}
        started = threading.Event()
        with patch("agent_workflow.records.response", side_effect=[None, {"request_id": "D-one", "answer": "A"}]), \
             patch("agent_workflow.workflow.answer", return_value={"request_id": "D-one", "answer": "A"}) as answer, \
             patch("agent_workflow.workflow.state", return_value=SimpleNamespace(
                 values={"pending": {"request_id": "D-one"}}, interrupts=[True])), \
             patch.object(service, "start_driver", side_effect=lambda *_: started.set()) as start:
            accepted = service.answer_task(body)
            self.assertEqual(accepted["status"], "accepted")
            self.assertTrue(started.wait(2))
            self.assertEqual(service.answer_task(body), accepted)
            self.assertEqual(service.answer_task({**body, "payload": {**body["payload"], "answer": "B"}})["code"], "idempotency_conflict")
            self.assertEqual(answer.call_count, 1)
            self.assertEqual(start.call_count, 1)


if __name__ == "__main__": unittest.main()
