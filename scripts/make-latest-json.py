#!/usr/bin/env python3
"""Writes latest.json for a release and attaches it, so installed apps find the update.

Run after every installer and its .sig file are on the release (Windows from CI,
Linux uploaded by hand):

    python3 scripts/make-latest-json.py v1.41.0
"""
import json
import subprocess
import sys
import tempfile
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

REPO = "scottws70/Net-Roster-Logger"

# updater platform key -> end of the installer's asset name
PLATFORMS = {
    "windows-x86_64-nsis": "_x64-setup.exe",
    "windows-x86_64-msi": "_x64_en-US.msi",
    "linux-x86_64-deb": "_amd64.deb",
    "linux-x86_64-rpm": ".x86_64.rpm",
    "linux-x86_64-appimage": "_amd64.AppImage",
}
# fallbacks for an app that doesn't know which installer it came from
FALLBACKS = {"windows-x86_64": "windows-x86_64-nsis", "linux-x86_64": "linux-x86_64-appimage"}


def gh(*args):
    return subprocess.run(["gh", *args], check=True, capture_output=True, text=True).stdout


def main():
    tag = sys.argv[1]
    rel = json.loads(gh("release", "view", tag, "--repo", REPO, "--json", "assets,body,publishedAt"))
    assets = {a["name"]: a["url"] for a in rel["assets"]}
    platforms, missing = {}, []
    for key, suffix in PLATFORMS.items():
        name = next((n for n in assets if n.endswith(suffix)), None)
        if not name or name + ".sig" not in assets:
            missing.append(f"{key} ({suffix} and its .sig)")
            continue
        with urllib.request.urlopen(assets[name + ".sig"]) as r:
            sig = r.read().decode().strip()
        platforms[key] = {"signature": sig, "url": assets[name]}
    if missing:
        sys.exit("Not on the release yet: " + ", ".join(missing))
    for key, target in FALLBACKS.items():
        platforms[key] = platforms[target]
    manifest = {
        "version": tag.lstrip("v"),
        "notes": rel.get("body") or "",
        "pub_date": rel.get("publishedAt") or datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": platforms,
    }
    with tempfile.TemporaryDirectory() as d:
        out = Path(d) / "latest.json"
        out.write_text(json.dumps(manifest, indent=2) + "\n")
        gh("release", "upload", tag, str(out), "--clobber", "--repo", REPO)
    print(f"latest.json attached to {tag} ({len(platforms)} platform entries)")


if __name__ == "__main__":
    main()
