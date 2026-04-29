from __future__ import annotations

import time
from typing import Any

import requests

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
        self.run_id = run_id or env_or_default("DM_RUN_ID")
        if not self.run_id:
            raise RuntimeError("run_id is required or DM_RUN_ID must be set")
        self.server_url = normalize_url(
            server_url or env_or_default("DM_SERVER_URL", "http://127.0.0.1:3210")
        )
        self.timeout = timeout

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

    def pull(
        self,
        *,
        tag: str | None = None,
        from_: str | None = None,
        after_seq: int | None = None,
        before_seq: int | None = None,
        limit: int = 100,
    ) -> list[dict[str, Any]]:
        """Fetch message history in ascending sequence order."""
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

    def subscribe(self):
        """Subscribe to future messages via WebSocket once real-time SDK support lands."""
        raise NotImplementedError("Coming soon")

    def _url(self, suffix: str) -> str:
        return f"{self.server_url}/api/runs/{self.run_id}{suffix}"
