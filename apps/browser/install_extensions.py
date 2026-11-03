#!/usr/bin/env python3
"""Install Chromium Web Store extensions into the Babel Browser profile.

The IDs are fixed. Downloads use Google's Chrome Web Store update service.
No alternate mirrors or executable post-install hooks are used.
"""
import argparse
import io
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import stat
import struct
import tempfile
from urllib.parse import urlencode
from urllib.request import Request, urlopen
import zipfile

EXTENSIONS = {
    "ublock-origin-lite": "ddkjiahejlhfcafbddmgiahcphecmpfh",
    "vimium": "dbepggeogbaibhgnhhndojpepiihcmeb",
    "dark-reader": "eimadpbcbfnmbkopoojfekhnkhdbieeh",
}
MAX_CRX = 60 * 1024 * 1024
MAX_EXTRACTED = 180 * 1024 * 1024


def crx_zip(payload: bytes) -> bytes:
    if payload[:4] != b"Cr24" or len(payload) < 12:
        raise ValueError("expected a CRX package")
    version, header_size = struct.unpack_from("<II", payload, 4)
    if version == 3:
        start = 12 + header_size
    elif version == 2:
        if len(payload) < 16:
            raise ValueError("truncated CRX2")
        signature_size = struct.unpack_from("<I", payload, 12)[0]
        start = 16 + header_size + signature_size
    else:
        raise ValueError(f"unsupported CRX version: {version}")
    if start >= len(payload) or payload[start:start + 2] != b"PK":
        raise ValueError("invalid CRX ZIP payload")
    return payload[start:]


def extract_package(payload: bytes, destination: Path) -> dict:
    archive = zipfile.ZipFile(io.BytesIO(crx_zip(payload)))
    members = archive.infolist()
    if len(members) > 12000 or sum(info.file_size for info in members) > MAX_EXTRACTED:
        raise ValueError("extension archive exceeds extraction limits")
    for info in members:
        parts = PurePosixPath(info.filename).parts
        mode = (info.external_attr >> 16) & 0o170000
        if (not parts or info.filename.startswith("/") or "\\" in info.filename
                or ".." in parts or mode == stat.S_IFLNK):
            raise ValueError(f"unsafe extension archive entry: {info.filename!r}")
        target = destination.joinpath(*parts)
        if info.is_dir():
            target.mkdir(parents=True, exist_ok=True)
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        with archive.open(info) as source, target.open("xb") as output:
            shutil.copyfileobj(source, output)
    manifest = destination / "manifest.json"
    if not manifest.is_file():
        raise ValueError("extension missing manifest.json")
    metadata = json.loads(manifest.read_text(encoding="utf-8"))
    if metadata.get("manifest_version") not in (2, 3):
        raise ValueError("invalid extension manifest version")
    return metadata


def download(extension_id: str) -> bytes:
    url = "https://clients2.google.com/service/update2/crx?" + urlencode({
        "response": "redirect",
        "prodversion": "131.0.0.0",
        "acceptformat": "crx3",
        "x": f"id={extension_id}&installsource=ondemand&uc",
    })
    request = Request(url, headers={"User-Agent": "Mozilla/5.0 Chromium/131.0.0.0"})
    with urlopen(request, timeout=45) as response:
        payload = response.read(MAX_CRX + 1)
    if len(payload) > MAX_CRX:
        raise ValueError("extension download exceeds maximum size")
    return payload


def install(root: Path, force: bool) -> None:
    root.mkdir(parents=True, exist_ok=True)
    for name, extension_id in EXTENSIONS.items():
        target = root / name
        if (target / "manifest.json").is_file() and not force:
            print(f"{name}: already installed")
            continue
        print(f"{name}: downloading {extension_id}", flush=True)
        payload = download(extension_id)
        with tempfile.TemporaryDirectory(dir=root, prefix=".extension-") as temp:
            staging = Path(temp) / "extension"
            staging.mkdir()
            metadata = extract_package(payload, staging)
            if target.exists():
                shutil.rmtree(target)
            shutil.move(str(staging), str(target))
            print(f"{name}: installed ({metadata.get('version', 'unknown')})")
    print("Extensions are ready to load at the next browser launch.")


def self_test() -> None:
    payload = io.BytesIO()
    with zipfile.ZipFile(payload, "w") as archive:
        archive.writestr("manifest.json", '{"manifest_version":3,"name":"Test","version":"1"}')
        archive.writestr("file.txt", "works")
    crx = b"Cr24" + struct.pack("<II", 3, 0) + payload.getvalue()
    with tempfile.TemporaryDirectory() as temp:
        metadata = extract_package(crx, Path(temp))
        assert metadata["name"] == "Test"
        assert (Path(temp) / "file.txt").read_text() == "works"
    malicious = io.BytesIO()
    with zipfile.ZipFile(malicious, "w") as archive:
        archive.writestr("../escape", "bad")
        archive.writestr("manifest.json", '{"manifest_version":3}')
    with tempfile.TemporaryDirectory() as temp:
        try:
            extract_package(b"Cr24" + struct.pack("<II", 3, 0) + malicious.getvalue(), Path(temp))
        except ValueError:
            pass
        else:
            raise AssertionError("path traversal accepted")
    print("offline extension installer tests passed")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--directory", type=Path)
    args = parser.parse_args()
    if args.self_test:
        self_test()
    else:
        base = Path(os.environ.get("BABEL_BROWSER_DIR", str(Path.home() / ".local/share/babel-browser")))
        install(args.directory or base / "extensions", args.force)
