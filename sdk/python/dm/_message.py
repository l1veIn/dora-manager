from __future__ import annotations

import time
from typing import Any
from urllib.parse import urlparse

import requests

from ._stream import MessageStream
from ._util import detect_caller_id, env_or_default, normalize_url


class Message:
    """Message operations for a dora-manager run."""

    def __init__(
        self,
        run_id: str | None = None,
        server_url: str | None = None,
        *,
        timeout: float = 5.0,
    ):
        explicit_server_url = server_url or env_or_default("DM_SERVER_URL")
        self.server_url = normalize_url(
            explicit_server_url or "http://127.0.0.1:3210"
        )
        self.timeout = timeout
        if explicit_server_url and _is_local_server_url(self.server_url):
            self._check_server_reachable()
        self.run_id = run_id or env_or_default("DM_RUN_ID")
        if not self.run_id:
            raise RuntimeError("run_id is required or DM_RUN_ID must be set")

    def send(self, tag: str, payload: dict[str, Any], *, from_: str | None = None) -> int:
        """Persist a message and return its sequence number."""
        body = {
            "from": from_ or detect_caller_id(),
            "tag": tag,
            "payload": payload,
            "timestamp": int(time.time() * 1000),
        }
        response = requests.post(
            self._url("/messages"),
            json=body,
            timeout=self.timeout,
        )
        response.raise_for_status()
        data = response.json()
        return data["seq"]

    def get(
        self,
        *,
        tag: str | None = None,
        from_: str | None = None,
        after_seq: int | None = None,
        before_seq: int | None = None,
        limit: int = 100,
    ) -> list[dict[str, Any]]:
        """Get message history in ascending sequence order."""
        params: dict[str, Any] = {"limit": limit}
        if tag is not None:
            params["tag"] = tag
        if from_ is not None:
            params["from"] = from_
        if after_seq is not None:
            params["after_seq"] = after_seq
        if before_seq is not None:
            params["before_seq"] = before_seq

        response = requests.get(
            self._url("/messages"),
            params=params,
            timeout=self.timeout,
        )
        response.raise_for_status()
        data = response.json()
        return data["messages"]

    def snapshots(self) -> list[dict[str, Any]]:
        """Return latest message snapshots grouped by node_id and tag."""
        response = requests.get(
            self._url("/messages/snapshots"),
            timeout=self.timeout,
        )
        response.raise_for_status()
        return response.json()

    def subscribe(
        self,
        *,
        tag: str | None = None,
        from_: str | None = None,
        timeout: float | None = None,
    ) -> MessageStream:
        """
        Subscribe to real-time messages via WebSocket.

        Returns a MessageStream context manager that yields messages as they arrive.

        Usage:
            with msg.subscribe() as stream:
                for event in stream:
                    print(event)  # {"seq": ..., "from": ..., "tag": ..., "payload": ...}

        The WebSocket connects to /api/runs/{run_id}/messages/ws on the dm-server.
        The server pushes MessageNotification events: {run_id, seq, from, tag}.
        After receiving a notification, the SDK fetches the full message payload via
        GET /api/runs/{run_id}/messages?after_seq={seq - 1}&limit=1.
        """
        return MessageStream(
            self.run_id,
            self.server_url,
            tag=tag,
            from_=from_,
            timeout=self.timeout if timeout is None else timeout,
        )

    def _url(self, suffix: str) -> str:
        return f"{self.server_url}/api/runs/{self.run_id}{suffix}"

    def _check_server_reachable(self) -> None:
        try:
            requests.get(f"{self.server_url}/api/doctor", timeout=self.timeout)
        except requests.RequestException:
            message = (
                f"❌ dm-server not reachable at {self.server_url}\n"
                "   This node requires dm-server for message and function services.\n"
                "   Start dm-server first, or use `dm run` which manages it automatically."
            )
            raise RuntimeError(message) from None


def _is_local_server_url(url: str) -> bool:
    hostname = urlparse(url).hostname
    return hostname in {"127.0.0.1", "localhost", "::1"}
