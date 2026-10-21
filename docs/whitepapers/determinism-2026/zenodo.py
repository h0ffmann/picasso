#!/usr/bin/env python3
"""Deposit the whitepaper on Zenodo, as a record of its own (not the picasso repository).

  ZENODO_TOKEN=... python3 zenodo.py draft     # create the draft from zenodo.json, reserve a DOI into doi.txt
  just render paper                            # re-render: the cover's "Cite as" now carries the DOI
  ZENODO_TOKEN=... python3 zenodo.py upload    # attach ../../../PICASSOPAPER.pdf to the draft
  python3 zenodo.py --self-test                # checks zenodo.json and the files, no network

It never publishes: a published DOI cannot be withdrawn, so the author reviews the draft on
zenodo.org and presses Publish. --sandbox targets sandbox.zenodo.org for a rehearsal (its own token).
The token needs the deposit:write scope; keep it in the environment, never in a file here.
"""
import json
import os
import sys
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
META, PDF, STATE, DOI = HERE / "zenodo.json", HERE.parents[2] / "PICASSOPAPER.pdf", HERE / ".zenodo-draft", HERE / "doi.txt"


def api(base, method, path, token, body=None, raw=None):
    data = raw if raw is not None else (json.dumps(body).encode() if body is not None else None)
    req = urllib.request.Request(base + path, data=data, method=method)
    req.add_header("Authorization", f"Bearer {token}")
    req.add_header("Content-Type", "application/octet-stream" if raw is not None else "application/json")
    with urllib.request.urlopen(req) as r:
        return json.loads(r.read() or b"{}")


def check():
    m = json.loads(META.read_text())
    for k in ("title", "upload_type", "publication_type", "description", "creators", "license", "access_right"):
        assert m.get(k), f"zenodo.json: missing {k}"
    assert "doi" not in m and "publication_date" not in m, "zenodo.json: Zenodo sets doi and publication_date"
    assert all(c.get("orcid") for c in m["creators"]), "zenodo.json: every creator needs an ORCID"
    assert len(m["references"]) == 30, "zenodo.json: references out of step with the paper"
    assert PDF.exists(), "PICASSOPAPER.pdf missing: run just render paper"
    return m


def main(argv):
    if "--self-test" in argv:
        check()
        print("zenodo.json ok")
        return
    base = "https://sandbox.zenodo.org/api" if "--sandbox" in argv else "https://zenodo.org/api"
    token = os.environ.get("ZENODO_TOKEN") or sys.exit("ZENODO_TOKEN is not set")
    m = check()
    if "draft" in argv:
        if STATE.exists():
            sys.exit(f"a draft already exists ({STATE.read_text().strip()}); delete {STATE.name} to start over")
        d = api(base, "POST", "/deposit/depositions", token, {"metadata": {**m, "prereserve_doi": True}})
        STATE.write_text(f"{base} {d['id']}\n")
        DOI.write_text(d["metadata"]["prereserve_doi"]["doi"] + "\n")
        print(f"draft {d['id']}: {d['links']['html']}\nreserved DOI {DOI.read_text().strip()}; now run: just render paper")
    elif "upload" in argv:
        base, dep = STATE.read_text().split()
        d = api(base, "GET", f"/deposit/depositions/{dep}", token)
        api(base, "PUT", d["links"]["bucket"].split("/api", 1)[1] + "/" + PDF.name, token, raw=PDF.read_bytes())
        print(f"uploaded {PDF.name}; review and publish at {d['links']['html']}")
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
