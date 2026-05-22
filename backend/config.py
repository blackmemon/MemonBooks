"""
config.py — Application settings loaded from .env file.
All secrets are read from environment; no hardcoded values.
"""

from pathlib import Path
from functools import lru_cache

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=str(Path(__file__).resolve().parent.parent / ".env"),
        env_file_encoding="utf-8",
        case_sensitive=True,
    )

    # App
    APP_NAME: str = "MemonBook ERP"
    APP_VERSION: str = "1.0.0"
    DEBUG: bool = False
    HOST: str = "127.0.0.1"
    PORT: int = 8765

    # Security keys (set by generate_keys.py → .env)
    APP_SECRET: str
    DB_SALT: str
    SECRET_TOKEN: str
    JWT_SECRET: str
    FIELD_ENCRYPT_KEY: str

    # Paths
    DATA_DIR: str = str(Path(__file__).resolve().parent.parent / "data")
    LOG_DIR: str = str(Path(__file__).resolve().parent.parent / "data" / "logs")
    BACKUP_DIR: str = str(Path(__file__).resolve().parent.parent / "data" / "backups")
    EXPORT_DIR: str = str(Path(__file__).resolve().parent.parent / "data" / "exports")

    # CORS (Tauri webview origin)
    ALLOWED_ORIGINS: list[str] = ["tauri://localhost", "http://tauri.localhost", "http://localhost"]

    # Token
    ACCESS_TOKEN_EXPIRE_MINUTES: int = 480
    REFRESH_TOKEN_EXPIRE_DAYS: int = 7


@lru_cache
def get_settings() -> Settings:
    return Settings()


settings = get_settings()
