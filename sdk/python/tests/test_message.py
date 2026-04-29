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
