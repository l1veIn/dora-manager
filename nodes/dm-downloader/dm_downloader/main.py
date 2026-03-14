"""dm-downloader: Model weight downloader with Panel UI lifecycle."""

import hashlib
import json
import os
import shutil
import tarfile
import zipfile
from pathlib import Path
from urllib.parse import urlparse

import pyarrow as pa
import requests
from dora import Node
from platformdirs import user_data_dir


# ---------------------------------------------------------------------------
# Config helpers
# ---------------------------------------------------------------------------

def _env_str(key: str, default: str = "") -> str:
    return os.getenv(key, default)


def _env_bool(key: str, default: bool = False) -> bool:
    val = os.getenv(key, "")
    if not val:
        return default
    return val.lower() in ("1", "true", "yes", "on")


def _default_download_dir() -> Path:
    """Persistent data directory for downloads (not cache — survives cleanup).

    Override with DM_DOWNLOAD_DIR environment variable.

    Defaults:
      macOS:   ~/Library/Application Support/dm/downloads
      Linux:   ~/.local/share/dm/downloads
      Windows: %LOCALAPPDATA%/dm/data/downloads
    """
    override = os.getenv("DM_DOWNLOAD_DIR")
    if override:
        return Path(override)
    return Path(user_data_dir("dm")) / "downloads"


def _dest_from_url(url: str) -> str:
    """Derive a destination name from the URL filename."""
    parsed = urlparse(url)
    filename = Path(parsed.path).name
    if not filename:
        filename = "download"
    return filename


# ---------------------------------------------------------------------------
# Hash verification
# ---------------------------------------------------------------------------

def _parse_hash(hash_str: str) -> tuple[str, str] | None:
    """Parse 'algorithm:hex' format. Returns (algorithm, expected_hex) or None."""
    if not hash_str or ":" not in hash_str:
        return None
    algo, expected = hash_str.split(":", 1)
    return algo.strip().lower(), expected.strip().lower()


def _verify_hash(filepath: Path, algo: str, expected: str) -> bool:
    """Verify file hash by streaming chunks."""
    h = hashlib.new(algo)
    with open(filepath, "rb") as f:
        while chunk := f.read(8 * 1024 * 1024):  # 8 MB chunks
            h.update(chunk)
    return h.hexdigest().lower() == expected


# ---------------------------------------------------------------------------
# Extraction
# ---------------------------------------------------------------------------

SUPPORTED_ARCHIVES = (".tar.gz", ".tgz", ".tar.bz2", ".tar.xz", ".zip")


def _is_extractable(filepath: Path) -> bool:
    name = filepath.name.lower()
    return any(name.endswith(ext) for ext in SUPPORTED_ARCHIVES)


def _extract(filepath: Path, dest: Path) -> bool:
    """Extract archive to dest. Returns True on success."""
    name = filepath.name.lower()
    try:
        if name.endswith(".zip"):
            with zipfile.ZipFile(filepath, "r") as zf:
                zf.extractall(dest)
        elif name.endswith((".tar.gz", ".tgz", ".tar.bz2", ".tar.xz")):
            with tarfile.open(filepath, "r:*") as tf:
                tf.extractall(dest)
        else:
            return False
        return True
    except (tarfile.TarError, zipfile.BadZipFile, OSError) as e:
        print(f"[dm-downloader] extraction failed: {e}", flush=True)
        return False


# ---------------------------------------------------------------------------
# UI helpers — emit widget overrides for Panel bind
# ---------------------------------------------------------------------------

def _send_ui(node: Node, state: dict):
    """Send widget override to the 'ui' output port."""
    node.send_output(
        "ui",
        pa.array([json.dumps(state)]),
        {"content_type": "application/json"},
    )


def _format_bytes(n: int) -> str:
    """Human-readable byte size."""
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024:
            return f"{n:.1f} {unit}" if unit != "B" else f"{n} {unit}"
        n /= 1024
    return f"{n:.1f} TB"


# ---------------------------------------------------------------------------
# Download
# ---------------------------------------------------------------------------

def _download(node: Node, url: str, tmp_path: Path) -> bool:
    """Download url to tmp_path with progress UI updates. Returns True on success."""
    try:
        resp = requests.get(url, stream=True, timeout=30)
        resp.raise_for_status()
    except requests.RequestException as e:
        print(f"[dm-downloader] download request failed: {e}", flush=True)
        return False

    total = resp.headers.get("Content-Length")
    total_bytes = int(total) if total else None
    downloaded = 0
    last_pct = -1

    tmp_path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with open(tmp_path, "wb") as f:
            for chunk in resp.iter_content(chunk_size=1024 * 1024):  # 1 MB
                if not chunk:
                    continue
                f.write(chunk)
                downloaded += len(chunk)

                # Throttle UI updates: every 2% or every 5 MB for indeterminate
                if total_bytes:
                    pct = int(downloaded / total_bytes * 100)
                    if pct != last_pct and pct % 2 == 0:
                        last_pct = pct
                        _send_ui(node, {
                            "loading": True,
                            "progress": downloaded / total_bytes,
                            "label": f"Downloading {pct}%",
                            "disabled": True,
                        })
                else:
                    if downloaded % (5 * 1024 * 1024) < (1024 * 1024):
                        _send_ui(node, {
                            "loading": True,
                            "progress": -1,
                            "label": f"Downloading {_format_bytes(downloaded)}",
                            "disabled": True,
                        })
        return True
    except (OSError, requests.RequestException) as e:
        print(f"[dm-downloader] download write failed: {e}", flush=True)
        return False


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    """Entry point for dm-downloader node."""
    node = Node()

    url = _env_str("URL")
    hash_str = _env_str("HASH")
    extract = _env_bool("EXTRACT")
    dest_raw = _env_str("DEST")

    if not url:
        print("[dm-downloader] ERROR: URL config is required", flush=True)
        _send_ui(node, {
            "loading": False,
            "label": "Error: no URL configured",
            "disabled": True,
            "variant": "destructive",
        })
        return

    # Resolve destination path
    base_dir = _default_download_dir()
    if dest_raw:
        dest = Path(dest_raw) if Path(dest_raw).is_absolute() else base_dir / dest_raw
    else:
        dest = base_dir / _dest_from_url(url)

    hash_info = _parse_hash(hash_str)
    tmp_path = dest.parent / f"{dest.name}.dm-tmp"

    print(f"[dm-downloader] url={url}", flush=True)
    print(f"[dm-downloader] dest={dest}", flush=True)
    print(f"[dm-downloader] hash={'yes' if hash_info else 'none'}", flush=True)
    print(f"[dm-downloader] extract={extract}", flush=True)

    # ---- State: Checking ----
    _send_ui(node, {"loading": True, "label": "Checking...", "disabled": True})

    need_download = True
    if dest.exists():
        if hash_info:
            algo, expected = hash_info
            print(f"[dm-downloader] verifying existing file hash ({algo})...", flush=True)
            if _verify_hash(dest, algo, expected):
                print("[dm-downloader] hash match — already ready", flush=True)
                need_download = False
            else:
                print("[dm-downloader] hash mismatch — need re-download", flush=True)
        else:
            # No hash configured, file exists → treat as ready
            print("[dm-downloader] file exists (no hash check) — ready", flush=True)
            need_download = False

    if not need_download:
        # ---- State: Ready (already exists) ----
        _send_ui(node, {
            "loading": False,
            "label": "Ready ✓",
            "disabled": True,
            "variant": "secondary",
        })
        node.send_output(
            "path",
            pa.array([str(dest)]),
            {"content_type": "text/plain"},
        )
        # Keep alive for tick events
        _wait_loop(node)
        return

    # ---- State: Waiting ----
    # Determine label
    if dest.exists():
        wait_label = "Re-download (hash mismatch)"
    else:
        # Try to show file size from HEAD request
        size_str = ""
        try:
            head = requests.head(url, timeout=10, allow_redirects=True)
            cl = head.headers.get("Content-Length")
            if cl:
                size_str = f" ({_format_bytes(int(cl))})"
        except requests.RequestException:
            pass
        wait_label = f"Download{size_str}"

    _send_ui(node, {"loading": False, "label": wait_label, "disabled": False})
    print(f"[dm-downloader] waiting for download trigger...", flush=True)

    # Event loop: wait for download trigger
    triggered = False
    finished = False
    while not finished:
        event = node.next(timeout=0.1)
        if event is None:
            finished = True
            break
        if event["type"] == "INPUT":
            if event["id"] == "download":
                triggered = True
                break
            # Ignore tick and other inputs while waiting

    if not triggered:
        return

    # ---- State: Downloading ----
    print(f"[dm-downloader] starting download: {url}", flush=True)
    _send_ui(node, {"loading": True, "progress": 0.0, "label": "Downloading 0%", "disabled": True})

    # Clean up stale tmp file
    if tmp_path.exists():
        tmp_path.unlink()

    dest.parent.mkdir(parents=True, exist_ok=True)

    ok = _download(node, url, tmp_path)
    if not ok:
        _send_ui(node, {
            "loading": False,
            "label": "Retry Download",
            "disabled": False,
            "variant": "destructive",
        })
        # Allow retry
        _retry_loop(node, url, hash_info, extract, dest, tmp_path)
        return

    # ---- State: Verifying ----
    if hash_info:
        algo, expected = hash_info
        _send_ui(node, {"loading": True, "label": "Verifying hash...", "disabled": True})
        print(f"[dm-downloader] verifying hash ({algo})...", flush=True)

        if not _verify_hash(tmp_path, algo, expected):
            print("[dm-downloader] hash verification FAILED", flush=True)
            tmp_path.unlink(missing_ok=True)
            _send_ui(node, {
                "loading": False,
                "label": "Hash mismatch — Retry",
                "disabled": False,
                "variant": "destructive",
            })
            _retry_loop(node, url, hash_info, extract, dest, tmp_path)
            return

        print("[dm-downloader] hash verified ✓", flush=True)
    else:
        print("[dm-downloader] no hash configured, skipping verification", flush=True)

    # ---- State: Extracting (optional) ----
    if extract:
        if not _is_extractable(tmp_path):
            print(f"[dm-downloader] unsupported archive format: {tmp_path.name}", flush=True)
            tmp_path.unlink(missing_ok=True)
            _send_ui(node, {
                "loading": False,
                "label": "Unsupported archive format",
                "disabled": True,
                "variant": "destructive",
            })
            _wait_loop(node)
            return

        _send_ui(node, {"loading": True, "label": "Extracting...", "disabled": True})
        print("[dm-downloader] extracting...", flush=True)

        # Extract to dest directory
        dest.mkdir(parents=True, exist_ok=True)
        if not _extract(tmp_path, dest):
            _send_ui(node, {
                "loading": False,
                "label": "Extraction failed — Retry",
                "disabled": False,
                "variant": "destructive",
            })
            _retry_loop(node, url, hash_info, extract, dest, tmp_path)
            return

        # Clean up archive after extraction
        tmp_path.unlink(missing_ok=True)
        print("[dm-downloader] extraction complete ✓", flush=True)
    else:
        # Atomic rename: tmp → dest
        if dest.exists():
            dest.unlink()
        shutil.move(str(tmp_path), str(dest))

    # ---- State: Ready ----
    _send_ui(node, {
        "loading": False,
        "label": "Ready ✓",
        "disabled": True,
        "variant": "secondary",
    })
    node.send_output(
        "path",
        pa.array([str(dest)]),
        {"content_type": "text/plain"},
    )
    print(f"[dm-downloader] READY: {dest}", flush=True)

    _wait_loop(node)


def _retry_loop(node: Node, url: str, hash_info, extract: bool, dest: Path, tmp_path: Path):
    """Wait for retry trigger, then re-run download → verify → extract → ready."""
    while True:
        event = node.next(timeout=0.1)
        if event is None:
            return
        if event["type"] == "INPUT" and event["id"] == "download":
            # Re-run the download sequence (recursive-ish via a fresh call)
            _send_ui(node, {
                "loading": True, "progress": 0.0,
                "label": "Downloading 0%", "disabled": True,
            })
            if tmp_path.exists():
                tmp_path.unlink()
            ok = _download(node, url, tmp_path)
            if not ok:
                _send_ui(node, {
                    "loading": False,
                    "label": "Retry Download",
                    "disabled": False,
                    "variant": "destructive",
                })
                continue  # Wait for another retry

            # Verify
            if hash_info:
                algo, expected = hash_info
                _send_ui(node, {"loading": True, "label": "Verifying hash...", "disabled": True})
                if not _verify_hash(tmp_path, algo, expected):
                    tmp_path.unlink(missing_ok=True)
                    _send_ui(node, {
                        "loading": False,
                        "label": "Hash mismatch — Retry",
                        "disabled": False,
                        "variant": "destructive",
                    })
                    continue

            # Extract
            if extract:
                if not _is_extractable(tmp_path):
                    tmp_path.unlink(missing_ok=True)
                    _send_ui(node, {
                        "loading": False,
                        "label": "Unsupported archive format",
                        "disabled": True,
                        "variant": "destructive",
                    })
                    _wait_loop(node)
                    return

                _send_ui(node, {"loading": True, "label": "Extracting...", "disabled": True})
                dest.mkdir(parents=True, exist_ok=True)
                if not _extract(tmp_path, dest):
                    _send_ui(node, {
                        "loading": False,
                        "label": "Extraction failed — Retry",
                        "disabled": False,
                        "variant": "destructive",
                    })
                    continue
                tmp_path.unlink(missing_ok=True)
            else:
                if dest.exists():
                    dest.unlink()
                shutil.move(str(tmp_path), str(dest))

            # Ready
            _send_ui(node, {
                "loading": False,
                "label": "Ready ✓",
                "disabled": True,
                "variant": "secondary",
            })
            node.send_output(
                "path",
                pa.array([str(dest)]),
                {"content_type": "text/plain"},
            )
            print(f"[dm-downloader] READY: {dest}", flush=True)
            _wait_loop(node)
            return


def _wait_loop(node: Node):
    """Keep node alive, consuming tick events until shutdown."""
    while True:
        event = node.next(timeout=0.1)
        if event is None:
            return
