"""Assemble assets only in a draft; published immutable releases are read-only."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib

TARGETS = ("x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl")


def run(*args):
    return subprocess.check_output(args, text=True).strip()


def api(repo, path):
    return json.loads(run("gh", "api", f"repos/{repo}/{path}"))


def require(condition, message):
    if not condition:
        raise ValueError(message)


def inspect(repo, tag, expected_sha, release_id):
    require(
        re.fullmatch(r"v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", tag),
        "invalid version tag",
    )
    sha = run("git", "rev-parse", f"refs/tags/{tag}^{{commit}}")
    require(re.fullmatch(r"[0-9a-f]{40}", sha), "invalid tag commit")
    if expected_sha:
        require(sha == expected_sha, "tag moved from the requested commit")
    # Re-read the actual remote tag, not just checkout's potentially stale refs.
    obj = api(repo, f"git/ref/tags/{tag}")["object"]
    while obj["type"] == "tag":
        obj = api(repo, f"git/tags/{obj['sha']}")["object"]
    require(
        obj["type"] == "commit" and obj["sha"] == sha,
        "remote tag conflicts with checkout",
    )
    subprocess.run(
        ["git", "merge-base", "--is-ancestor", sha, "origin/main"], check=True
    )
    manifest = tomllib.loads(run("git", "show", f"{sha}:Cargo.toml"))
    require(
        tag == f"v{manifest['package']['version']}",
        "tag does not match package version",
    )
    release = api(repo, f"releases/tags/{tag}")
    require(release["tag_name"] == tag, "release tag mismatch")
    if release_id:
        require(
            str(release["id"]) == release_id, "release was replaced during assembly"
        )
    # release-plz uses main here; once a tag exists GitHub ignores target_commitish.
    # The peeled tag above, pinned across jobs, is the source of truth.
    require(
        release["target_commitish"] in ("main", sha),
        "conflicting release target_commitish",
    )
    require(
        release["draft"] or release.get("immutable") is True,
        "published release is not immutable; owner inspection required",
    )
    return sha, release


def names(tag, target):
    archive = f"cmprss-{tag}-{target}.tar.gz"
    return archive, f"{archive}.sha256"


def verify(repo, tag, release):
    required = [name for target in TARGETS for name in names(tag, target)]
    assets = release["assets"]
    for name in required:
        matches = [asset for asset in assets if asset["name"] == name]
        require(
            len(matches) == 1,
            f"missing/duplicate required asset {name}; retry draft assembly or ask the owner to inspect a published release",
        )
        require(
            matches[0]["state"] == "uploaded" and matches[0]["size"] > 0,
            f"incomplete asset {name}",
        )
    with tempfile.TemporaryDirectory() as tmp:
        for name in required:
            asset = next(asset for asset in assets if asset["name"] == name)
            path = Path(tmp, name)
            with path.open("wb") as output:
                subprocess.run(
                    [
                        "gh",
                        "api",
                        f"repos/{repo}/releases/assets/{asset['id']}",
                        "-H",
                        "Accept: application/octet-stream",
                    ],
                    stdout=output,
                    check=True,
                )
        for target in TARGETS:
            archive, checksum = names(tag, target)
            with Path(tmp, archive).open("rb") as source:
                digest = hashlib.file_digest(source, "sha256").hexdigest()
            require(
                Path(tmp, checksum).read_text() == f"{digest}  {archive}\n",
                f"checksum mismatch for {archive}; owner inspection required",
            )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", choices=("prepare", "upload", "finalize"))
    parser.add_argument("--tag", required=True)
    parser.add_argument("--sha", default="")
    parser.add_argument("--release-id", default="")
    parser.add_argument("--target", choices=TARGETS)
    parser.add_argument("--directory", type=Path, default=Path("."))
    args = parser.parse_args()
    require(
        args.stage == "prepare" or (args.sha and args.release_id),
        "assembly requires the prepared commit and release ID",
    )
    repo = os.environ["GITHUB_REPOSITORY"]
    sha, release = inspect(repo, args.tag, args.sha, args.release_id)
    if not release["draft"]:
        verify(repo, args.tag, release)
        print("Complete immutable release verified; no release or asset writes.")
    elif args.stage == "upload":
        require(args.target is not None, "upload requires a target")
        archive, checksum = names(args.tag, args.target)
        # Clobber is safe only while draft, under the workflow's per-tag lock.
        # Re-running a failed leg replaces that leg's partial draft assets.
        run(
            "gh",
            "release",
            "upload",
            args.tag,
            str(args.directory / archive),
            str(args.directory / checksum),
            "--clobber",
            "--repo",
            repo,
        )
    elif args.stage == "finalize":
        verify(repo, args.tag, release)
        # Recheck immediately before the sole publication write. Never move a tag.
        _, current = inspect(repo, args.tag, sha, str(release["id"]))
        if current["draft"]:
            run(
                "gh",
                "api",
                "--method",
                "PATCH",
                f"repos/{repo}/releases/{release['id']}",
                "-F",
                "draft=false",
            )
        _, published = inspect(repo, args.tag, sha, str(release["id"]))
        require(not published["draft"], "release is still a draft")
        verify(repo, args.tag, published)
        print("Published complete immutable release and verified readback.")
    if args.stage == "prepare":
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write(
                f"sha={sha}\nrelease_id={release['id']}\ndraft={str(release['draft']).lower()}\ntag={args.tag}\n"
            )


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, subprocess.CalledProcessError) as error:
        raise SystemExit(
            f"Release assembly stopped: {error}. Inspect the release/tag; do not replace published assets."
        ) from error
