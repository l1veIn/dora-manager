from __future__ import annotations

import pytest

from dm import Message, MessageStream


class Response:
    def __init__(self, data, status_code=200):
        self._data = data
        self.status_code = status_code
        self.text = str(data)

    def json(self):
        return self._data

    def raise_for_status(self):
        if self.status_code >= 400:
            raise RuntimeError(f"HTTP {self.status_code}")


def test_message_requires_run_id(monkeypatch):
    monkeypatch.delenv("DM_RUN_ID", raising=False)

    with pytest.raises(RuntimeError):
        Message()


def test_message_reads_env(monkeypatch):
    monkeypatch.setenv("DM_RUN_ID", "run-env")
    monkeypatch.setenv("DM_SERVER_URL", "http://dm.local/")

    msg = Message()

    assert msg.run_id == "run-env"
    assert msg.server_url == "http://dm.local"


def test_send_posts_message_and_returns_seq(monkeypatch):
    calls = {}

    def fake_post(url, json, timeout):
        calls.update(url=url, json=json, timeout=timeout)
        return Response({"seq": 42})

    monkeypatch.setattr("dm._message.requests.post", fake_post)

    seq = Message(run_id="run-1", server_url="http://server").send(
        "text", {"content": "hello"}, from_="node-a"
    )

    assert seq == 42
    assert calls["url"] == "http://server/api/runs/run-1/messages"
    assert calls["timeout"] == 5.0
    assert calls["json"]["from"] == "node-a"
    assert calls["json"]["tag"] == "text"
    assert calls["json"]["payload"] == {"content": "hello"}
    assert isinstance(calls["json"]["timestamp"], int)


def test_send_retries_transient_server_errors(monkeypatch):
    calls = []
    responses = [Response({"error": "locked"}, status_code=500), Response({"seq": 43})]

    def fake_post(url, json, timeout):
        calls.append((url, json, timeout))
        return responses.pop(0)

    monkeypatch.setattr("dm._message.requests.post", fake_post)
    monkeypatch.setattr("dm._message.time.sleep", lambda _seconds: None)

    seq = Message(run_id="run-1", server_url="http://server").send(
        "widgets", {"widgets": {}}, from_="node-a"
    )

    assert seq == 43
    assert len(calls) == 2


def test_get_builds_query_and_returns_messages(monkeypatch):
    calls = {}

    def fake_get(url, params=None, timeout=None):
        calls.update(url=url, params=params, timeout=timeout)
        return Response({"messages": [{"seq": 2, "tag": "text"}], "next_seq": 2})

    monkeypatch.setattr("dm._message.requests.get", fake_get)

    messages = Message(run_id="run-1", server_url="http://server").get(
        tag="text", from_="node-a", after_seq=1, before_seq=5, limit=10
    )

    assert messages == [{"seq": 2, "tag": "text"}]
    assert calls["url"] == "http://server/api/runs/run-1/messages"
    assert calls["params"] == {
        "limit": 10,
        "tag": "text",
        "from": "node-a",
        "after_seq": 1,
        "before_seq": 5,
    }
    assert calls["timeout"] == 5.0


def test_snapshots_returns_list(monkeypatch):
    monkeypatch.setattr(
        "dm._message.requests.get",
        lambda url, timeout: Response([{"node_id": "node-a", "tag": "text"}]),
    )

    snapshots = Message(run_id="run-1").snapshots()

    assert snapshots == [{"node_id": "node-a", "tag": "text"}]


def test_subscribe_returns_message_stream():
    stream = Message(run_id="run-1", server_url="http://server", timeout=3).subscribe(
        tag="text", from_="node-a", timeout=7
    )

    assert isinstance(stream, MessageStream)
    assert stream.run_id == "run-1"
    assert stream.server_url == "http://server"
    assert stream.tag == "text"
    assert stream.from_ == "node-a"
    assert stream.timeout == 7


def test_message_stream_connects_to_ws_and_closes(monkeypatch):
    class FakeWebSocket:
        def __init__(self):
            self.closed = False

        def close(self):
            self.closed = True

    calls = {}
    websocket = FakeWebSocket()

    def fake_connect(url, open_timeout=None, close_timeout=None):
        calls.update(url=url, open_timeout=open_timeout, close_timeout=close_timeout)
        return websocket

    monkeypatch.setattr("dm._stream.connect", fake_connect)

    with MessageStream("run-1", "http://server", timeout=9) as stream:
        assert stream is not None

    assert calls == {
        "url": "ws://server/api/runs/run-1/messages/ws",
        "open_timeout": 9,
        "close_timeout": 9,
    }
    assert websocket.closed is True


def test_message_stream_fetches_payload_and_filters(monkeypatch):
    class FakeWebSocket:
        def __init__(self):
            self.notifications = iter(
                [
                    '{"run_id":"run-1","seq":2,"from":"node-b","tag":"text"}',
                    '{"run_id":"run-1","seq":3,"from":"node-a","tag":"text"}',
                ]
            )

        def recv(self, timeout=None):
            return next(self.notifications)

    calls = []

    def fake_get(url, params=None, timeout=None):
        calls.append({"url": url, "params": params, "timeout": timeout})
        seq = params["after_seq"] + 1
        return Response(
            {
                "messages": [
                    {
                        "seq": seq,
                        "from": "node-b" if seq == 2 else "node-a",
                        "tag": "text",
                        "payload": {"seq": seq},
                        "timestamp": 123,
                    }
                ]
            }
        )

    stream = MessageStream(
        "run-1",
        "http://server",
        tag="text",
        from_="node-a",
        timeout=4,
    )
    stream._ws = FakeWebSocket()
    monkeypatch.setattr("dm._stream.requests.get", fake_get)

    message = next(stream)

    assert message == {
        "seq": 3,
        "from": "node-a",
        "tag": "text",
        "payload": {"seq": 3},
        "timestamp": 123,
    }
    assert calls == [
        {
            "url": "http://server/api/runs/run-1/messages",
            "params": {"after_seq": 1, "limit": 1},
            "timeout": 4,
        },
        {
            "url": "http://server/api/runs/run-1/messages",
            "params": {"after_seq": 2, "limit": 1},
            "timeout": 4,
        },
    ]


def test_embed_sets_default_template_and_merges_into_send(monkeypatch):
    calls = {}

    def fake_post(url, json, timeout):
        calls.update(url=url, json=json, timeout=timeout)
        return Response({"seq": 1})

    monkeypatch.setattr("dm._message.requests.post", fake_post)

    msg = Message(run_id="run-1", server_url="http://server")
    msg.embed(author={"name": "Bot"}, color="green")
    msg.send("text", {"content": "hello"})

    embed = calls["json"]["payload"].get("embed", {})
    assert embed["author"] == {"name": "Bot"}
    assert embed["color"] == 0x22C55E


def test_embed_per_message_overrides_default(monkeypatch):
    calls = []

    def fake_post(url, json, timeout):
        calls.append(json)
        return Response({"seq": 1})

    monkeypatch.setattr("dm._message.requests.post", fake_post)

    msg = Message(run_id="run-1", server_url="http://server")
    msg.embed(author={"name": "Bot"}, color="green")

    msg.send("text", {"content": "one"})
    msg.send("text", {"content": "two"}, embed={"color": "red"})

    assert calls[0]["payload"]["embed"]["color"] == 0x22C55E
    assert calls[1]["payload"]["embed"]["color"] == "red"
    assert calls[1]["payload"]["embed"]["author"] == {"name": "Bot"}


def test_embed_chaining_returns_self():
    msg = Message(run_id="run-1", server_url="http://server")
    result = msg.embed(author={"name": "X"})
    assert result is msg
    assert msg._default_embed["author"] == {"name": "X"}


def test_embed_merge_called_twice():
    msg = Message(run_id="run-1", server_url="http://server")
    msg.embed(author={"name": "Bot"})
    msg.embed(color="blue")
    assert msg._default_embed["author"] == {"name": "Bot"}
    assert msg._default_embed["color"] == 0x3B82F6


def test_embed_send_without_default_does_not_inject(monkeypatch):
    calls = {}

    def fake_post(url, json, timeout):
        calls.update(json=json)
        return Response({"seq": 1})

    monkeypatch.setattr("dm._message.requests.post", fake_post)

    msg = Message(run_id="run-1", server_url="http://server")
    msg.send("text", {"content": "no embed"})

    assert "embed" not in calls["json"]["payload"]


def test_normalize_color_by_name():
    from dm._message import _normalize_color
    assert _normalize_color("green") == 0x22C55E
    assert _normalize_color("RED") == 0xEF4444
    assert _normalize_color("Blue") == 0x3B82F6


def test_normalize_color_by_hex_string():
    from dm._message import _normalize_color
    assert _normalize_color("#ff0000") == 0xFF0000
    assert _normalize_color("00ff00") == 0x00FF00


def test_normalize_color_by_int():
    from dm._message import _normalize_color
    assert _normalize_color(0xFFA500) == 0xFFA500


def test_normalize_color_unknown_falls_back_to_gray():
    from dm._message import _normalize_color
    assert _normalize_color("nonexistent") == 0x6B7280


def test_widgets_property_returns_widget_manager():
    msg = Message(run_id="run-1", server_url="http://server")
    w = msg.widgets
    from dm._message import WidgetManager
    assert isinstance(w, WidgetManager)
    assert w._msg is msg


def test_widgets_property_is_singleton():
    msg = Message(run_id="run-1", server_url="http://server")
    assert msg.widgets is msg.widgets


def test_widget_register_sends_widgets_tag(monkeypatch):
    calls = {}

    def fake_post(url, json, timeout):
        calls.update(json=json)
        return Response({"seq": 1})

    monkeypatch.setattr("dm._message.requests.post", fake_post)

    msg = Message(run_id="run-1", server_url="http://server")
    msg.widgets.register(key="my-slider", type="slider", label="Temp",
                          config={"min": 0, "max": 100})

    assert calls["json"]["tag"] == "widgets"
    payload = calls["json"]["payload"]
    assert payload["widget_key"] == "my-slider"
    assert payload["label"] == "Temp"
    assert payload["widgets"]["value"]["type"] == "slider"
    assert payload["widgets"]["value"]["min"] == 0
    assert payload["widgets"]["value"]["max"] == 100


def test_widget_update_sends_config(monkeypatch):
    calls = []

    def fake_post(url, json, timeout):
        calls.append(json)
        return Response({"seq": 1})

    monkeypatch.setattr("dm._message.requests.post", fake_post)

    msg = Message(run_id="run-1", server_url="http://server")
    msg.widgets.update(key="my-slider", disabled=True, label="Disabled Temp")

    payload = calls[0]["payload"]
    assert payload["widget_key"] == "my-slider"
    assert payload["widget_update"] is True
    assert payload["config"]["disabled"] is True
    assert payload["config"]["label"] == "Disabled Temp"


def test_widget_remove_calls_update_with_hidden(monkeypatch):
    calls = []

    def fake_post(url, json, timeout):
        calls.append(json)
        return Response({"seq": 1})

    monkeypatch.setattr("dm._message.requests.post", fake_post)

    msg = Message(run_id="run-1", server_url="http://server")
    msg.widgets.remove(key="my-slider")

    payload = calls[0]["payload"]
    assert payload["widget_key"] == "my-slider"
    assert payload["config"]["hidden"] is True


def test_widget_list_reads_snapshots(monkeypatch):
    snapshots = [
        {"node_id": "node-a", "tag": "widgets",
         "payload": {"label": "Slider", "widget_key": "k1",
                      "widgets": {"value": {"type": "slider"}}}},
        {"node_id": "node-b", "tag": "widgets",
         "payload": {"label": "Button", "widget_key": "k2",
                      "widgets": {"value": {"type": "button"}}}},
        {"node_id": "node-c", "tag": "text",
         "payload": {"content": "hello"}},
    ]

    monkeypatch.setattr(
        "dm._message.Message.snapshots",
        lambda self: snapshots,
    )

    msg = Message(run_id="run-1", server_url="http://server")
    widgets = msg.widgets.list()

    assert len(widgets) == 2
    assert widgets[0]["key"] == "k1"
    assert widgets[0]["type"] == "slider"
    assert widgets[1]["key"] == "k2"
    assert widgets[1]["type"] == "button"


def test_subscribe_with_widget_key_filters_by_key(monkeypatch):
    """Test that msg.subscribe(widget_key=...) filters messages on the client side."""
    from dm._stream import MessageStream

    class FakeStream:
        def __init__(self):
            self.notifications = [
                '{"run_id":"run-1","seq":1,"from":"web","tag":"input"}',
                '{"run_id":"run-1","seq":2,"from":"web","tag":"input"}',
            ]
            self._idx = 0

        def __enter__(self):
            return self

        def __exit__(self, *args):
            pass

        def __iter__(self):
            return self

        def recv(self, timeout=None):
            if self._idx >= len(self.notifications):
                raise ConnectionError("closed")
            item = self.notifications[self._idx]
            self._idx += 1
            return item

        def close(self):
            pass

    monkeypatch.setattr(
        "dm._stream.connect",
        lambda url, open_timeout=None, close_timeout=None: None,
    )
    monkeypatch.setattr(
        "dm._message.requests.get",
        lambda url, params=None, timeout=None: Response({
            "messages": [
                {"seq": 1, "from": "web", "tag": "input",
                 "payload": {"widget_key": "other", "value": "no"}, "timestamp": 0},
                {"seq": 2, "from": "web", "tag": "input",
                 "payload": {"widget_key": "my-key", "value": "yes", "output_id": "val"}, "timestamp": 0},
            ]
        }),
    )

    # Direct test: MessageStream with widget_key filter
    stream = MessageStream(
        "run-1", "http://server",
        tag="input", widget_key="my-key", timeout=4,
    )
    stream._ws = FakeStream()
    monkeypatch.setattr("dm._stream.requests.get", lambda url, params=None, timeout=None: Response({
        "messages": [{"seq": 2, "from": "web", "tag": "input",
                       "payload": {"widget_key": "my-key", "value": "yes", "output_id": "val"},
                       "timestamp": 0}]
    }))

    message = next(stream)
    assert message["payload"]["value"] == "yes"
    assert message["payload"]["output_id"] == "val"

    # Test that msg.get() with widget_key also filters client-side
    monkeypatch.setattr(
        "dm._message.requests.get",
        lambda url, params=None, timeout=None: Response({
            "messages": [
                {"seq": 1, "payload": {"widget_key": "other", "value": "no"}},
                {"seq": 2, "payload": {"widget_key": "my-key", "value": "yes"}},
            ]
        }),
    )
    msg = Message(run_id="run-1", server_url="http://server")
    results = msg.get(tag="input", widget_key="my-key")
    assert len(results) == 1
    assert results[0]["payload"]["value"] == "yes"
