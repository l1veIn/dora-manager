#!/usr/bin/env python3

import argparse
import json
import os
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    tomllib = None


SKIP_DIRS = {
    ".git",
    ".hg",
    ".svn",
    ".venv",
    "venv",
    "__pycache__",
    "node_modules",
    "target",
    "dist",
    "build",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Migrate node dm.json files to the new schema.")
    parser.add_argument(
        "roots",
        nargs="*",
        default=[str(Path.home() / ".dm" / "nodes")],
        help="Node roots to scan. Defaults to ~/.dm/nodes",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print the files that would change without writing them",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    changed = 0

    for root_arg in args.roots:
        root = Path(root_arg).expanduser()
        if not root.exists():
            print(f"skip missing root: {root}")
            continue

        for node_dir in sorted(path for path in root.iterdir() if path.is_dir()):
            dm_path = node_dir / "dm.json"
            if not dm_path.exists():
                continue

            migrated = migrate_node(node_dir, dm_path)
            if migrated is None:
                continue

            changed += 1
            if args.dry_run:
                print(f"would update {dm_path}")
                continue

            dm_path.write_text(
                json.dumps(migrated, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            print(f"updated {dm_path}")

    if args.dry_run:
        print(f"dry-run complete: {changed} file(s) would change")
    else:
        print(f"migration complete: {changed} file(s) updated")
    return 0


def migrate_node(node_dir: Path, dm_path: Path) -> dict | None:
    raw = json.loads(dm_path.read_text(encoding="utf-8"))
    pyproject = load_toml(node_dir / "pyproject.toml")
    cargo = load_toml(node_dir / "Cargo.toml")

    node_id = raw.get("id") or node_dir.name
    category = raw.get("category") or raw.get("display", {}).get("category", "")
    avatar = raw.get("avatar")
    source = raw.get("source") or {}
    repo_url = raw.get("repository", {}).get("url") or source.get("github")
    maintainers = normalize_maintainers(raw.get("maintainers"), raw.get("author"))
    ports = normalize_ports(raw, pyproject)
    files = infer_files(node_dir, node_id, raw)

    migrated = {
        "id": node_id,
        "name": raw.get("name") or node_id,
        "version": raw.get("version") or "",
        "installed_at": raw.get("installed_at") or "",
        "source": {
            "build": source.get("build", ""),
            "github": source.get("github"),
        },
        "description": raw.get("description") or "",
        "executable": raw.get("executable") or "",
        "repository": (
            {
                "url": repo_url,
                "default_branch": raw.get("repository", {}).get("default_branch"),
                "reference": raw.get("repository", {}).get("reference"),
                "subdir": raw.get("repository", {}).get("subdir"),
            }
            if repo_url
            else None
        ),
        "maintainers": maintainers,
        "license": (
            raw.get("license")
            or extract_pyproject_license(pyproject)
            or extract_cargo_license(cargo)
        ),
        "display": {
            "category": category,
            "tags": normalize_tags(raw.get("display", {}).get("tags"), category),
            "avatar": avatar,
        },
        "capabilities": normalize_capabilities(raw.get("capabilities"), raw.get("config_schema"), ports),
        "runtime": infer_runtime(node_dir, pyproject, cargo, raw),
        "ports": ports,
        "files": files,
        "examples": normalize_examples(raw.get("examples"), files["examples"]),
        "config_schema": raw.get("config_schema"),
    }

    return strip_nones(migrated)


def load_toml(path: Path) -> dict:
    if tomllib is None or not path.exists():
        return {}
    return tomllib.loads(path.read_text(encoding="utf-8"))


def normalize_maintainers(existing, author) -> list[dict]:
    if isinstance(existing, list) and existing:
        items = []
        for item in existing:
            if isinstance(item, dict):
                items.append(
                    {
                        "name": item.get("name", ""),
                        "email": item.get("email"),
                        "url": item.get("url"),
                    }
                )
        return [strip_nones(item) for item in items if item["name"]]

    if isinstance(author, str) and author.strip():
        return [{"name": author.strip()}]
    return []


def normalize_ports(raw: dict, pyproject: dict) -> list[dict]:
    if isinstance(raw.get("ports"), list) and raw["ports"]:
        ports = []
        for item in raw["ports"]:
            if not isinstance(item, dict):
                continue
            ports.append(
                {
                    "id": item.get("id") or item.get("name") or "",
                    "name": item.get("name") or item.get("id") or "",
                    "direction": item.get("direction") or "input",
                    "data_type": item.get("data_type"),
                    "description": item.get("description") or "",
                    "required": item.get("required", True),
                    "multiple": item.get("multiple", False),
                }
            )
        return [strip_nones(item) for item in ports if item["id"]]

    ports: list[dict] = []
    for port_id in raw.get("inputs") or []:
        ports.append(
            {
                "id": port_id,
                "name": port_id,
                "direction": "input",
                "description": "",
                "required": True,
                "multiple": False,
            }
        )
    for port_id in raw.get("outputs") or []:
        ports.append(
            {
                "id": port_id,
                "name": port_id,
                "direction": "output",
                "description": "",
                "required": True,
                "multiple": False,
            }
        )

    project = pyproject.get("project") or {}
    optional = set(project.get("optional-dependencies", {}).keys())
    if optional:
        for port in ports:
            if port["id"] in optional:
                port["required"] = False

    return ports


def infer_runtime(node_dir: Path, pyproject: dict, cargo: dict, raw: dict) -> dict:
    runtime = raw.get("runtime") or {}
    language = runtime.get("language") or detect_language(node_dir)
    python = runtime.get("python")
    if python is None:
        python = ((pyproject.get("project") or {}).get("requires-python"))
    return {
        "language": language,
        "python": python,
        "platforms": runtime.get("platforms") or [],
    }


def detect_language(node_dir: Path) -> str:
    if (node_dir / "pyproject.toml").exists():
        return "python"
    if (node_dir / "Cargo.toml").exists():
        return "rust"
    if (node_dir / "package.json").exists():
        return "node"
    return ""


def infer_files(node_dir: Path, node_id: str, raw: dict) -> dict:
    existing = raw.get("files") or {}
    readme = existing.get("readme") or pick_first_existing(node_dir, ["README.md", "README.mdx", "README.txt"]) or "README.md"
    entry = existing.get("entry") or infer_entry(node_dir, node_id)
    config = existing.get("config") or pick_first_existing(
        node_dir, ["config.json", "config.toml", "config.yaml", "config.yml"]
    )
    tests = sorted(collect_named_paths(node_dir, ["test", "tests"]))
    examples = sorted(collect_named_paths(node_dir, ["example", "examples", "demo"]))
    return {
        "readme": readme,
        "entry": entry,
        "config": config,
        "tests": tests,
        "examples": examples,
    }


def infer_entry(node_dir: Path, node_id: str) -> str | None:
    module_name = node_id.replace("-", "_")
    candidates = [
        f"{module_name}/main.py",
        f"src/{module_name}/main.py",
        "main.py",
        "src/main.rs",
        "main.rs",
        "index.js",
        "src/index.ts",
    ]
    return pick_first_existing(node_dir, candidates)


def pick_first_existing(node_dir: Path, candidates: list[str]) -> str | None:
    for candidate in candidates:
        if (node_dir / candidate).exists():
            return candidate
    return None


def collect_named_paths(node_dir: Path, needles: list[str]) -> list[str]:
    results: list[str] = []
    for current_root, dir_names, file_names in os.walk(node_dir):
        dir_names[:] = [name for name in dir_names if name not in SKIP_DIRS]
        root_path = Path(current_root)

        for dir_name in dir_names:
            if matches_named_path(dir_name, needles):
                results.append((root_path / dir_name).relative_to(node_dir).as_posix())

        for file_name in file_names:
            if not matches_named_path(file_name, needles):
                continue
            results.append((root_path / file_name).relative_to(node_dir).as_posix())
    return results


def matches_named_path(name: str, needles: list[str]) -> bool:
    lower = name.lower()
    stem = Path(lower).stem
    for needle in needles:
        if stem == needle:
            return True
        for separator in ("-", "_", "."):
            if stem.startswith(f"{needle}{separator}"):
                return True
    return False


def normalize_examples(existing, example_paths: list[str]) -> list[dict]:
    if isinstance(existing, list) and existing:
        items = []
        for item in existing:
            if isinstance(item, dict) and item.get("path"):
                items.append(
                    {
                        "title": item.get("title") or pretty_title(item["path"]),
                        "path": item["path"],
                        "description": item.get("description") or "",
                    }
                )
        if items:
            return items

    return [
        {"title": pretty_title(path), "path": path, "description": ""}
        for path in example_paths
        if Path(path).suffix in {".yml", ".yaml", ".json", ".md", ".py", ".rs"}
    ]


def normalize_tags(existing, category: str) -> list[str]:
    if isinstance(existing, list) and existing:
        return [str(item) for item in existing if str(item).strip()]
    if not category:
        return []
    return [segment.strip().lower().replace(" ", "-") for segment in category.split("/") if segment.strip()]


def normalize_capabilities(existing, config_schema, ports: list[dict]) -> list[str]:
    if isinstance(existing, list) and existing:
        return [str(item) for item in existing if str(item).strip()]

    capabilities = set()
    if config_schema:
        capabilities.add("configurable")
    for port in ports:
        port_id = port["id"].lower()
        if any(token in port_id for token in ["image", "video", "audio"]):
            capabilities.add("media")
        if "panel" in port_id:
            capabilities.add("panel")
        if "file" in port_id:
            capabilities.add("filesystem")
    return sorted(capabilities)


def extract_pyproject_license(pyproject: dict) -> str | None:
    project = pyproject.get("project") or {}
    license_value = project.get("license")
    if isinstance(license_value, str):
        return license_value
    if isinstance(license_value, dict):
        return license_value.get("text")
    return None


def extract_cargo_license(cargo: dict) -> str | None:
    package = cargo.get("package") or {}
    license_value = package.get("license")
    if isinstance(license_value, str):
        return license_value
    return None


def pretty_title(path: str) -> str:
    stem = Path(path).stem.replace("-", " ").replace("_", " ")
    stem = re.sub(r"\s+", " ", stem).strip()
    return stem.title() if stem else path


def strip_nones(value):
    if isinstance(value, dict):
        return {
            key: strip_nones(item)
            for key, item in value.items()
            if item is not None
        }
    if isinstance(value, list):
        return [strip_nones(item) for item in value]
    return value


if __name__ == "__main__":
    raise SystemExit(main())
