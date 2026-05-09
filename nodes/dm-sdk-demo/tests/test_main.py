from __future__ import annotations

import importlib
import sys
import types


class FakeMessage:
    def __init__(self):
        self.sent = []

    def send(self, tag, payload, *, from_=None):
        self.sent.append({"tag": tag, "payload": payload, "from": from_})
        return len(self.sent)


def load_demo(monkeypatch):
    monkeypatch.setitem(sys.modules, "pyarrow", types.SimpleNamespace())
    monkeypatch.setitem(sys.modules, "dora", types.SimpleNamespace(Node=object))
    module = importlib.import_module("dm_sdk_demo.main")
    module.COUNTER = 0
    module.CHAT_HISTORY = []
    return module


def test_on_new_message_handles_stream_message_shape(monkeypatch):
    demo = load_demo(monkeypatch)
    msg = FakeMessage()

    demo.on_new_message(
        msg,
        {
            "seq": 3,
            "from": "web",
            "tag": "input",
            "payload": {
                "widget_key": "sdk-demo-input",
                "output_id": "value",
                "value": "hello world",
            },
            "timestamp": 123,
        },
    )

    assert [item["tag"] for item in msg.sent] == ["text", "text", "text"]
    assert msg.sent[0]["payload"] == {"content": "**Reversed:** dlrow olleh"}
    assert all(item["from"] == "dm-sdk-demo" for item in msg.sent)


def test_on_new_message_still_handles_flat_replay_shape(monkeypatch):
    demo = load_demo(monkeypatch)
    msg = FakeMessage()

    demo.on_new_message(msg, {"value": "abc", "seq": 1})

    assert msg.sent[0]["payload"] == {"content": "**Reversed:** cba"}
