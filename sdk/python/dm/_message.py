from __future__ import annotations

import time
from typing import Any
from urllib.parse import urlparse

import requests

from ._stream import MessageStream
from ._util import detect_caller_id, env_or_default, normalize_url


class WidgetManager:
    """Manage interaction widgets for a dora-manager run."""

    def __init__(self, msg: Message):
        self._msg = msg
        self._LAST_SEQ = 0

    def register(
        self,
        key: str,
        type: str,
        label: str,
        *,
        config: dict[str, Any] | None = None,
    ) -> "WidgetManager":
        """Register a new widget or update an existing one.

        Args:
            key: Unique widget key (used for input routing and subscription)
            type: Widget type (\"input\", \"textarea\", \"button\", \"slider\",
                  \"switch\", \"select\", \"radio\", \"checkbox\")
            label: Display label shown in the UI
            config: Additional widget configuration (min, max, step,
                   placeholder, options, default, disabled, hidden, etc.)
        """
        payload: dict[str, Any] = {
            "label": label,
            "widget_key": key,
            "widgets": {
                "value": {
                    "type": type,
                    "label": label,
                    **(config or {}),
                }
            },
        }
        self._msg.send("widgets", payload)
        return self

    def update(self, key: str, **config: Any) -> "WidgetManager":
        """Update an existing widget's configuration.

        Common config fields:
        - disabled: bool — grey out the control
        - hidden: bool — remove from UI
        - label: str — change display name
        - color: str — card accent color
        - placeholder, min, max, step, options, etc.
        """
        payload: dict[str, Any] = {
            "widget_key": key,
            "widget_update": True,
            "config": config,
        }
        self._msg.send("widgets", payload)
        return self

    def remove(self, key: str) -> "WidgetManager":
        """Remove a widget from the UI."""
        return self.update(key, hidden=True)

    def list(self) -> list[dict[str, Any]]:
        """Return all registered widgets from snapshots."""
        snapshots = self._msg.snapshots()
        return [
            {
                "key": s["payload"].get("widget_key", s["node_id"]),
                "node_id": s["node_id"],
                "label": s["payload"].get("label", s["node_id"]),
                "type": _infer_widget_type(s["payload"]),
                "disabled": s["payload"].get("widgets", {}).get("value", {}).get("disabled", False),
                "hidden": s["payload"].get("widget_update") is True
                         and s["payload"].get("config", {}).get("hidden", False),
            }
            for s in snapshots
            if s["tag"] == "widgets"
        ]


def _infer_widget_type(payload: dict[str, Any]) -> str | None:
    widgets_val = payload.get("widgets", {})
    if not widgets_val:
        return None
    first = next(iter(widgets_val.values()), {})
    if isinstance(first, dict):
        return first.get("type")
    return None


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
        self._default_embed: dict[str, Any] | None = None
        if explicit_server_url and _is_local_server_url(self.server_url):
            self._check_server_reachable()
        self.run_id = run_id or env_or_default("DM_RUN_ID")
        if not self.run_id:
            raise RuntimeError("run_id is required or DM_RUN_ID must be set")
        self._widget_manager: WidgetManager | None = None

    @property
    def widgets(self) -> WidgetManager:
        """Access widget management API for this run.

        Usage::

            msg.widgets.register(key="my-input", type="input", label="My Input")
            for event in msg.widgets.subscribe("my-input"):
                print(event["value"])
        """
        if self._widget_manager is None:
            self._widget_manager = WidgetManager(self)
        return self._widget_manager

    def embed(
        self,
        *,
        author: dict | None = None,
        title: str | None = None,
        title_url: str | None = None,
        body: str | None = None,
        body_formatted: str | None = None,
        body_format: str = "plain",
        color: str | int | None = None,
        fields: list[dict] | None = None,
        media: dict | None = None,
        thumbnail: dict | None = None,
        footer: dict | None = None,
        side: str | None = None,
        width: str | None = None,
        actions: list[dict] | None = None,
        status: str | None = None,
        progress: float | None = None,
        timestamp_display: str | None = None,
    ) -> Message:
        """Set default embed template for all subsequent sends.

        All fields are optional. Calling embed() multiple times merges
        the new fields into the existing template (shallow merge).

        Returns self for chaining.

        Example::

            msg = Message().embed(author={"name": "Bot"}, color="green")
            msg.send("text", {"content": "hello"})
            # → payload automatically includes embed
        """
        new_embed: dict[str, Any] = {}
        if author is not None:
            new_embed["author"] = author
        if title is not None:
            new_embed["title"] = title
        if title_url is not None:
            new_embed["title_url"] = title_url
        if body is not None:
            new_embed["body"] = body
        if body_formatted is not None:
            new_embed["body_formatted"] = body_formatted
        if body_format != "plain":
            new_embed["body_format"] = body_format
        if color is not None:
            new_embed["color"] = _normalize_color(color)
        if fields is not None:
            new_embed["fields"] = fields
        if media is not None:
            new_embed["media"] = media
        if thumbnail is not None:
            new_embed["thumbnail"] = thumbnail
        if footer is not None:
            new_embed["footer"] = footer
        if side is not None:
            new_embed["side"] = side
        if width is not None:
            new_embed["width"] = width
        if actions is not None:
            new_embed["actions"] = actions
        if status is not None:
            new_embed["status"] = status
        if progress is not None:
            new_embed["progress"] = progress
        if timestamp_display is not None:
            new_embed["timestamp"] = timestamp_display
        if self._default_embed is None:
            self._default_embed = {}
        self._default_embed.update(new_embed)
        return self

    def send(
        self,
        tag: str,
        payload: dict[str, Any],
        *,
        from_: str | None = None,
        embed: dict[str, Any] | None = None,
    ) -> int:
        """Persist a message and return its sequence number.

        If embed is provided, it is merged into payload["embed"].
        If embed is not provided but a default template was set via
        embed(), the default template is automatically applied.

        Args:
            tag: Message tag (e.g. "text", "image", "json")
            payload: Message content as a JSON-serializable dict
            from_: Override sender ID (auto-detected by default)
            embed: Optional render description object. Takes priority
                   over the default template set by embed().
        """
        body = {
            "from": from_ or detect_caller_id(),
            "tag": tag,
            "payload": dict(payload),
            "timestamp": int(time.time() * 1000),
        }
        if embed is not None:
            merged = dict(self._default_embed or {})
            merged.update(embed)
            body["payload"]["embed"] = merged
        elif self._default_embed is not None:
            body["payload"]["embed"] = dict(self._default_embed)
        response = self._post_message(body)
        response.raise_for_status()
        data = response.json()
        return data["seq"]

    def _post_message(self, body: dict[str, Any]) -> requests.Response:
        last_response: requests.Response | None = None
        for attempt in range(3):
            response = requests.post(
                self._url("/messages"),
                json=body,
                timeout=self.timeout,
            )
            if response.status_code < 500:
                return response
            last_response = response
            if attempt < 2:
                time.sleep(0.2 * (attempt + 1))
        return last_response

    def get(
        self,
        *,
        tag: str | None = None,
        from_: str | None = None,
        widget_key: str | None = None,
        after_seq: int | None = None,
        before_seq: int | None = None,
        limit: int = 100,
    ) -> list[dict[str, Any]]:
        """Get message history in ascending sequence order.

        Args:
            tag: Filter by tag (e.g. "input", "text")
            from_: Filter by sender
            widget_key: Filter by widget key (filters payload.widget_key)
            after_seq: Only messages after this sequence number
            before_seq: Only messages before this sequence number
            limit: Max messages to return (default 100)
        """
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
        messages = data["messages"]

        if widget_key is not None:
            messages = [
                m for m in messages
                if m.get("payload", {}).get("widget_key") == widget_key
            ]

        return messages

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
        widget_key: str | None = None,
        timeout: float | None = None,
    ) -> MessageStream:
        """
        Subscribe to real-time messages via WebSocket.

        Returns a MessageStream context manager that yields messages as they arrive.
        When widget_key is set, only messages matching that key are yielded.

        Usage::

            with msg.subscribe(tag="input", widget_key="my-slider") as stream:
                for event in stream:
                    print(event)  # only events with that widget_key
        """
        return MessageStream(
            self.run_id,
            self.server_url,
            tag=tag,
            from_=from_,
            widget_key=widget_key,
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


_SEMANTIC_COLORS: dict[str, int] = {
    "green": 0x22c55e,
    "red": 0xef4444,
    "yellow": 0xeab308,
    "blue": 0x3b82f6,
    "purple": 0xa855f7,
    "gray": 0x6b7280,
    "orange": 0xf97316,
}


def _normalize_color(color: str | int) -> int:
    """Convert a color value to a normalized integer RGB.

    Accepts:
    - Semantic names: \"green\", \"red\", \"yellow\", \"blue\", \"purple\", \"gray\", \"orange\"
    - Hex integer: 0x00ff00
    """
    if isinstance(color, int):
        return color
    lowered = color.lower().strip()
    if lowered in _SEMANTIC_COLORS:
        return _SEMANTIC_COLORS[lowered]
    if lowered.startswith("#"):
        try:
            return int(lowered[1:], 16)
        except ValueError:
            return _SEMANTIC_COLORS.get("gray", 0x6b7280)
    try:
        return int(lowered, 16)
    except ValueError:
        return _SEMANTIC_COLORS.get("gray", 0x6b7280)
