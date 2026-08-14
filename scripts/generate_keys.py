#!/usr/bin/env python3
"""Run once per install (spec: Setup Sequence, step 1).

Generates data/.keys, the per-installation secret material that
security/keys.py combines with the device fingerprint to derive the
SQLCipher encryption keys for master.db and user.db. Never commit this
file — it is gitignored.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from config import KEYS_FILE
from security.keys import InstallKeys


def main() -> None:
    if KEYS_FILE.exists():
        print(f"{KEYS_FILE} already exists — leaving it in place.")
        print("Delete it manually first if you really want to regenerate "
              "(this would make any existing encrypted databases unreadable).")
        return
    InstallKeys.generate().to_file()
    print(f"Generated {KEYS_FILE}")


if __name__ == "__main__":
    main()
