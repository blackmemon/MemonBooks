"""
generate_keys.py — One-time script to generate APP_SECRET and DB_SALT.
Run ONCE on initial deployment. Store output in .env securely.
NEVER run again — changing these keys locks out all existing databases.
"""

import os
import secrets
import base64
import json
from pathlib import Path
from datetime import datetime, timezone


def generate_secret(byte_length: int = 32) -> str:
    """Generate a cryptographically secure random secret, base64-encoded."""
    raw = secrets.token_bytes(byte_length)
    return base64.urlsafe_b64encode(raw).decode("utf-8")


def generate_hex_salt(byte_length: int = 32) -> str:
    """Generate a cryptographically secure random salt as hex string."""
    return secrets.token_hex(byte_length)


def write_env_file(keys: dict[str, str], env_path: Path) -> None:
    """Write keys to .env file with clear warnings."""
    lines = [
        "# MemonBook ERP — Auto-Generated Security Keys",
        f"# Generated: {datetime.now(timezone.utc).isoformat()}",
        "# WARNING: NEVER share, commit, or regenerate these keys.",
        "# Regenerating locks out ALL existing encrypted databases.",
        "",
    ]
    for key, value in keys.items():
        lines.append(f"{key}={value}")
    lines.append("")

    env_path.write_text("\n".join(lines), encoding="utf-8")


def write_json_backup(keys: dict[str, str], backup_path: Path) -> None:
    """Write keys to a JSON backup file for offline cold storage."""
    payload = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "warning": "KEEP THIS FILE OFFLINE AND ENCRYPTED. Loss = permanent data loss.",
        "keys": keys,
    }
    backup_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def main() -> None:
    project_root = Path(__file__).resolve().parent.parent
    env_path = project_root / ".env"
    backup_path = project_root / "data" / "keys_backup.json"

    if env_path.exists():
        print("ERROR: .env already exists. Delete it manually if you truly need to regenerate.")
        print("WARNING: Regenerating keys = all encrypted databases become permanently inaccessible.")
        raise SystemExit(1)

    keys = {
        "APP_SECRET": generate_secret(48),
        "DB_SALT": generate_hex_salt(32),
        "SECRET_TOKEN": generate_secret(32),
        "JWT_SECRET": generate_secret(64),
        "FIELD_ENCRYPT_KEY": generate_secret(32),
    }

    write_env_file(keys, env_path)
    backup_path.parent.mkdir(parents=True, exist_ok=True)
    write_json_backup(keys, backup_path)

    # Set restrictive permissions on both files (owner read-only)
    os.chmod(env_path, 0o600)
    os.chmod(backup_path, 0o600)

    print("Keys generated successfully.")
    print(f"  .env         → {env_path}")
    print(f"  Backup JSON  → {backup_path}")
    print()
    print("CRITICAL ACTIONS REQUIRED:")
    print("  1. Copy keys_backup.json to encrypted offline storage (USB + password manager).")
    print("  2. Delete keys_backup.json from this machine after cold backup is confirmed.")
    print("  3. Add .env to .gitignore IMMEDIATELY before any git operations.")
    print("  4. Never run this script again.")


if __name__ == "__main__":
    main()
