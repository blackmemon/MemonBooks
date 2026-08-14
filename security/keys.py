"""Device fingerprinting and PBKDF2 key derivation for the 2-database
SQLCipher security scheme.

master.db key = PBKDF2(SHA256, DEVICE_HASH + APP_SECRET,                  DB_SALT, 600_000)
user.db key   = PBKDF2(SHA256, DEVICE_HASH + SECRET_TOKEN + USER_HASH,    DB_SALT, 600_000)

APP_SECRET / DB_SALT / SECRET_TOKEN / USER_HASH are generated once per
install by scripts/generate_keys.py and persisted (base64) in data/.keys,
which is gitignored and never leaves the machine.

Note on USER_HASH: this is a per-installation secret, not a per-login
username. MemonBook is a single-installation ERP shared by several staff
accounts (cashier, accountant, admin, ...) all reading/writing the same
user.db; re-deriving the database key from whichever username is typed at
the login screen would make the file only openable by one specific staff
account. Instead user.db is opened once at app startup using the fixed
installation secret, and individual staff logins are authenticated
separately (bcrypt check against the Users table already inside the
opened database) by security.auth.
"""
from __future__ import annotations

import base64
import hashlib
import json
import os
import platform
import stat
import uuid
from dataclasses import dataclass
from functools import lru_cache

from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC

from config import KEY_LENGTH_BYTES, KEYS_FILE, PBKDF2_ITERATIONS


class KeysNotGeneratedError(RuntimeError):
    """Raised when data/.keys does not exist yet — run scripts/generate_keys.py."""


@dataclass(frozen=True)
class InstallKeys:
    app_secret: bytes
    db_salt: bytes
    secret_token: bytes
    user_hash: bytes

    @classmethod
    def generate(cls) -> "InstallKeys":
        return cls(
            app_secret=os.urandom(32),
            db_salt=os.urandom(16),
            secret_token=os.urandom(32),
            user_hash=os.urandom(32),
        )

    def to_file(self, path=KEYS_FILE) -> None:
        payload = {
            "app_secret": base64.b64encode(self.app_secret).decode(),
            "db_salt": base64.b64encode(self.db_salt).decode(),
            "secret_token": base64.b64encode(self.secret_token).decode(),
            "user_hash": base64.b64encode(self.user_hash).decode(),
        }
        path.write_text(json.dumps(payload, indent=2))
        os.chmod(path, stat.S_IRUSR | stat.S_IWUSR)  # 600: owner read/write only

    @classmethod
    def from_file(cls, path=KEYS_FILE) -> "InstallKeys":
        if not path.exists():
            raise KeysNotGeneratedError(
                f"{path} not found. Run `python scripts/generate_keys.py` first."
            )
        payload = json.loads(path.read_text())
        return cls(
            app_secret=base64.b64decode(payload["app_secret"]),
            db_salt=base64.b64decode(payload["db_salt"]),
            secret_token=base64.b64decode(payload["secret_token"]),
            user_hash=base64.b64decode(payload["user_hash"]),
        )


@lru_cache(maxsize=1)
def get_install_keys() -> InstallKeys:
    return InstallKeys.from_file()


@lru_cache(maxsize=1)
def get_device_hash() -> bytes:
    """A stable-per-machine identifier. Not cryptographically secret on its
    own — it is combined with APP_SECRET/SECRET_TOKEN before use."""
    machine_id_path = "/etc/machine-id"
    if os.path.exists(machine_id_path):
        raw = open(machine_id_path, "r", encoding="utf-8").read().strip()
    else:
        raw = f"{platform.node()}:{uuid.getnode()}"
    return hashlib.sha256(raw.encode("utf-8")).digest()


def _pbkdf2(material: bytes, salt: bytes) -> bytes:
    kdf = PBKDF2HMAC(
        algorithm=hashes.SHA256(),
        length=KEY_LENGTH_BYTES,
        salt=salt,
        iterations=PBKDF2_ITERATIONS,
    )
    return kdf.derive(material)


def derive_master_key() -> str:
    """Returns the SQLCipher key for master.db as a hex string."""
    keys = get_install_keys()
    material = get_device_hash() + keys.app_secret
    return _pbkdf2(material, keys.db_salt).hex()


def derive_user_key() -> str:
    """Returns the SQLCipher key for user.db as a hex string. Fixed per
    installation — see module docstring for why this does not vary by
    which staff member is logging in."""
    keys = get_install_keys()
    material = get_device_hash() + keys.secret_token + keys.user_hash
    return _pbkdf2(material, keys.db_salt).hex()
