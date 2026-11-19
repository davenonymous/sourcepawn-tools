# Copyright 2026 davenonymous
#
# The logic of the SourcePawn Lint action (action.yml): gets pawncritic and
# pawntidy (a checked release from the tool cache, or a build of the action's
# own sources), chooses the files, runs both tools, writes the annotations or
# the SARIF file, the job summary and the step outputs. It never fails the step
# itself: the output `failed` tells action.yml to, after the SARIF upload.
#
# Needs Python 3.9 or newer and nothing else; downloads go through `gh`.

from __future__ import annotations

import hashlib
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath, PureWindowsPath

SOURCEPAWN_SUFFIXES = (".sp", ".inc")
SUMMARY_VIOLATION_LIMIT = 200
NULL_SHA = "0" * 40
PLATFORMS = {
    ("Linux", "X64"): ("linux-x86_64", "tar.gz", ""),
    ("Windows", "X64"): ("windows-x86_64", "zip", ".exe"),
}
ANNOTATION_COMMANDS = {"error": "error", "warning": "warning", "info": "notice", "hint": "notice"}
WORKSPACE_VERSION_LINE = re.compile(r'version\s*=\s*"([^"]+)"\s*(#.*)?')


class ActionError(Exception):
    """A mistake in the inputs or the environment that stops the action."""


@dataclass(frozen=True)
class Inputs:
    paths: list[str]
    version: str
    critic_options: list[str]
    critic_args: list[str]
    tidy_args: list[str]
    tidy: bool
    sarif: bool
    changed_only: bool
    fail_on_violations: bool


@dataclass(frozen=True)
class Tools:
    version: str
    critic: Path
    tidy: Path


@dataclass(frozen=True)
class CriticResult:
    status: int
    reported: bool
    violations: list[dict]
    errors: list[dict]


@dataclass(frozen=True)
class TidyResult:
    status: int
    reported: bool
    unformatted: list[str]
    errors: list[dict]


def counted(count: int, singular: str, plural: str) -> str:
    return f"{count} {singular if count == 1 else plural}"


def parse_bool(name: str, text: str) -> bool:
    value = text.strip().lower()
    if value not in ("true", "false"):
        raise ActionError(f"input {name} must be true or false, not {text!r}")
    return value == "true"


def parse_inputs(env: dict[str, str]) -> Inputs:
    paths = [line.strip() for line in env.get("INPUT_PATHS", "").splitlines() if line.strip()]
    if not paths:
        raise ActionError("input paths is empty: give at least one path, one per line")

    critic_options = []
    for option, name in (("--severity", "SEVERITY"), ("--theme", "THEME"), ("--config", "CONFIG")):
        value = env.get(f"INPUT_{name}", "").strip()
        if value:
            critic_options += [option, value]

    return Inputs(
        paths=paths,
        version=env.get("INPUT_VERSION", "").strip(),
        critic_options=critic_options,
        critic_args=shlex.split(env.get("INPUT_ARGS", "")),
        tidy_args=shlex.split(env.get("INPUT_TIDY_ARGS", "")),
        tidy=parse_bool("tidy", env.get("INPUT_TIDY", "false")),
        sarif=parse_bool("sarif", env.get("INPUT_SARIF", "false")),
        changed_only=parse_bool("changed-only", env.get("INPUT_CHANGED_ONLY", "false")),
        fail_on_violations=parse_bool("fail-on-violations", env.get("INPUT_FAIL_ON_VIOLATIONS", "true")),
    )


def run_program(command: list[str], **options) -> subprocess.CompletedProcess:
    """subprocess.run, with a missing program as an ActionError."""
    try:
        return subprocess.run(command, **options)
    except FileNotFoundError:
        raise ActionError(f"program not found: {command[0]}") from None


def parsed_json(tool: str, text: str) -> dict:
    try:
        return json.loads(text)
    except json.JSONDecodeError as error:
        raise ActionError(f"{tool} wrote output that is not JSON: {error}") from None


def workspace_version(action_path: Path) -> str:
    """The `version` of `[workspace.package]` in the action's Cargo.toml."""
    manifest = action_path / "Cargo.toml"
    in_workspace_package = False
    for line in manifest.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            in_workspace_package = stripped == "[workspace.package]"
            continue
        if not in_workspace_package:
            continue
        match = WORKSPACE_VERSION_LINE.fullmatch(stripped)
        if match:
            return match.group(1)
    raise ActionError(f'{manifest} has no version = "..." in [workspace.package]')


def runner_platform(env: dict[str, str]) -> tuple[str, str, str]:
    key = (env.get("RUNNER_OS", ""), env.get("RUNNER_ARCH", ""))
    if key not in PLATFORMS:
        supported = ", ".join(f"{os_name} {arch}" for os_name, arch in PLATFORMS)
        raise ActionError(f"no release build for {key[0]} {key[1]}; supported runners: {supported}")
    return PLATFORMS[key]


def build_from_source(action_path: Path, env: dict[str, str]) -> Tools:
    _, _, exe = runner_platform(env)
    target_dir = Path(env["RUNNER_TEMP"]) / "sourcepawn-tools-target"
    print("::group::Building pawncritic and pawntidy from the action's sources", flush=True)
    run_program(
        ["cargo", "build", "--release", "--locked", "--manifest-path", str(action_path / "Cargo.toml"),
         "--target-dir", str(target_dir), "-p", "pawncritic", "-p", "pawntidy"],
        check=True,
    )
    print("::endgroup::", flush=True)
    release_dir = target_dir / "release"
    return Tools(workspace_version(action_path), release_dir / f"pawncritic{exe}", release_dir / f"pawntidy{exe}")


def checked_sha256(archive: Path, sums_file: Path) -> None:
    expected = None
    for line in sums_file.read_text().splitlines():
        digest, _, name = line.partition("  ")
        if name.strip() == archive.name:
            expected = digest.strip().lower()
    if expected is None:
        raise ActionError(f"SHA256SUMS of the release lists no {archive.name}")
    actual = hashlib.sha256(archive.read_bytes()).hexdigest()
    if actual != expected:
        raise ActionError(f"{archive.name} has SHA-256 {actual}, but SHA256SUMS says {expected}")


def checked_tar_members(archive: Path, packed: tarfile.TarFile) -> list[tarfile.TarInfo]:
    """The members of a tar archive, checked and normalized as tarfile's data filter would, for Pythons without it."""
    members = packed.getmembers()
    for member in members:
        member.mode &= 0o755
        member.uid = member.gid = 0
        member.uname = member.gname = ""
        name = member.name
        if not (member.isfile() or member.isdir()):
            raise ActionError(f"{archive.name} holds {name}, which is neither a regular file nor a directory")
        if PurePosixPath(name).is_absolute() or PureWindowsPath(name).is_absolute() or name.startswith(("/", "\\")):
            raise ActionError(f"{archive.name} holds the absolute path {name}")
        if ".." in PureWindowsPath(name).parts:
            raise ActionError(f"{archive.name} holds {name}, which leaves the archive's directory")
    return members


def unpack(archive: Path, destination: Path) -> None:
    if archive.name.endswith(".zip"):
        with zipfile.ZipFile(archive) as packed:
            packed.extractall(destination)
        return
    with tarfile.open(archive) as packed:
        if hasattr(tarfile, "data_filter"):
            try:
                packed.extractall(destination, filter="data")
            except tarfile.FilterError as error:
                raise ActionError(f"{archive.name} holds an entry the data filter refuses: {error}") from None
            return
        packed.extractall(destination, members=checked_tar_members(archive, packed))


def repository_from_action_path(action_path: Path) -> str | None:
    """OWNER/REPO of a path `<...>/_actions/OWNER/REPO/REF[/...]`, as runners check actions out."""
    parts = action_path.parts
    if "_actions" not in parts:
        return None
    start = len(parts) - 1 - parts[::-1].index("_actions")
    names = parts[start + 1:start + 4]
    if len(names) < 3:
        return None
    return f"{names[0]}/{names[1]}"


def release_repository(env: dict[str, str]) -> str:
    """The repository the action comes from, whose releases hold the tools."""
    if env.get("ACTION_REPOSITORY"):
        return env["ACTION_REPOSITORY"]
    action_path = Path(env["GITHUB_ACTION_PATH"]).resolve()
    if is_below(action_path, Path(env["GITHUB_WORKSPACE"]).resolve()):
        return env["GITHUB_REPOSITORY"]
    repository = repository_from_action_path(action_path)
    if repository is None:
        raise ActionError(
            "cannot tell which repository the action comes from: github.action_repository is empty "
            f"and {action_path} is not below _actions/OWNER/REPO/REF"
        )
    return repository


def download_release(version: str, env: dict[str, str]) -> Tools:
    platform, extension, exe = runner_platform(env)
    cache_root = Path(env["RUNNER_TOOL_CACHE"]) / "sourcepawn-tools" / version
    tool_dir = cache_root / platform
    complete_marker = cache_root / f"{platform}.complete"
    tools = Tools(version, tool_dir / f"pawncritic{exe}", tool_dir / f"pawntidy{exe}")
    if complete_marker.exists():
        print(f"Using sourcepawn-tools {version} from the tool cache", flush=True)
        return tools

    repository = release_repository(env)
    stem = f"sourcepawn-tools-{version}-{platform}"
    archive_name = f"{stem}.{extension}"
    with tempfile.TemporaryDirectory(dir=env["RUNNER_TEMP"]) as scratch:
        scratch_dir = Path(scratch)
        print(f"Downloading {archive_name} from {repository} release v{version}", flush=True)
        run_program(
            ["gh", "release", "download", f"v{version}", "--repo", repository,
             "--pattern", archive_name, "--pattern", "SHA256SUMS", "--dir", str(scratch_dir)],
            check=True,
        )
        archive = scratch_dir / archive_name
        checked_sha256(archive, scratch_dir / "SHA256SUMS")
        unpack(archive, scratch_dir)
        shutil.rmtree(tool_dir, ignore_errors=True)
        tool_dir.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(scratch_dir / stem), str(tool_dir))
    for tool in (tools.critic, tools.tidy):
        tool.chmod(0o755)
    complete_marker.touch()
    return tools


def fetched_commit(sha: str) -> bool:
    """Whether the commit is in the checkout, fetching it when it is not yet."""
    present = run_program(["git", "cat-file", "-e", f"{sha}^{{commit}}"], capture_output=True)
    if present.returncode == 0:
        return True
    fetch = run_program(["git", "fetch", "--no-tags", "--depth=1", "origin", sha], capture_output=True, text=True)
    return fetch.returncode == 0


def changed_base(event_name: str, event: dict) -> str | None:
    if event_name in ("pull_request", "pull_request_target"):
        return event["pull_request"]["base"]["sha"]
    if event_name == "push" and event.get("before", NULL_SHA) != NULL_SHA:
        return event["before"]
    return None


def is_below(file: Path, root: Path) -> bool:
    return file == root or root in file.parents


def is_sourcepawn_file(name: str) -> bool:
    return name.lower().endswith(SOURCEPAWN_SUFFIXES)


def changed_files(inputs: Inputs, env: dict[str, str]) -> list[str] | None:
    """The changed SourcePawn files below the paths, or None to check all paths."""
    event = json.loads(Path(env["GITHUB_EVENT_PATH"]).read_text())
    base = changed_base(env["GITHUB_EVENT_NAME"], event)
    if base is None:
        print(f"changed-only: no base commit on a {env['GITHUB_EVENT_NAME']} event, checking all paths", flush=True)
        return None
    if not fetched_commit(base):
        print(f"::notice title=SourcePawn Lint::changed-only: could not fetch {base}, checking all paths", flush=True)
        return None

    diff = run_program(
        ["git", "diff", "--name-only", "--relative", "--diff-filter=d", "-z", base, "HEAD"],
        check=True, capture_output=True, text=True,
    )
    roots = [Path(path).resolve() for path in inputs.paths]
    changed = [name for name in diff.stdout.split("\0") if is_sourcepawn_file(name)]
    return [name for name in changed if any(is_below(Path(name).resolve(), root) for root in roots)]


def run_critic(tools: Tools, inputs: Inputs, targets: list[str]) -> CriticResult:
    command = [str(tools.critic), *inputs.critic_options, *inputs.critic_args, "--format", "json", "--", *targets]
    run = run_program(command, stdout=subprocess.PIPE, text=True, encoding="utf-8")
    if run.returncode not in (0, 1, 2):
        raise ActionError(f"pawncritic ended with status {run.returncode}")
    if not run.stdout.strip():
        return CriticResult(run.returncode, False, [], [])
    report = parsed_json("pawncritic", run.stdout)
    return CriticResult(run.returncode, True, report["violations"], report["errors"])


def github_escape(text: str, is_property: bool) -> str:
    """Escapes text for a GitHub workflow command as pawncritic's github format does."""
    escaped = text.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
    if is_property:
        escaped = escaped.replace(":", "%3A").replace(",", "%2C")
    return escaped


def annotation(violation: dict) -> str:
    """A violation as the workflow command `pawncritic --format github` writes for it."""
    slashed_file = violation["file"].replace("\\", "/")
    return (
        f"::{ANNOTATION_COMMANDS[violation['level']]} file={github_escape(slashed_file, True)},"
        f"line={violation['line']},col={violation['column']},"
        f"endLine={violation['end_line']},endColumn={violation['end_column']},"
        f"title={github_escape(violation['policy'], True)}::{github_escape(violation['message'], False)}"
    )


def write_annotations(critic: CriticResult) -> None:
    """Writes the annotations as UTF-8 with bare line feeds, as pawncritic does, whatever the console's encoding."""
    text = "".join(annotation(violation) + "\n" for violation in critic.violations)
    sys.stdout.flush()
    sys.stdout.buffer.write(text.encode("utf-8"))
    sys.stdout.buffer.flush()


def write_sarif(tools: Tools, inputs: Inputs, targets: list[str], sarif_file: Path) -> bool:
    """Runs pawncritic again for the SARIF file; whether it wrote one."""
    command = [str(tools.critic), *inputs.critic_options, *inputs.critic_args, "--format", "sarif", "--", *targets]
    with open(sarif_file, "w", encoding="utf-8") as output:
        run_program(command, stdout=output, stderr=subprocess.DEVNULL)
    if sarif_file.stat().st_size > 0:
        return True
    sarif_file.unlink()
    return False


def run_tidy(tools: Tools, inputs: Inputs, targets: list[str]) -> TidyResult:
    command = [str(tools.tidy), "--check", *inputs.tidy_args, "--format", "json", "--", *targets]
    run = run_program(command, stdout=subprocess.PIPE, text=True, encoding="utf-8")
    if run.returncode not in (0, 1, 2):
        raise ActionError(f"pawntidy ended with status {run.returncode}")
    if not run.stdout.strip():
        return TidyResult(run.returncode, False, [], [])
    files = parsed_json("pawntidy", run.stdout)["files"]
    unformatted = [file["path"] for file in files if file["changed"]]
    errors = [file for file in files if file.get("error")]
    for path in unformatted:
        print(f"would reformat {path}", flush=True)
    return TidyResult(run.returncode, True, unformatted, errors)


def table_cell(text: str) -> str:
    return " ".join(str(text).split()).replace("|", "\\|")


def count_by(items: list[dict], key: str) -> list[tuple]:
    counts: dict = {}
    for item in items:
        counts[item[key]] = counts.get(item[key], 0) + 1
    return sorted(counts.items(), key=lambda entry: (-entry[1], str(entry[0])))


def hidden_violations_line(hidden: int, sarif: bool) -> str:
    where = (
        "the SARIF file has them all; code scanning has them once the upload succeeds"
        if sarif else "the log of this step has them all"
    )
    verb = "is" if hidden == 1 else "are"
    return f"{counted(hidden, 'more violation', 'more violations')} {verb} not shown; {where}."


def critic_summary(critic: CriticResult, sarif: bool) -> list[str]:
    lines = ["### pawncritic", ""]
    if not critic.reported and critic.status == 2:
        return lines + ["pawncritic could not complete the run; its errors are in the log of this step.", ""]
    if not critic.violations and not critic.errors and critic.status == 2:
        return lines + ["pawncritic reported problems before checking any file; they are in the log of this step.", ""]
    if not critic.violations and not critic.errors:
        return lines + ["No violations.", ""]

    by_severity = sorted(count_by(critic.violations, "severity"), reverse=True)
    if by_severity:
        lines += ["| Severity | Violations |", "|---:|---:|"]
        lines += [f"| {level} | {count} |" for level, count in by_severity]
        lines += ["", "| Policy | Violations |", "|---|---:|"]
        lines += [f"| `{policy}` | {count} |" for policy, count in count_by(critic.violations, "policy")]
        lines += ["", "| Location | Severity | Policy | Message |", "|---|---:|---|---|"]
        for violation in critic.violations[:SUMMARY_VIOLATION_LIMIT]:
            location = f"{violation['file']}:{violation['line']}:{violation['column']}"
            lines.append(
                f"| `{table_cell(location)}` | {violation['severity']} | `{violation['policy']}` "
                f"| {table_cell(violation['message'])} |"
            )
        hidden = len(critic.violations) - SUMMARY_VIOLATION_LIMIT
        if hidden > 0:
            lines += ["", hidden_violations_line(hidden, sarif)]
        lines.append("")

    if critic.errors:
        lines += ["Errors (the result is incomplete):", ""]
        for error in critic.errors:
            policy = f" `{error['policy']}`" if error.get("policy") else ""
            lines.append(f"- `{table_cell(error.get('file') or '-')}`{policy}: {table_cell(error['message'])}")
        lines.append("")
    return lines


def tidy_summary(tidy: TidyResult) -> list[str]:
    lines = ["### pawntidy", ""]
    if not tidy.reported and tidy.status == 2:
        return lines + ["pawntidy could not complete the run; its errors are in the log of this step.", ""]
    if not tidy.unformatted and not tidy.errors:
        return lines + ["Every file is formatted.", ""]
    if tidy.unformatted:
        lines += [f"Files pawntidy would reformat ({len(tidy.unformatted)}; `pawntidy --write` fixes them):", ""]
        lines += [f"- `{table_cell(path)}`" for path in tidy.unformatted]
        lines.append("")
    if tidy.errors:
        lines += ["Errors:", ""]
        lines += [f"- `{table_cell(file['path'])}`: {table_cell(file['error'])}" for file in tidy.errors]
        lines.append("")
    return lines


def write_summary(env: dict[str, str], lines: list[str]) -> None:
    with open(env["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as summary:
        summary.write("\n".join(lines) + "\n")


def write_outputs(env: dict[str, str], outputs: dict[str, str]) -> None:
    with open(env["GITHUB_OUTPUT"], "a", encoding="utf-8") as output_file:
        for name, value in outputs.items():
            output_file.write(f"{name}={value}\n")


def failure_reason(inputs: Inputs, critic: CriticResult, tidy: TidyResult | None) -> str:
    if critic.status == 2:
        return "pawncritic could not complete the run (status 2); its errors are in the log"
    if tidy and tidy.status == 2:
        return "pawntidy could not complete the run (status 2); its errors are in the log"
    if not inputs.fail_on_violations:
        return ""
    if critic.violations:
        return f"pawncritic reported {counted(len(critic.violations), 'violation', 'violations')}"
    if tidy and tidy.unformatted:
        return f"pawntidy would reformat {counted(len(tidy.unformatted), 'file', 'files')}"
    return ""


def lint(inputs: Inputs, env: dict[str, str]) -> None:
    action_path = Path(env["GITHUB_ACTION_PATH"])
    if inputs.version == "source":
        tools = build_from_source(action_path, env)
    else:
        tools = download_release(inputs.version or workspace_version(action_path), env)

    summary = ["## SourcePawn Lint", "", f"sourcepawn-tools {tools.version}", ""]
    outputs = {"version": tools.version, "violations": "0", "unformatted": "0", "sarif-file": "", "failed": "false"}

    targets = inputs.paths
    if inputs.changed_only:
        changed = changed_files(inputs, env)
        if changed is not None:
            targets = changed
            summary += [f"Changed SourcePawn files checked: {len(changed)}", ""]
    if not targets:
        write_summary(env, summary + ["No SourcePawn file changed; nothing to check.", ""])
        write_outputs(env, outputs)
        return

    critic = run_critic(tools, inputs, targets)
    if inputs.sarif:
        sarif_file = Path(env["RUNNER_TEMP"]) / "pawncritic.sarif"
        if write_sarif(tools, inputs, targets, sarif_file):
            outputs["sarif-file"] = str(sarif_file)
    else:
        write_annotations(critic)
    summary += critic_summary(critic, inputs.sarif)
    outputs["violations"] = str(len(critic.violations))

    tidy = run_tidy(tools, inputs, targets) if inputs.tidy else None
    if tidy:
        summary += tidy_summary(tidy)
        outputs["unformatted"] = str(len(tidy.unformatted))

    reason = failure_reason(inputs, critic, tidy)
    if reason:
        outputs["failed"] = "true"
        outputs["failure-reason"] = reason
    write_summary(env, summary)
    write_outputs(env, outputs)


def main() -> int:
    env = dict(os.environ)
    try:
        lint(parse_inputs(env), env)
    except ActionError as error:
        print(f"::error title=SourcePawn Lint::{error}", flush=True)
        return 1
    except subprocess.CalledProcessError as error:
        command = shlex.join(error.cmd)
        print(f"::error title=SourcePawn Lint::{command} failed with status {error.returncode}", flush=True)
        return 1
    except Exception as error:
        print(f"::error title=SourcePawn Lint::unexpected error: {type(error).__name__}: {error}", flush=True)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
