#!/usr/bin/env python3
"""Build the npm package from the six checksummed GitHub release archives."""

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {
    ("linux", "x64"): "x86_64-unknown-linux-musl",
    ("linux", "arm64"): "aarch64-unknown-linux-musl",
    ("darwin", "x64"): "x86_64-apple-darwin",
    ("darwin", "arm64"): "aarch64-apple-darwin",
    ("win32", "x64"): "x86_64-pc-windows-msvc",
    ("win32", "arm64"): "aarch64-pc-windows-msvc",
}


def copy_binaries(assets, package, platform, arch, target):
    windows = platform == "win32"
    stem = f"hive-{target}"
    archive = assets / (f"{stem}.zip" if windows else f"{stem}.tar.gz")
    checksum = archive.with_name(archive.name + ".sha256").read_text().split()[0]
    with archive.open("rb") as file:
        if hashlib.file_digest(file, "sha256").hexdigest() != checksum:
            raise ValueError(f"Checksum mismatch for {archive.name}")

    directory = package / "bin" / platform / arch
    directory.mkdir(parents=True)
    open_archive = zipfile.ZipFile if windows else tarfile.open
    with open_archive(archive) as compressed:
        for executable in ("hive", "hived"):
            filename = f"{executable}.exe" if windows else executable
            member = f"{stem}/{filename}"
            source = compressed.open(member) if windows else compressed.extractfile(member)
            path = directory / filename
            with source, path.open("wb") as destination:
                shutil.copyfileobj(source, destination)
            path.chmod(0o755)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version", help="Release version, such as v0.5.11")
    parser.add_argument("assets", type=Path, help="Directory with archives and checksums")
    parser.add_argument("output", type=Path, help="Directory for the npm tarball")
    args = parser.parse_args()
    version = args.version.removeprefix("v")
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        parser.error("expected a stable release version")

    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as temporary:
        package = Path(temporary)
        for name in ("hive.cjs", "package.json", "README.md"):
            shutil.copy(ROOT / "packaging" / "npm" / name, package)
        shutil.copy(ROOT / "LICENSE", package)
        manifest = package / "package.json"
        metadata = json.loads(manifest.read_text())
        metadata["version"] = version
        manifest.write_text(json.dumps(metadata, indent=2) + "\n")

        for (platform, arch), target in TARGETS.items():
            copy_binaries(args.assets, package, platform, arch, target)
        subprocess.run(
            ["npm", "pack", "--pack-destination", str(args.output.resolve())],
            cwd=package,
            check=True,
        )


if __name__ == "__main__":
    main()
