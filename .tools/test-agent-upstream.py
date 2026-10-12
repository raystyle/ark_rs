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
import io
import json
import os
import sys
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
        # token 不可见私有仓恒 404（GitHub 防存在性探测），须分码上报；
        # 锚定 (HTTP <码>) 形防数字串误吸（评审 G5）
        for code in ("401", "403", "404"):
            proc = mock.Mock(returncode=1, stderr=f"gh: Not Found (HTTP {code})", stdout="")
            env = {**os.environ, "AGENT_UPSTREAM_DISPATCH_TOKEN": "t"}
            with mock.patch.dict(os.environ, env, clear=True), \
                    mock.patch.object(AU.subprocess, "run", return_value=proc):
                self.assertEqual(AU.probe_dispatch_target(),
                                 f"inaccessible:http-{code}")

    def test_err_without_code_lands_inaccessible_generic(self):
        # 网络抖动等非分码失败如实报态不抛（评审 G4 面：多行压白截断）
        proc = mock.Mock(returncode=1, stderr="dial tcp: timeout\nretrying", stdout="")
        env = {**os.environ, "AGENT_UPSTREAM_DISPATCH_TOKEN": "t"}
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(AU.subprocess, "run", return_value=proc):
            state = AU.probe_dispatch_target()
        self.assertTrue(state.startswith("inaccessible:"))
        self.assertNotIn("\n", state)


class SendDispatchRobustnessTest(unittest.TestCase):
    def test_empty_stderr_gets_fallback_reason(self):
        # gh 失败但 stderr 空串时兜退出码文案，不出空缘由（评审 G4）
        proc = mock.Mock(returncode=1, stderr="", stdout="")
        env = {**os.environ, "AGENT_UPSTREAM_DISPATCH_TOKEN": "t"}
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(AU.subprocess, "run", return_value=proc):
            state = AU.send_dispatch(SEEDED)
        self.assertEqual(state, "failed:gh exit 1")

    def test_multiline_stderr_collapsed_single_line(self):
        proc = mock.Mock(returncode=1,
                         stderr="gh: Not Found (HTTP 404)\nsecond line", stdout="")
        env = {**os.environ, "AGENT_UPSTREAM_DISPATCH_TOKEN": "t"}
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(AU.subprocess, "run", return_value=proc):
            state = AU.send_dispatch(SEEDED)
        self.assertNotIn("\n", state)
        self.assertIn("HTTP 404", state)


class MainSummaryContractTest(unittest.TestCase):
    """main() 摘要契约（评审 G3）：dispatch 恒出键、dispatch_probe 条件在、
    skip-no-seed 联动、通知失败不进 fails 与退出码；零网络（探活与发送
    全 mock），种子腿以域面 HEAD 全 404 短路前置失败面（claude/codex/kimi
    的 combined/sidecar 校验在下载之后，HEAD 404 即 problem 先行）"""

    def _run_main(self, probe):
        env = {k: v for k, v in os.environ.items()
               if k != "AGENT_UPSTREAM_DISPATCH_TOKEN"}
        if probe is not None:
            env["AGENT_UPSTREAM_DISPATCH_TOKEN"] = "t"
        out = io.StringIO()
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(AU, "probe_dispatch_target",
                                  return_value=probe), \
                mock.patch.object(AU, "gh_latest_tag",
                                  return_value="v0.0.0-test"), \
                mock.patch.object(sys, "stdout", out), \
                mock.patch.object(AU, "http_head", return_value=404), \
                mock.patch.object(AU, "http_get", return_value=(-1, b"net")):
            rc = AU.main()
        return rc, out.getvalue()

    def test_probe_archived_warns_and_summary_carries_state(self):
        rc, out = self._run_main(probe="archived")
        self.assertNotEqual(rc, 0)  # 种子腿下载失败（零网络假败）应红
        self.assertIn("探活异常", out)
        self.assertIn('"dispatch": "skip-no-seed"', out)
        self.assertIn('"dispatch_probe": "archived"', out)

    def test_no_token_no_probe_key_in_summary(self):
        rc, out = self._run_main(probe=None)
        self.assertNotEqual(rc, 0)
        self.assertIn('"dispatch": "skip-no-seed"', out)
        self.assertNotIn("dispatch_probe", out)
        self.assertNotIn("探活异常", out)


class MainDegradeContractTest(unittest.TestCase):
    """通知失败降级契约（评审 G3）：真 main() 全程零网络，kimi（sidecar 形）
    种子腿全绿、dispatch 失败、探活失联同时在场，断言退出码 0、fails 空、
    dispatch 态带缘由、dispatch_probe 在位"""

    ASSET = b"fake-kimi-asset-bytes"

    def test_dispatch_failure_degrades_not_red(self):
        sha = __import__("hashlib").sha256(self.ASSET).hexdigest()
        env = {k: v for k, v in os.environ.items()
               if k != "AGENT_UPSTREAM_DISPATCH_TOKEN"}
        env["AGENT_UPSTREAM_DISPATCH_TOKEN"] = "t"

        def fake_get(url, timeout=30):
            # 边车 URL 出官方清单形内容（hex 空两格名，同 seed.py 契约）
            if url.endswith(".sha256"):
                return 200, f"{sha}  {url.rsplit('/', 1)[-1][:-7]}\n".encode()
            return 200, self.ASSET

        def fake_gh(argv=None, **kw):
            argv = argv or kw.get("args")
            if "releases/latest" in " ".join(argv):
                return mock.Mock(returncode=0, stdout="@moonshot-ai/kimi-code@2.1.1\n",
                                 stderr="")
            return mock.Mock(returncode=1, stderr="gh: Not Found (HTTP 404)", stdout="")

        out = io.StringIO()
        with mock.patch.dict(os.environ, env, clear=True), \
                mock.patch.object(sys, "argv",
                                  ["agent-upstream.py", "--tier", "agent",
                                   "--tools", "kimi"]), \
                mock.patch.object(AU, "http_head", return_value=404), \
                mock.patch.object(AU, "http_get", side_effect=fake_get), \
                mock.patch.object(AU, "upload_pair", return_value=True), \
                mock.patch.object(AU.subprocess, "run", side_effect=fake_gh), \
                mock.patch.object(AU, "probe_dispatch_target",
                                  return_value="archived"), \
                mock.patch.object(sys, "stdout", out):
            rc = AU.main()
        summary = json.loads(out.getvalue().strip().splitlines()[-1])
        self.assertEqual(rc, 0)  # 降级契约：种子成功 + 通知失联 = 不红
        self.assertEqual(summary["seeded"], 1)
        self.assertEqual(summary["failed"], 0)
        self.assertEqual(summary["fails"], [])
        self.assertIn("HTTP 404", summary["dispatch"])
        self.assertEqual(summary["dispatch_probe"], "archived")
        self.assertIn("通知面失联不红", out.getvalue())


class DispatchRepoFallbackTest(unittest.TestCase):
    def test_empty_env_falls_back_to_default(self):
        mod = _load_with_env("")
        self.assertEqual(mod.DISPATCH_REPO, "raystyle/ai-cloud")

    def test_override_env_respected(self):
        mod = _load_with_env("raystyle/ledger-rs")
        self.assertEqual(mod.DISPATCH_REPO, "raystyle/ledger-rs")


if __name__ == "__main__":
    unittest.main()
