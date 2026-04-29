from __future__ import annotations

import json
from typing import Any
from urllib.parse import urlsplit, urlunsplit

import requests
from websockets.sync.client import ClientConnection, connect

from ._util import normalize_url


class MessageStream:
    """
    Context manager that yields messages from a WebSocket.

    Uses the `websockets` library. Connects to
    ws://<server_url>/api/runs/<run_id>/messages/ws. On each notification
    {run_id, seq, from, tag}, fetches the full message via the REST API and
    yields it.

    Yields dicts with keys: seq, from, tag, payload, timestamp.
    """

    def __init__(
        self,
        run_id: str,
        server_url: str,
        *,
        tag: str | None = None,
        from_: str | None = None,
        timeout: float | None = None,
    ):
        self.run_id = run_id
        self.server_url = normalize_url(server_url)
        self.tag = tag
        self.from_ = from_
        self.timeout = timeout
        self._ws: ClientConnection | None = None

    def __enter__(self) -> MessageStream:
        self._ws = connect(
            self._ws_url(),
            open_timeout=self.timeout,
            close_timeout=self.timeout,
        )
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        if self._ws is not None:
            self._ws.close()
            self._ws = None

    def __iter__(self) -> MessageStream:
        return self

    def __next__(self) -> dict[str, Any]:
        if self._ws is None:
            raise RuntimeError("MessageStream must be used as a context manager")

        while True:
            notification = self._read_notification()
            message = self._fetch_message(int(notification["seq"]))
            if message is None:
                continue
            if self.tag is not None and message.get("tag") != self.tag:
                continue
            if self.from_ is not None and message.get("from") != self.from_:
                continue
            return message

    def _read_notification(self) -> dict[str, Any]:
        assert self._ws is not None
        raw = self._ws.recv(timeout=self.timeout)
        if isinstance(raw, bytes):
            raw = raw.decode("utf-8")
        notification = json.loads(raw)
        if not isinstance(notification, dict):
            raise ValueError("Message notification must be a JSON object")
        return notification

    def _fetch_message(self, seq: int) -> dict[str, Any] | None:
        response = requests.get(
            self._rest_url("/messages"),
            params={"after_seq": seq - 1, "limit": 1},
            timeout=self.timeout,
        )
        response.raise_for_status()
        data = response.json()
        messages = data.get("messages", [])
        for message in messages:
            if message.get("seq") == seq:
                return message
        return None

    def _rest_url(self, suffix: str) -> str:
        return f"{self.server_url}/api/runs/{self.run_id}{suffix}"

    def _ws_url(self) -> str:
        parts = urlsplit(self._rest_url("/messages/ws"))
        if parts.scheme == "http":
            scheme = "ws"
        elif parts.scheme == "https":
            scheme = "wss"
        else:
            scheme = parts.scheme
        return urlunsplit((scheme, parts.netloc, parts.path, parts.query, parts.fragment))
