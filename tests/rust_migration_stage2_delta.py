"""Differential bounded review-delta cases against Python difflib."""
import argparse
import difflib
import hashlib
import json
from pathlib import Path
import random
import subprocess

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("binary",type=Path)
    ap.add_argument("evidence",type=Path)
    args=ap.parse_args()
    binary=args.binary.resolve();evidence=args.evidence.resolve()
    evidence.mkdir(parents=True,exist_ok=False)
    rng=random.Random(721)
    cases=[]
    for index in range(120):
        n=rng.randrange(0,35) if index<100 else rng.randrange(195,260)
        a=[rng.choice(["alpha\n","beta\n","gamma\n","\n","delta\n"])
           for _ in range(n)]
        b=a.copy()
        for _ in range(rng.randrange(0,8)):
            at=rng.randrange(len(b)+1)
            if b and rng.choice([True,False]):b.pop(min(at,len(b)-1))
            else:b.insert(at,rng.choice(["new\n","x\n","\n","alpha\n"]))
        if rng.choice([True,False]) and a:a[-1]=a[-1].rstrip("\n")
        if rng.choice([True,False]) and b:b[-1]=b[-1].rstrip("\n")
        previous="".join(a);current="".join(b)
        expected="".join(difflib.unified_diff(previous.splitlines(True),current.splitlines(True),
            fromfile="previous-reviewed",tofile="current-revision"))
        result=json.loads(subprocess.check_output([str(binary),"review_delta"],
            input=json.dumps({"previous":previous,"current":current}).encode()))
        if not result["ok"] or result["value"]!=expected:
            (evidence/f"mismatch-{index}.json").write_text(json.dumps({"previous":previous,
                "current":current,"expected":expected,"rust":result},indent=2)+"\n")
            raise AssertionError(f"review delta differs in case {index}; retained input")
        cases.append({"case":index,"old_lines":len(a),"new_lines":len(b),
            "delta_sha256":hashlib.sha256(expected.encode()).hexdigest()})
    (evidence/"delta-comparison.json").write_text(json.dumps({"schema_version":1,
        "cases":cases},indent=2)+"\n")
    print("Stage 2 deltas: 120 cases matched Python difflib, including 200+ line inputs")

if __name__=="__main__":main()
