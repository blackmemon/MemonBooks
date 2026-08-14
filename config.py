"""Central application configuration and paths."""
from __future__ import annotations

from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent
DATA_DIR = BASE_DIR / "data"
ATTACHMENTS_DIR = DATA_DIR / "attachments"
KEYS_FILE = DATA_DIR / ".keys"
MASTER_DB_PATH = DATA_DIR / "master.db"
USER_DB_PATH = DATA_DIR / "user.db"

APP_NAME = "MemonBook ERP"
APP_VERSION = "0.1.0"

# Security
PBKDF2_ITERATIONS = 600_000
KEY_LENGTH_BYTES = 32  # 256-bit

# Financial precision
DECIMAL_PLACES = 2
CURRENCY = "PKR"

DATA_DIR.mkdir(parents=True, exist_ok=True)
ATTACHMENTS_DIR.mkdir(parents=True, exist_ok=True)
