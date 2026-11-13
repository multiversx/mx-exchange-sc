#!/usr/bin/env python3
"""Codehashes block for reproducible build releases.

Used by reproducible-build.sh and .github/workflows/release.yml.

  summary  Checks the files in a release-assets folder and writes the codehashes
           block. Every <contract>.artifacts.json must list at least one contract,
           and the blake2b hash of each .wasm must match the codehash that sc-meta
           recorded in artifacts.json.

  merge    Puts a codehashes block into an existing release body, between the
           markers. Codehashes already in the body are kept, so contracts can be
           attached to the same release in several runs. Refuses to mix builds
           made from a different commit or with a different Docker image.
"""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

START = "<!-- reproducible-build:start -->"
END = "<!-- reproducible-build:end -->"

IMAGE_RE = re.compile(r"^Built using Docker image: \*\*(.+?)\*\*\.$", re.M)
COMMIT_RE = re.compile(r"^Source commit: `([0-9a-f]{40})`$", re.M)
CODEHASH_RE = re.compile(r"^\*\*(.+?\.wasm)\*\*: `([0-9a-f]{64})`$", re.M)


def fail(message):
    print(f"error: {message}", file=sys.stderr)
    sys.exit(1)


def blake2b_hex(path):
    return hashlib.blake2b(path.read_bytes(), digest_size=32).hexdigest()


def render(image, commit, codehashes):
    lines = [
        START,
        "## Reproducible build",
        "",
        f"Built using Docker image: **{image}**.",
        f"Source commit: `{commit}`",
        "",
        "## Codehashes (blake2b):",
    ]
    lines += [f"**{wasm}**: `{codehash}`" for wasm, codehash in codehashes.items()]
    lines.append(END)
    return "\n".join(lines) + "\n"


def parse_block(text):
    """Returns (image, commit, codehashes) of the block in text, or None."""
    start, end = text.find(START), text.find(END)
    if start == -1 or end == -1 or end < start:
        return None
    block = text[start:end]
    image, commit = IMAGE_RE.search(block), COMMIT_RE.search(block)
    if not image or not commit:
        fail("the reproducible-build block in the release body is malformed")
    return image.group(1), commit.group(1), dict(CODEHASH_RE.findall(block))


def summary(args):
    assets = Path(args.assets)
    artifacts_files = sorted(assets.glob("*.artifacts.json"))
    if not artifacts_files:
        fail(f"no *.artifacts.json in {assets}")

    codehashes = {}
    for artifacts_file in artifacts_files:
        contracts = json.loads(artifacts_file.read_text()).get("contracts") or {}
        if not contracts:
            fail(f"{artifacts_file.name} lists no contract (wrong contract name?)")
        for name, entry in contracts.items():
            wasm = assets / entry["artifacts"]["bytecode"]
            if not wasm.is_file():
                fail(f"{wasm.name} (from {artifacts_file.name}) is missing")
            actual = blake2b_hex(wasm)
            if actual != entry["codehash"]:
                fail(f"{wasm.name}: codehash {actual} != {entry['codehash']} in {artifacts_file.name}")
            codehashes[wasm.name] = actual

    for wasm in sorted(assets.glob("*.wasm")):
        if wasm.name not in codehashes:
            fail(f"{wasm.name} is not listed in any artifacts.json")

    Path(args.output).write_text(render(args.image, args.commit, codehashes))


def merge(args):
    body = Path(args.body).read_text()
    new = parse_block(Path(args.block).read_text())
    if new is None:
        fail(f"no reproducible-build block in {args.block}")
    image, commit, codehashes = new

    old = parse_block(body)
    if old is None:
        merged = body.rstrip() + "\n\n" + render(image, commit, codehashes)
    else:
        old_image, old_commit, old_codehashes = old
        if old_commit != commit:
            fail(f"the release already has files built from commit {old_commit}, not {commit}")
        if old_image != image:
            fail(f"the release already has files built with {old_image}, not {image}")
        old_codehashes.update(codehashes)
        start, end = body.find(START), body.find(END) + len(END)
        merged = body[:start] + render(image, commit, old_codehashes).rstrip("\n") + body[end:]

    Path(args.output).write_text(merged)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)

    p = commands.add_parser("summary", help="check release assets and write the codehashes block")
    p.add_argument("--assets", required=True, help="release-assets folder")
    p.add_argument("--image", required=True, help="Docker image used for the build")
    p.add_argument("--commit", required=True, help="full commit hash that was built")
    p.add_argument("--output", required=True, help="file to write the block to")
    p.set_defaults(run=summary)

    p = commands.add_parser("merge", help="merge a codehashes block into a release body")
    p.add_argument("--body", required=True, help="current release body")
    p.add_argument("--block", required=True, help="block written by 'summary'")
    p.add_argument("--output", required=True, help="file to write the new release body to")
    p.set_defaults(run=merge)

    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
