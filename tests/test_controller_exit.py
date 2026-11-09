"""Installed LangGraph executor shutdown test; no Herdr writes or worker launch."""
import os
import signal
import subprocess
import sys
import time
import unittest

class ControllerExit(unittest.TestCase):
    def test_sigint_releases_graph_wait_without_cancelling_execution(self):
        code = r'''
import signal, time
from unittest.mock import patch
from typing import TypedDict
from langgraph.graph import START, END, StateGraph
from agent_workflow import herdr_backend as backend
from agent_workflow.storage import identity
class State(TypedDict):
    done: bool
def node(state):
    print("OBSERVING", flush=True)
    with patch.object(backend, "ensure", return_value={"status":"active","binding":{"pane_id":"unit-fixture"}}), \
         patch.object(backend, "receipt", return_value=None), \
         patch.object(backend, "read", return_value={"runner":identity()}):
        backend.wait("/tmp/unit-fixture-with-no-worker")
    return {"done":True}
def stop(_signal,_frame):
    backend.STOP_OBSERVING.set()
    raise KeyboardInterrupt
signal.signal(signal.SIGINT,stop)
g=StateGraph(State)
g.add_node("observe",node);g.add_edge(START,"observe");g.add_edge("observe",END)
try:
    g.compile().invoke({"done":False})
except KeyboardInterrupt:
    print("DETACHED",flush=True)
'''
        child=subprocess.Popen([sys.executable,"-c",code],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        try:
            # Wait for the graph node to actually start before interrupting.
            self.assertEqual(child.stdout.readline().strip(),"OBSERVING")
            started=time.monotonic()
            child.send_signal(signal.SIGINT)
            out,err=child.communicate(timeout=4)
            self.assertEqual(child.returncode,0,err)
            self.assertIn("DETACHED",out)
            self.assertLess(time.monotonic()-started,3)
        finally:
            if child.poll() is None:
                child.kill()
                child.wait()

if __name__=="__main__":
    unittest.main()
