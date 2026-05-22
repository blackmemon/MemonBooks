"""
device_fingerprint.py — Hardware fingerprint for Windows, Linux, macOS.
Returns a stable SHA-256 hash derived from immutable hardware identifiers.
Android/iOS fingerprints are handled in src-tauri/src/commands.rs.
"""

import hashlib
import platform
import subprocess
import re
import uuid
from pathlib import Path


def _run(cmd: list[str], timeout: int = 5) -> str:
    """Run a shell command and return stripped stdout. Empty string on failure."""
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return result.stdout.strip()
    except Exception:
        return ""


def _read_file(path: str) -> str:
    """Read a file and return stripped contents. Empty string on failure."""
    try:
        return Path(path).read_text(encoding="utf-8", errors="ignore").strip()
    except Exception:
        return ""


def _first_nonempty(*values: str) -> str:
    for v in values:
        stripped = (v or "").strip()
        if stripped and stripped not in {"", "unknown", "None", "null"}:
            return stripped
    return "UNKNOWN"


# ──────────────────────────────────────────────────────────────────────────────
# Linux
# ──────────────────────────────────────────────────────────────────────────────

def _linux_machine_id() -> str:
    return _first_nonempty(
        _read_file("/etc/machine-id"),
        _read_file("/var/lib/dbus/machine-id"),
    )


def _linux_cpu_serial() -> str:
    for line in _read_file("/proc/cpuinfo").splitlines():
        if line.lower().startswith("serial"):
            parts = line.split(":")
            if len(parts) == 2:
                return parts[1].strip()
    return "UNKNOWN"


def _linux_board_serial() -> str:
    return _first_nonempty(
        _read_file("/sys/class/dmi/id/board_serial"),
        _read_file("/sys/class/dmi/id/product_uuid"),
    )


def _linux_disk_serial() -> str:
    out = _run(["lsblk", "-ndo", "SERIAL", "/dev/sda"])
    if not out:
        out = _run(["lsblk", "-ndo", "SERIAL", "/dev/nvme0n1"])
    return out or "UNKNOWN"


def _linux_mac_address() -> str:
    for iface_path in sorted(Path("/sys/class/net").iterdir()):
        iface = iface_path.name
        if iface == "lo":
            continue
        mac = _read_file(str(iface_path / "address"))
        if mac and mac != "00:00:00:00:00:00":
            return mac
    return "UNKNOWN"


def _get_linux_fingerprint() -> str:
    components = [
        _linux_machine_id(),
        _linux_cpu_serial(),
        _linux_board_serial(),
        _linux_disk_serial(),
        _linux_mac_address(),
    ]
    return "|".join(components)


# ──────────────────────────────────────────────────────────────────────────────
# Windows
# ──────────────────────────────────────────────────────────────────────────────

def _wmic(alias: str, field: str) -> str:
    out = _run(["wmic", alias, "get", field, "/value"])
    for line in out.splitlines():
        if "=" in line:
            _, _, val = line.partition("=")
            v = val.strip()
            if v:
                return v
    return "UNKNOWN"


def _win_registry(key: str, value: str) -> str:
    out = _run(["reg", "query", key, "/v", value])
    for line in out.splitlines():
        if value in line:
            parts = line.split(None, 2)
            return parts[-1].strip() if len(parts) >= 3 else "UNKNOWN"
    return "UNKNOWN"


def _get_windows_fingerprint() -> str:
    cpu_id = _wmic("cpu", "ProcessorId")
    mb_serial = _wmic("baseboard", "SerialNumber")
    hdd_serial = _wmic("diskdrive", "SerialNumber")
    mac = _wmic("nic where PhysicalAdapter=True", "MACAddress").split("\n")[0]
    machine_guid = _win_registry(
        r"HKLM\SOFTWARE\Microsoft\Cryptography", "MachineGuid"
    )
    components = [cpu_id, mb_serial, hdd_serial, mac, machine_guid]
    return "|".join(components)


# ──────────────────────────────────────────────────────────────────────────────
# macOS
# ──────────────────────────────────────────────────────────────────────────────

def _ioreg(key: str) -> str:
    out = _run(["ioreg", "-rd1", "-c", "IOPlatformExpertDevice"])
    for line in out.splitlines():
        if key in line:
            m = re.search(r'"([^"]+)"$', line)
            if m:
                return m.group(1)
    return "UNKNOWN"


def _mac_system_profiler(data_type: str, field: str) -> str:
    out = _run(["system_profiler", data_type])
    for line in out.splitlines():
        if field.lower() in line.lower():
            _, _, val = line.partition(":")
            return val.strip()
    return "UNKNOWN"


def _mac_active_mac() -> str:
    out = _run(["ifconfig"])
    iface_mac = {}
    current = None
    for line in out.splitlines():
        if not line.startswith("\t") and not line.startswith(" "):
            current = line.split(":")[0]
        elif "ether " in line and current:
            iface_mac[current] = line.split("ether ")[1].split()[0]
    for preferred in ["en0", "en1"]:
        if preferred in iface_mac:
            return iface_mac[preferred]
    return next(iter(iface_mac.values()), "UNKNOWN")


def _get_macos_fingerprint() -> str:
    platform_uuid = _ioreg("IOPlatformUUID")
    platform_serial = _ioreg("IOPlatformSerialNumber")
    hardware_uuid = _mac_system_profiler("SPHardwareDataType", "Hardware UUID")
    mac = _mac_active_mac()
    components = [platform_uuid, platform_serial, hardware_uuid, mac]
    return "|".join(components)


# ──────────────────────────────────────────────────────────────────────────────
# Public API
# ──────────────────────────────────────────────────────────────────────────────

def get_device_fingerprint_raw() -> str:
    """Return the raw (unhashed) fingerprint string for the current OS."""
    system = platform.system()
    if system == "Linux":
        return _get_linux_fingerprint()
    elif system == "Windows":
        return _get_windows_fingerprint()
    elif system == "Darwin":
        return _get_macos_fingerprint()
    else:
        # Fallback: use Python's uuid based on MAC
        return f"FALLBACK|{uuid.getnode()}"


def get_device_hash() -> str:
    """
    Return a stable, 64-char SHA-256 hex digest of the hardware fingerprint.
    This hash is used as input to the PBKDF2 key derivation for both databases.
    """
    raw = get_device_fingerprint_raw()
    return hashlib.sha256(raw.encode("utf-8")).hexdigest()


def verify_device_hash(expected_hash: str) -> bool:
    """Check whether the current device matches a stored hash."""
    return get_device_hash() == expected_hash


if __name__ == "__main__":
    raw = get_device_fingerprint_raw()
    digest = get_device_hash()
    print(f"OS       : {platform.system()}")
    print(f"Raw      : {raw}")
    print(f"SHA-256  : {digest}")
