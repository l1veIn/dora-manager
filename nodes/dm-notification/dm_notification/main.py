import json
import os
import platform
import signal
import subprocess
import sys

import pyarrow as pa
from dora import Node

SDK_PATH = os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "sdk", "python"
)
SDK_PATH = os.path.normpath(SDK_PATH)
if os.path.isdir(SDK_PATH) and SDK_PATH not in sys.path:
    sys.path.insert(0, SDK_PATH)

import dm  # noqa: E402


RUNNING = True


def env_str(name: str, default: str = "") -> str:
    raw = os.getenv(name)
    if raw is None or not raw.strip():
        return default
    return raw.strip()


def env_bool(name: str, default: bool) -> bool:
    raw = env_str(name, str(default)).lower()
    return raw in {"1", "true", "yes", "on"}


def handle_stop(_signum, _frame):
    global RUNNING
    RUNNING = False


def extract_text(value) -> str:
    if hasattr(value, "to_pylist"):
        values = value.to_pylist()
        if len(values) == 1:
            return extract_text(values[0])
        return "\n".join(extract_text(item) for item in values)
    raw = value.as_py() if hasattr(value, "as_py") else value
    if isinstance(raw, bytes):
        return raw.decode("utf-8", errors="replace")
    if raw is None:
        return ""
    return str(raw)


def notify(title: str, body: str, subtitle: str, sound: str):
    system = platform.system()
    if system == "Darwin":
        script = f'display notification {json.dumps(body)} with title {json.dumps(title)}'
        if subtitle:
            script += f' subtitle {json.dumps(subtitle)}'
        if sound:
            script += f' sound name {json.dumps(sound)}'
        subprocess.run(["osascript", "-e", script], check=True)
        return
    if system == "Linux":
        command = ["notify-send", title, body]
        if subtitle:
            command.extend(["--app-name", subtitle])
        subprocess.run(command, check=True)
        return
    if system == "Windows":
        ps_title = title.replace("'", "''")
        ps_body = body.replace("'", "''")
        script = (
            "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null;"
            "$template = [Windows.UI.Notifications.ToastTemplateType]::ToastText02;"
            "$xml = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent($template);"
            f"$xml.GetElementsByTagName('text')[0].AppendChild($xml.CreateTextNode('{ps_title}')) > $null;"
            f"$xml.GetElementsByTagName('text')[1].AppendChild($xml.CreateTextNode('{ps_body}')) > $null;"
            "$toast = [Windows.UI.Notifications.ToastNotification]::new($xml);"
            "[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Dora Manager').Show($toast);"
        )
        subprocess.run(["powershell", "-NoProfile", "-Command", script], check=True)
        return
    raise RuntimeError(f"unsupported notification platform: {system}")


def send_frontend(msg: dm.Message, node_id: str, title: str, body: str, ok: bool):
    try:
        msg.send(
            "text",
            {
                "content": body,
                "embed": {
                    "author": {"name": "dm-notification"},
                    "title": title,
                    "body": body,
                    "color": "green" if ok else "red",
                    "status": "success" if ok else "error",
                    "timestamp": "relative",
                    "width": "compact",
                },
            },
            from_=node_id,
        )
    except Exception as exc:
        print(f"[dm-notification] frontend send failed: {exc}", file=sys.stderr, flush=True)


def main():
    global RUNNING
    signal.signal(signal.SIGTERM, handle_stop)
    signal.signal(signal.SIGINT, handle_stop)

    node_id = env_str("DM_NODE_ID", "dm-notification")
    title = env_str("TITLE", "Dora Manager")
    subtitle = env_str("SUBTITLE", "")
    sound = env_str("SOUND", "")
    send_frontend_status = env_bool("SEND_FRONTEND_STATUS", True)

    msg = dm.Message()
    node = Node()

    for event in node:
        if not RUNNING:
            break
        if event["type"] != "INPUT" or event["id"] != "message":
            continue
        body = extract_text(event["value"])
        ok = True
        error = ""
        try:
            notify(title, body, subtitle, sound)
        except Exception as exc:
            ok = False
            error = str(exc)
            print(f"[dm-notification] notify failed: {error}", file=sys.stderr, flush=True)
        payload = {
            "ok": ok,
            "title": title,
            "message": body,
            "error": error,
        }
        node.send_output("status", pa.array([json.dumps(payload)]), {"content_type": "application/json"})
        if send_frontend_status:
            send_frontend(msg, node_id, title if ok else "Notification failed", body if ok else error, ok)


if __name__ == "__main__":
    main()
