#!/usr/bin/env python3

import argparse
import re
import subprocess
import sys
from pathlib import Path


def run(cmd: list[str], *, check: bool = True) -> subprocess.CompletedProcess:
    print(f"+ {' '.join(cmd)}")
    return subprocess.run(cmd, check=check)


def capture(cmd: list[str]) -> str:
    return subprocess.check_output(cmd, text=True).strip()


def die(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    sys.exit(1)


def validate_version(version: str) -> None:
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?", version):
        die(
            f"invalid version {version!r}; expected something like "
            "1.2.3, 1.2.3-beta.1, or 1.2.3+build.1"
        )


def check_clean_git() -> None:
    status = capture(["git", "status", "--porcelain"])
    if status:
        die("git working tree is not clean; commit or stash changes first")


def update_cargo_toml(version: str, path: Path) -> None:
    if not path.exists():
        die(f"{path} not found")

    text = path.read_text()

    # This updates the first `version = "..."`
    # In a normal single-crate Cargo.toml, that is usually [package].version.
    new_text, count = re.subn(
        r'(?m)^version\s*=\s*"[^"]+"',
        f'version = "{version}"',
        text,
        count=1,
    )

    if count != 1:
        die(f"could not find exactly one package version line in {path}")

    path.write_text(new_text)
    print(f"Updated {path}: version = {version!r}")


def changelog_contains_version(version: str, path: Path) -> bool:
    if not path.exists():
        return False

    text = path.read_text()
    patterns = [
        rf"(?m)^##\s+\[?v?{re.escape(version)}\]?\b",
        rf"(?m)^##\s+Version\s+v?{re.escape(version)}\b",
        rf"(?m)^#\s+\[?v?{re.escape(version)}\]?\b",
    ]
    return any(re.search(p, text) for p in patterns)


def confirm_changelog(version: str) -> None:
    changelog = Path("CHANGELOG.md")

    if not changelog.exists():
        die("CHANGELOG.md not found")

    if changelog_contains_version(version, changelog):
        print(f"CHANGELOG.md appears to contain a section for {version}.")
    else:
        print(f"CHANGELOG.md does not obviously contain a section for {version}.")

    while True:
        answer = input("Has CHANGELOG.md been updated for this release (in the Unreleased section)? [y/n] ").strip().lower()
        if answer in {"y", "yes"}:
            return
        if answer in {"n", "no"}:
            die("update CHANGELOG.md first, then rerun this script")

def update_changelog(version: str, path: Path) -> bool:
    if not path.exists():
        die(f"{path} not found")

    text = path.read_text()

    # Already has this version section?
    if changelog_contains_version(version, path):
        print(f"{path} already contains a section for {version}; not moving Unreleased.")
        return False

    unreleased_match = re.search(
        r"(?m)^##\s+Unreleased\s*$",
        text,
    )
    if not unreleased_match:
        die(f"could not find '## Unreleased' section in {path}")

    next_heading_match = re.search(
        r"(?m)^##\s+",
        text[unreleased_match.end():],
    )

    if not next_heading_match:
        die(f"could not find next '##' release section after Unreleased in {path}")

    unreleased_start = unreleased_match.end()
    next_heading_start = unreleased_match.end() + next_heading_match.start()

    unreleased_body = text[unreleased_start:next_heading_start].strip()

    if not unreleased_body:
        die("CHANGELOG.md Unreleased section is empty; nothing to release")

    before_unreleased_body = text[:unreleased_start]
    after_unreleased_body = text[next_heading_start:]

    new_section = f"\n\n## Version {version}\n\n{unreleased_body}\n\n"

    new_text = before_unreleased_body.rstrip() + new_section + after_unreleased_body.lstrip()

    path.write_text(new_text)
    print(f"Moved CHANGELOG.md Unreleased entries to Version {version}")
    return True

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Prepare a Rust release for cargo-dist."
    )
    parser.add_argument(
        "--version",
        required=True,
        help="Release version, e.g. 1.2.3",
    )
    parser.add_argument(
        "--no-commit",
        action="store_true",
        help="Modify files but do not commit or tag",
    )
    parser.add_argument(
        "--no-lock-update",
        action="store_true",
        help="Do not run `cargo update --workspace` after editing Cargo.toml",
    )
    parser.add_argument(
        "--tag-prefix",
        default="v",
        help='Git tag prefix; default is "v", producing tags like v1.2.3',
    )
    parser.add_argument(
        "--allow-dirty",
        action="store_true",
        help="Allow running with an already-dirty git working tree",
    )
    parser.add_argument(
        "--library-only",
        action="store_true",
        help="Skip `dist plan` (cargo-dist) and release as a plain library crate via `cargo publish`",
    )

    args = parser.parse_args()
    version = args.version
    tag = f"{args.tag_prefix}{version}"

    validate_version(version)

    if not args.allow_dirty:
        check_clean_git()

    existing_tags = capture(["git", "tag", "--list", tag])
    if existing_tags:
        die(f"git tag {tag!r} already exists")

    confirm_changelog(version)

    update_changelog(version, Path("CHANGELOG.md"))

    cargo_toml_changed = update_cargo_toml(version, Path("Cargo.toml"))

    update_cargo_toml(version, Path("Cargo.toml"))

    if not args.no_lock_update:
        run(["cargo", "update", "--workspace"])

    run(["cargo", "test"])
    if not args.library_only:
        run(["dist", "plan"])

    if args.no_commit:
        print()
        print("Stopped before commit/tag because --no-commit was supplied.")
        print("Review changes with:")
        print("  git diff")
        return

    files_to_add = ["Cargo.toml", "CHANGELOG.md"]
    if not args.library_only:
        files_to_add.append("Cargo.lock")
    run(["git", "add", *files_to_add])
    run(["git", "commit", "-m", f"Release {tag}"])
    run(["git", "tag", tag])

    run(["cargo", "publish"])
    run(["git", "push", "origin", "HEAD", "--tags"])

    print()
    if args.library_only:
        print("Done I think.")
    else:
        print("Release commit and tag created, pushed to GitHub, and published to crates.io. Next update the bioconda recipe and submit a PR to bioconda-recipes.")


if __name__ == "__main__":
    main()