"""
encryption.py — AES-256 encryption and PBKDF2 key derivation for MemonBook ERP.

Key derivation:
  master.db key = PBKDF2(SHA256, DEVICE_HASH + APP_SECRET, DB_SALT, 600000 iter)
  user.db key   = PBKDF2(SHA256, DEVICE_HASH + SECRET_TOKEN + USER_HASH, DB_SALT, 600000 iter)

Field-level AES-256-GCM encryption is used for sensitive columns:
  cnic, passport_no, bank_account, bank_iban, imei_1, imei_2, mac_address,
  audit old_values, audit new_values.
"""

import base64
import hashlib
import os
import struct
from typing import Optional

from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
from cryptography.hazmat.primitives import hashes


# ──────────────────────────────────────────────────────────────────────────────
# PBKDF2 key derivation
# ──────────────────────────────────────────────────────────────────────────────

_PBKDF2_ITERATIONS = 600_000
_KEY_LENGTH = 32  # 256 bits


def _pbkdf2_derive(password_material: str, salt_hex: str) -> bytes:
    """Derive a 32-byte key using PBKDF2-HMAC-SHA256."""
    salt = bytes.fromhex(salt_hex)
    kdf = PBKDF2HMAC(
        algorithm=hashes.SHA256(),
        length=_KEY_LENGTH,
        salt=salt,
        iterations=_PBKDF2_ITERATIONS,
    )
    return kdf.derive(password_material.encode("utf-8"))


def derive_master_db_key(device_hash: str, app_secret: str, db_salt: str) -> str:
    """
    Derive the SQLCipher PRAGMA key for master.db.
    Returns a hex string suitable for: PRAGMA key = "x'<hex>'";
    """
    material = device_hash + app_secret
    key_bytes = _pbkdf2_derive(material, db_salt)
    return key_bytes.hex()


def derive_user_db_key(
    device_hash: str,
    secret_token: str,
    user_hash: str,
    db_salt: str,
) -> str:
    """
    Derive the SQLCipher PRAGMA key for user.db.
    Returns a hex string suitable for: PRAGMA key = "x'<hex>'";
    """
    material = device_hash + secret_token + user_hash
    key_bytes = _pbkdf2_derive(material, db_salt)
    return key_bytes.hex()


def derive_user_hash(username: str, password_hash: str) -> str:
    """
    Derive a stable user-specific hash from username + bcrypt password hash.
    Used as part of the user.db key derivation — changes when password changes
    (intentionally locks out user.db after password reset until re-keyed).
    """
    combined = f"{username}:{password_hash}"
    return hashlib.sha256(combined.encode("utf-8")).hexdigest()


# ──────────────────────────────────────────────────────────────────────────────
# Field-level AES-256-GCM encryption
# ──────────────────────────────────────────────────────────────────────────────

_NONCE_SIZE = 12  # 96-bit GCM nonce


def _get_field_key(field_encrypt_key_b64: str) -> bytes:
    """Decode the base64 field encryption key to 32 bytes."""
    key = base64.urlsafe_b64decode(field_encrypt_key_b64 + "==")
    if len(key) < 32:
        raise ValueError("FIELD_ENCRYPT_KEY must be at least 32 bytes (base64-encoded).")
    return key[:32]


def encrypt_field(plaintext: str, field_encrypt_key_b64: str) -> str:
    """
    Encrypt a sensitive field value with AES-256-GCM.
    Returns a base64-encoded string: nonce(12 bytes) + ciphertext + tag(16 bytes).
    Stores as TEXT in SQLite (prefix 'ENC:' marks encrypted columns).
    """
    if not plaintext:
        return plaintext

    key = _get_field_key(field_encrypt_key_b64)
    nonce = os.urandom(_NONCE_SIZE)
    aesgcm = AESGCM(key)
    ciphertext_with_tag = aesgcm.encrypt(nonce, plaintext.encode("utf-8"), None)
    blob = nonce + ciphertext_with_tag
    return "ENC:" + base64.urlsafe_b64encode(blob).decode("utf-8")


def decrypt_field(encrypted_value: str, field_encrypt_key_b64: str) -> str:
    """
    Decrypt a field value encrypted with encrypt_field().
    Returns the original plaintext. Returns value as-is if not prefixed with 'ENC:'.
    """
    if not encrypted_value or not encrypted_value.startswith("ENC:"):
        return encrypted_value

    key = _get_field_key(field_encrypt_key_b64)
    blob = base64.urlsafe_b64decode(encrypted_value[4:] + "==")
    nonce = blob[:_NONCE_SIZE]
    ciphertext_with_tag = blob[_NONCE_SIZE:]
    aesgcm = AESGCM(key)
    plaintext_bytes = aesgcm.decrypt(nonce, ciphertext_with_tag, None)
    return plaintext_bytes.decode("utf-8")


def is_encrypted(value: str) -> bool:
    """Return True if the value was encrypted with encrypt_field()."""
    return isinstance(value, str) and value.startswith("ENC:")


# ──────────────────────────────────────────────────────────────────────────────
# SQLCipher PRAGMA key format
# ──────────────────────────────────────────────────────────────────────────────

def sqlcipher_pragma_key(hex_key: str) -> str:
    """
    Return the PRAGMA key statement for SQLCipher using a raw hex key.
    Usage: conn.execute(sqlcipher_pragma_key(hex_key))
    """
    return f"PRAGMA key = \"x'{hex_key}'\";"


def sqlcipher_rekey(new_hex_key: str) -> str:
    """Return the PRAGMA rekey statement for rotating the SQLCipher key."""
    return f"PRAGMA rekey = \"x'{new_hex_key}'\";"


# ──────────────────────────────────────────────────────────────────────────────
# Encrypted fields registry
# ──────────────────────────────────────────────────────────────────────────────

ENCRYPTED_FIELDS: dict[str, list[str]] = {
    "employees": ["cnic", "passport_no", "bank_account", "bank_iban", "imei_1", "imei_2", "mac_address"],
    "customers": ["cnic", "bank_iban"],
    "vendors": ["cnic", "bank_iban"],
    "audit_log": ["old_values", "new_values"],
    "approved_devices": ["imei_1", "imei_2", "mac_address"],
}


def encrypt_record_fields(
    table: str,
    record: dict,
    field_encrypt_key_b64: str,
) -> dict:
    """Encrypt all sensitive fields in a record dict before DB write."""
    fields = ENCRYPTED_FIELDS.get(table, [])
    result = dict(record)
    for field in fields:
        if field in result and result[field] is not None:
            result[field] = encrypt_field(str(result[field]), field_encrypt_key_b64)
    return result


def decrypt_record_fields(
    table: str,
    record: dict,
    field_encrypt_key_b64: str,
) -> dict:
    """Decrypt all sensitive fields in a record dict after DB read."""
    fields = ENCRYPTED_FIELDS.get(table, [])
    result = dict(record)
    for field in fields:
        if field in result and result[field] is not None:
            result[field] = decrypt_field(str(result[field]), field_encrypt_key_b64)
    return result
