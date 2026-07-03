#!/usr/bin/env python3
"""
Whiteboard Video Workflow environment check.

Checks:
  1. Python virtual environment for whiteboard-animation.
  2. OpenAI-compatible gateway configuration: url/model/key, or OPENAI_* vars.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
SKILL_DIR = SCRIPT_DIR.parent
SKILLS_ROOT = SKILL_DIR.parent
ANIMATION_SKILL = SKILLS_ROOT / "whiteboard-animation"


def load_env_file():
    values = {}
    env_file = SKILL_DIR / ".env"
    if not env_file.exists():
        return values

    for line in env_file.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or "=" not in stripped:
            continue
        key, value = stripped.split("=", 1)
        values[key.strip()] = value.strip().strip('"').strip("'")
    return values


def config_value(env_values, *names):
    for name in names:
        if os.environ.get(name):
            return os.environ[name]
    lowered = {k.lower(): v for k, v in env_values.items()}
    for name in names:
        if name in env_values and env_values[name]:
            return env_values[name]
        lower_name = name.lower()
        if lower_name in lowered and lowered[lower_name]:
            return lowered[lower_name]
    return None


def check_python_venv(check_only):
    setup_script = ANIMATION_SKILL / "scripts" / "setup_env.py"
    if not setup_script.exists():
        return {"ok": False, "error": f"setup_env.py not found: {setup_script}"}

    result = subprocess.run(
        [sys.executable, str(setup_script), "--check"],
        capture_output=True,
        text=True,
    )

    python_path = None
    for line in result.stdout.strip().splitlines():
        if line.startswith("PYTHON_PATH="):
            python_path = line.split("=", 1)[1]

    if result.returncode == 0 and python_path:
        return {"ok": True, "pythonPath": python_path}

    if not check_only:
        print("[..] Python dependencies are missing; installing...")
        result = subprocess.run(
            [sys.executable, str(setup_script)],
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            result2 = subprocess.run(
                [sys.executable, str(setup_script), "--check"],
                capture_output=True,
                text=True,
            )
            for line in result2.stdout.strip().splitlines():
                if line.startswith("PYTHON_PATH="):
                    python_path = line.split("=", 1)[1]
            if python_path:
                return {"ok": True, "pythonPath": python_path}

        return {"ok": False, "error": "Python virtual environment installation failed; run setup_env.py manually."}

    return {"ok": False, "error": "Python virtual environment is not ready; dependencies are missing."}


def check_gateway_config():
    env_values = load_env_file()
    url = config_value(env_values, "url", "OPENAI_API_BASE", "OPENAI_BASE_URL")
    model = config_value(env_values, "model", "OPENAI_IMAGE_MODEL", "OPENAI_MODEL")
    key = config_value(env_values, "key", "OPENAI_API_KEY", "CODEX_API_KEY")

    missing = []
    if not url:
        missing.append("url")
    if not model:
        missing.append("model")
    if not key:
        missing.append("key")

    if missing:
        env_file = SKILL_DIR / ".env"
        return {
            "ok": False,
            "error": (
                f"Missing {', '.join(missing)}. Create {env_file} with: "
                "url=http://81.68.73.15:3000/openai/v1, model=gpt-5.5, key=cr_xxx"
            ),
        }

    return {
        "ok": True,
        "url": url.rstrip("/"),
        "model": model,
        "keySource": "configured",
    }


def main():
    check_only = "--check-only" in sys.argv

    results = {
        "python": check_python_venv(check_only),
        "gateway": check_gateway_config(),
    }
    all_ok = all(item["ok"] for item in results.values())

    print("[check] Python virtual environment...")
    print("[check] OpenAI-compatible gateway config...")

    if all_ok:
        print("\n[OK] All environment checks passed.")
        print(f"PYTHON_PATH={results['python']['pythonPath']}")
        print(f"OPENAI_API_BASE={results['gateway']['url']}")
        print(f"OPENAI_IMAGE_MODEL={results['gateway']['model']}")
    else:
        print("\n[failed] Some checks did not pass:")
        for name, result in results.items():
            status = "OK" if result["ok"] else f"failed - {result.get('error', 'unknown error')}"
            print(f"  {name}: {status}")

    print(f"\nENV_RESULT={json.dumps({'allOk': all_ok, 'checks': results}, ensure_ascii=False)}")
    sys.exit(0 if all_ok else 1)


if __name__ == "__main__":
    main()
