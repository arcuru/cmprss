"""Offline release assembly and released-caller regressions.

Every subprocess boundary in release.py is stubbed; unexpected commands fail.
Run with: python3 -m unittest discover -s .github/scripts -p 'test_*.py' -v
"""

import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import yaml

ROOT = Path(os.environ.get("WORKFLOW_ROOT", Path(__file__).resolve().parents[2]))
SCRIPT = Path(os.environ.get("RELEASE_SCRIPT", Path(__file__).with_name("release.py")))
spec = importlib.util.spec_from_file_location("release", SCRIPT)
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)
SHA = "a" * 40
TAG = "v1.2.3"
PIN = "3cee03dd694bcbfe4a64ae899f4b9a7cbdfe357c"


class AssemblyTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.directory = Path(self.tmp.name)
        self.output = self.directory / "output"
        self.data = {
            "id": 7,
            "tag_name": TAG,
            "target_commitish": "main",
            "draft": True,
            "immutable": False,
            "assets": [],
        }
        self.bytes = {}
        self.calls = []
        self.tag_sha = SHA
        self.annotated = False
        self.download_failure = False
        self.local_sha = SHA
        self.version = "1.2.3"
        self.ancestor = True
        self.partial_upload = False
        for target in release.TARGETS:
            archive, checksum = release.names(TAG, target)
            content = f"binary for {target}".encode()
            (self.directory / archive).write_bytes(content)
            (self.directory / checksum).write_text(
                f"{hashlib.sha256(content).hexdigest()}  {archive}\n"
            )

    def add_asset(self, name, content):
        self.data["assets"] = [a for a in self.data["assets"] if a["name"] != name]
        asset_id = len(self.bytes) + 1
        self.bytes[asset_id] = content
        self.data["assets"].append(
            {"id": asset_id, "name": name, "size": len(content), "state": "uploaded"}
        )

    def complete(self):
        for path in self.directory.glob("*.tar.gz*"):
            self.add_asset(path.name, path.read_bytes())

    def fake_output(self, args, text):
        self.assertTrue(text)
        args = tuple(args)
        self.calls.append(args)
        if args == ("git", "rev-parse", f"refs/tags/{TAG}^{{commit}}"):
            return self.local_sha
        if args == ("git", "show", f"{self.local_sha}:Cargo.toml"):
            return f'[package]\nversion = "{self.version}"\n'
        if args == ("gh", "api", f"repos/arcuru/cmprss/git/ref/tags/{TAG}"):
            obj = {"sha": self.tag_sha, "type": "commit", "url": "example"}
            if self.annotated:
                obj.update(sha="c" * 40, type="tag")
            return json.dumps({"object": obj})
        if args == ("gh", "api", f"repos/arcuru/cmprss/git/tags/{'c' * 40}"):
            return json.dumps({"object": {"sha": self.tag_sha, "type": "commit"}})
        if args == ("gh", "api", f"repos/arcuru/cmprss/releases/tags/{TAG}"):
            return json.dumps(self.data)
        if args[:3] == ("gh", "release", "upload"):
            self.assertTrue(self.data["draft"], "attempt to mutate a published release")
            self.assertEqual(args[3], TAG)
            self.assertEqual(args[-3:], ("--clobber", "--repo", "arcuru/cmprss"))
            for path in args[4:-3]:
                p = Path(path)
                self.add_asset(p.name, p.read_bytes())
                if self.partial_upload:
                    raise subprocess.CalledProcessError(1, args)
            return ""
        if args == (
            "gh",
            "api",
            "--method",
            "PATCH",
            "repos/arcuru/cmprss/releases/7",
            "-F",
            "draft=false",
        ):
            self.assertTrue(self.data["draft"], "second publication attempted")
            self.assertEqual(len(self.data["assets"]), 4)
            self.data.update(draft=False, immutable=True)
            return json.dumps(self.data)
        self.fail(f"unexpected command (network is forbidden): {args}")

    def fake_run(self, args, check, stdout=None):
        self.assertTrue(check)
        self.calls.append(tuple(args))
        if args == [
            "git",
            "merge-base",
            "--is-ancestor",
            self.local_sha,
            "origin/main",
        ]:
            if not self.ancestor:
                raise subprocess.CalledProcessError(1, args)
            return
        if args[:2] == ["gh", "api"] and args[2].startswith(
            "repos/arcuru/cmprss/releases/assets/"
        ):
            self.assertEqual(args[3:], ["-H", "Accept: application/octet-stream"])
            if self.download_failure:
                raise subprocess.CalledProcessError(1, args)
            stdout.write(self.bytes[int(args[2].rsplit("/", 1)[1])])
            return
        self.fail(f"unexpected command (network is forbidden): {args}")

    def invoke(self, stage, target=None, sha=SHA, release_id="7", tag=TAG):
        args = [
            str(SCRIPT),
            stage,
            "--tag",
            tag,
            "--sha",
            sha,
            "--release-id",
            release_id,
            "--directory",
            str(self.directory),
        ]
        if target:
            args += ["--target", target]
        with (
            patch.object(sys, "argv", args),
            patch.dict(
                os.environ,
                {
                    "GITHUB_REPOSITORY": "arcuru/cmprss",
                    "GITHUB_OUTPUT": str(self.output),
                },
            ),
            patch.object(release.subprocess, "check_output", self.fake_output),
            patch.object(release.subprocess, "run", self.fake_run),
            patch("sys.stdout", new_callable=io.StringIO),
        ):
            release.main()

    def writes(self):
        return [
            c
            for c in self.calls
            if c[:3] == ("gh", "release", "upload") or "PATCH" in c
        ]

    def test_prepare_pins_commit_release_and_draft(self):
        self.invoke("prepare", sha="", release_id="")
        self.assertEqual(
            self.output.read_text(), f"sha={SHA}\nrelease_id=7\ndraft=true\ntag={TAG}\n"
        )
        self.assertEqual(self.writes(), [])

    def test_annotated_release_plz_tag_is_peeled(self):
        self.annotated = True
        self.invoke("prepare")
        self.assertIn(f"sha={SHA}\n", self.output.read_text())
        self.assertEqual(self.writes(), [])

    def test_asset_download_failure_suppresses_publication(self):
        self.complete()
        self.download_failure = True
        with self.assertRaises(subprocess.CalledProcessError):
            self.invoke("finalize")
        self.assertEqual(self.writes(), [])

    def test_all_legs_upload_before_single_publication_and_retry_is_noop(self):
        for target in release.TARGETS:
            self.invoke("upload", target)
            self.assertTrue(self.data["draft"])
        self.invoke("finalize")
        self.invoke("finalize")
        self.assertEqual(len(self.writes()), 3)
        self.assertIn("PATCH", self.writes()[-1])
        self.assertFalse(self.data["draft"])

    def test_partial_matrix_cannot_publish(self):
        self.invoke("upload", release.TARGETS[0])
        with self.assertRaisesRegex(ValueError, "required asset"):
            self.invoke("finalize")
        self.assertTrue(self.data["draft"])
        self.assertEqual(len(self.writes()), 1)

    def test_failed_partial_upload_can_retry_draft(self):
        self.partial_upload = True
        with self.assertRaises(subprocess.CalledProcessError):
            self.invoke("upload", release.TARGETS[0])
        with self.assertRaises(ValueError):
            self.invoke("finalize")
        self.partial_upload = False
        for target in release.TARGETS:
            self.invoke("upload", target)
        self.invoke("finalize")
        self.assertFalse(self.data["draft"])

    def test_missing_corrupt_empty_or_incomplete_asset_suppresses_publish(self):
        self.complete()
        original = copy.deepcopy(self.data)
        original_bytes = self.bytes.copy()
        for defect in ("missing", "corrupt", "empty", "uploading", "duplicate"):
            with self.subTest(defect=defect):
                self.data = copy.deepcopy(original)
                self.bytes = original_bytes.copy()
                self.calls = []
                if defect == "missing":
                    self.data["assets"].pop()
                elif defect == "corrupt":
                    self.bytes[self.data["assets"][0]["id"]] = b"corrupt"
                elif defect == "empty":
                    self.data["assets"][0]["size"] = 0
                elif defect == "uploading":
                    self.data["assets"][0]["state"] = "new"
                else:
                    self.data["assets"].append(self.data["assets"][0])
                with self.assertRaises(ValueError):
                    self.invoke("finalize")
                self.assertEqual(self.writes(), [])

    def test_complete_published_release_is_read_only_for_every_stage(self):
        self.complete()
        self.data.update(draft=False, immutable=True)
        for stage in ("prepare", "upload", "finalize"):
            self.invoke(stage, release.TARGETS[0])
        self.assertEqual(self.writes(), [])

    def test_incomplete_published_release_fails_without_writes(self):
        self.data.update(draft=False, immutable=True)
        for stage in ("prepare", "upload", "finalize"):
            with self.subTest(stage=stage), self.assertRaisesRegex(ValueError, "owner"):
                self.invoke(stage, release.TARGETS[0])
        self.assertEqual(self.writes(), [])

    def test_mutable_published_release_is_not_treated_as_verified(self):
        self.complete()
        self.data.update(draft=False, immutable=False)
        with self.assertRaisesRegex(ValueError, "not immutable"):
            self.invoke("upload", release.TARGETS[0])
        self.assertEqual(self.writes(), [])

    def test_moved_local_or_remote_tag_and_non_main_source_fail(self):
        for defect in ("local", "remote", "ancestry", "version", "metadata", "id"):
            with self.subTest(defect=defect):
                self.local_sha = self.tag_sha = SHA
                self.ancestor = True
                self.version = "1.2.3"
                self.data.update(target_commitish="main", id=7)
                self.calls = []
                if defect == "local":
                    self.local_sha = "b" * 40
                elif defect == "remote":
                    self.tag_sha = "b" * 40
                elif defect == "ancestry":
                    self.ancestor = False
                elif defect == "version":
                    self.version = "9.9.9"
                elif defect == "metadata":
                    self.data["target_commitish"] = "b" * 40
                else:
                    self.data["id"] = 8
                with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                    self.invoke("upload", release.TARGETS[0])
                self.assertEqual(self.writes(), [])

    def test_assembly_requires_prepared_coordinates_and_valid_tag(self):
        for kwargs in ({"sha": ""}, {"release_id": ""}, {"tag": "../main"}):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                self.invoke("upload", release.TARGETS[0], **kwargs)
        self.assertEqual(self.writes(), [])


class WorkflowTests(unittest.TestCase):
    def load(self, name):
        # BaseLoader preserves 'on', rather than treating it as a YAML 1.1 bool.
        return yaml.load(
            (ROOT / f".github/workflows/{name}.yml").read_text(), Loader=yaml.BaseLoader
        )

    def test_draft_creation_and_explicit_main_dispatch(self):
        config = tomllib.loads((ROOT / ".config/release-plz.toml").read_text())
        self.assertIs(config["workspace"].get("git_release_draft"), True)
        job = self.load("release-plz")["jobs"]["release-plz-release"]
        self.assertEqual(job["environment"], "publish")
        self.assertEqual(job["concurrency"]["cancel-in-progress"], "false")
        dispatch = next(
            s for s in job["steps"] if s.get("name") == "Trigger publish workflow"
        )
        self.assertEqual(
            dispatch["if"], "steps.release-plz.outputs.releases_created == 'true'"
        )
        self.assertIn(
            '--ref main -f tag_name="$TAG" -f commit_sha="$SHA"', dispatch["run"]
        )
        self.assertIn("refs/tags/$TAG^{commit}", dispatch["run"])
        self.assertTrue(
            any("crates-io-auth-action@" in s.get("uses", "") for s in job["steps"])
        )

    def test_join_failure_and_cancellation_contract(self):
        workflow = self.load("publish")
        self.assertEqual(workflow["concurrency"]["cancel-in-progress"], "false")
        jobs = workflow["jobs"]
        self.assertIn("github.ref == 'refs/heads/main'", jobs["prepare"]["if"])
        self.assertIn(
            "github.event.workflow_run.conclusion == 'success'", jobs["prepare"]["if"]
        )
        self.assertEqual(set(jobs["finalize"]["needs"]), {"prepare", "binaries"})
        self.assertEqual(jobs["snap"]["needs"], "prepare")
        self.assertEqual(jobs["snap"]["permissions"], {"contents": "read"})
        self.assertIn("needs.binaries.result == 'success'", jobs["finalize"]["if"])
        self.assertNotIn("always()", jobs["finalize"]["if"])
        self.assertIn("needs.prepare.outputs.draft == 'true'", jobs["binaries"]["if"])
        self.assertEqual(
            {x["target"] for x in jobs["binaries"]["strategy"]["matrix"]["include"]},
            set(release.TARGETS),
        )
        self.assertEqual(workflow["on"]["release"]["types"], ["published"])
        for name, job in jobs.items():
            self.assertIn("github.repository_owner == 'arcuru'", job["if"])
            self.assertNotIn("continue-on-error", job)
            for step in job["steps"]:
                self.assertNotIn("continue-on-error", step)
                if "run" in step:
                    self.assertNotIn("${{", step["run"])
                    self.assertNotIn("gh release", step["run"])
                    if "finalize --tag" in step["run"]:
                        self.assertEqual(name, "finalize")
                if "checkout@" in step.get("uses", ""):
                    self.assertEqual(step["with"]["persist-credentials"], "false")

    def test_all_shared_pins_and_released_caller_contracts(self):
        callers = {
            "update-hold",
            "flake-update",
            "cargo-update",
            "actions-update",
            "security-audit",
            "codeberg-mirror",
            "dependency-hold",
        }
        found = set()
        for path in (ROOT / ".github").rglob("*.yml"):
            for line in path.read_text().splitlines():
                if "uses: arcuru/actions/" in line:
                    self.assertTrue(line.endswith(f"@{PIN} # v0.5.0"), line)
                    found.add(path.stem)
        self.assertEqual(found, callers)
        for name in callers:
            workflow = self.load(name)
            job = next(iter(workflow["jobs"].values()))
            if name != "dependency-hold":
                self.assertEqual(job["if"], "github.repository_owner == 'arcuru'")
            if name in (
                "flake-update",
                "cargo-update",
                "actions-update",
                "update-hold",
            ):
                self.assertEqual(job["with"]["environment"], "automation")
                if name != "update-hold":
                    self.assertEqual(
                        job["secrets"], {"PAT_TOKEN": "${{ secrets.PAT_TOKEN }}"}
                    )
            if name == "update-hold":
                self.assertEqual(job["secrets"], "inherit")
                self.assertNotIn("verify-pins", job["with"])
                self.assertEqual(
                    workflow["permissions"],
                    {"contents": "read", "pull-requests": "write"},
                )
            elif name == "dependency-hold":
                self.assertIn("unlabeled", workflow["on"]["pull_request"]["types"])
                self.assertEqual(
                    workflow["permissions"],
                    {"contents": "read", "pull-requests": "read"},
                )


if __name__ == "__main__":
    unittest.main()
