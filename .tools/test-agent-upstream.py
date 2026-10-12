# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""test-agent-upstream.py - agent-upstream.py 通知面回归（issue#11）。

覆盖 dispatch 降级口径、目标仓探活与可配面（零网络：gh 子进程全 mock）：
- token 未配：skip-token 态，不触子进程（REQ-0017 既有口径）
- 网络端 404（ai-cloud 归档同型失败）：failed 态如实带缘由，不抛不红
- 发送形契约：event_type + client_payload 顶层键形（F5）、GH_TOKEN 注入
- 探活（评审格 O2）：archived 标志位判据（GET 200 不代表可写）、token 不可见分码
- DISPATCH_REPO 空值回退缺省（repo variable 未配时 workflow 透传空串）

跑：uv run --script .tools/test-agent-upstream.py
"""

from __future__ import annotations

import importlib.util
import json
import os
import unittest
from pathlib import Path
from unittest import mock

_SPEC = importlib.util.spec_from_file_location(
    "agent_upstream", Path(__file__).with_name("agent-upstream.py"))
AU = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(AU)

SEEDED = [{"name": "claude", "tier": "agent", "repo": "anthropics/claude-code",
           "tag": "v2.1.296", "version": "2.1.296",
           "assets": [{"name": "claude-linux-x64.tar.gz", "sha256": "AB"}]}]


def _load_with_env(repo_env: str):
    """按指定 AGENT_UPSTREAM_DISPATCH_REPO 重载模块（常量 import 时求值）"""
    env = {**os.environ, "AGENT_UPSTREAM_DISPATCH_REPO": repo_env}
    with mock.patch.dict(os.environ, env, clear=True):
        spec = importlib.util.spec_from_file_location(
            "agent_upstream_env", Path(__file__).with_name("agent-upstream.py"))
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
    return mod


class TokenMissingTest(unittest.TestCase):
    def test_skip_token_without_subprocess(self):
        env = {k: v for k, v in os.environ.items()
               if k != "AGENT_UPSTREAM_DISPATCH_TOKEN"}
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(AU.subprocess, "run") as run:
            self.assertEqual(AU.send_dispatch(SEEDED), "skip-token")
        run.assert_not_called()


class DispatchFailTest(unittest.TestCase):
    def test_404_archived_repo_degrades_not_raises(self):
        proc = mock.Mock(returncode=1, stderr="gh: Not Found (HTTP 404)", stdout="")
        env = {**os.environ, "AGENT_UPSTREAM_DISPATCH_TOKEN": "t"}
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(AU.subprocess, "run", return_value=proc) as run:
            state = AU.send_dispatch(SEEDED)
        self.assertTrue(state.startswith("failed:"))
        self.assertIn("404", state)
        run.assert_called_once()
        argv = run.call_args.args[0]
        self.assertIn(f"repos/{AU.DISPATCH_REPO}/dispatches", argv)
        self.assertEqual(run.call_args.kwargs["env"]["GH_TOKEN"], "t")


class DispatchOkTest(unittest.TestCase):
    def test_ok_state_and_payload_shape(self):
        proc = mock.Mock(returncode=0, stderr="", stdout="")
        env = {**os.environ, "AGENT_UPSTREAM_DISPATCH_TOKEN": "t"}
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(AU.subprocess, "run", return_value=proc) as run:
            self.assertEqual(AU.send_dispatch(SEEDED), "ok")
        payload = json.loads(run.call_args.kwargs["input"])
        self.assertEqual(payload["event_type"], AU.DISPATCH_EVENT)
        self.assertEqual(payload["client_payload"]["tools"], SEEDED)
        self.assertTrue(payload["client_payload"]["run_url"] == "" or
                        payload["client_payload"]["run_url"].startswith("https://"))
        # F5 顶层键契约：event_type 加 client_payload，不超 10 键
        self.assertLessEqual(len(payload), 2)


class ProbeTest(unittest.TestCase):
    def test_token_missing_returns_none_without_probe(self):
        env = {k: v for k, v in os.environ.items()
               if k != "AGENT_UPSTREAM_DISPATCH_TOKEN"}
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(AU.subprocess, "run") as run:
            self.assertIsNone(AU.probe_dispatch_target())
        run.assert_not_called()

    def test_archived_repo_flag_detected_via_get(self):
        # 归档仓 GET 仍 200（.archived=true），POST dispatches 才 404（issue#11 根因）：
        # 探活判据必须取标志位而非 HTTP 200
        proc = mock.Mock(returncode=0, stderr="", stdout="true\n")
        env = {**os.environ, "AGENT_UPSTREAM_DISPATCH_TOKEN": "t"}
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(AU.subprocess, "run", return_value=proc) as run:
            self.assertEqual(AU.probe_dispatch_target(), "archived")
        argv = run.call_args.args[0]
        self.assertEqual(argv, ["gh", "api", f"repos/{AU.DISPATCH_REPO}",
                                "--jq", ".archived"])

    def test_active_repo_probes_ok(self):
        proc = mock.Mock(returncode=0, stderr="", stdout="false\n")
        env = {**os.environ, "AGENT_UPSTREAM_DISPATCH_TOKEN": "t"}
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(AU.subprocess, "run", return_value=proc):
            self.assertEqual(AU.probe_dispatch_target(), "ok")

    def test_invisible_repo_classified_by_code(self):
        # token 不可见私有仓恒 404（GitHub 防存在性探测），须分码上报
        for code in ("401", "403", "404"):
            proc = mock.Mock(returncode=1, stderr=f"gh: Not Found (HTTP {code})", stdout="")
            env = {**os.environ, "AGENT_UPSTREAM_DISPATCH_TOKEN": "t"}
            with mock.patch.dict(os.environ, env, clear=True), \
                    mock.patch.object(AU.subprocess, "run", return_value=proc):
                self.assertEqual(AU.probe_dispatch_target(),
                                 f"inaccessible:http-{code}")


class DispatchRepoFallbackTest(unittest.TestCase):
    def test_empty_env_falls_back_to_default(self):
        mod = _load_with_env("")
        self.assertEqual(mod.DISPATCH_REPO, "raystyle/ai-cloud")

    def test_override_env_respected(self):
        mod = _load_with_env("raystyle/ledger-rs")
        self.assertEqual(mod.DISPATCH_REPO, "raystyle/ledger-rs")


if __name__ == "__main__":
    unittest.main()
