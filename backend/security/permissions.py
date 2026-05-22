"""
permissions.py — Role-based access control (RBAC) for MemonBook ERP.

Roles (hierarchy): admin > manager > user > viewer
Modules: masters, purchases, sales, payments, accounts, hr, reports, system

Actions: view, create, edit, delete, approve, post, export

Permission check flow on every API request:
  1. JWT decoded → role extracted
  2. Module + action checked against role permission matrix
  3. Custom overrides checked from user_permissions table in master.db
  4. Device hash re-verified (done in auth.py)
"""

from enum import Enum
from functools import lru_cache
from typing import Callable

from fastapi import Depends, HTTPException, status

from backend.security.auth import get_current_user


# ──────────────────────────────────────────────────────────────────────────────
# Enums
# ──────────────────────────────────────────────────────────────────────────────

class Role(str, Enum):
    ADMIN = "admin"
    MANAGER = "manager"
    USER = "user"
    VIEWER = "viewer"


class Module(str, Enum):
    MASTERS = "masters"
    PURCHASES = "purchases"
    SALES = "sales"
    PAYMENTS = "payments"
    ACCOUNTS = "accounts"
    HR = "hr"
    REPORTS = "reports"
    SYSTEM = "system"


class Action(str, Enum):
    VIEW = "view"
    CREATE = "create"
    EDIT = "edit"
    DELETE = "delete"
    APPROVE = "approve"
    POST = "post"
    EXPORT = "export"


# ──────────────────────────────────────────────────────────────────────────────
# Default permission matrix
# ──────────────────────────────────────────────────────────────────────────────
# Format: { Role: { Module: set[Action] } }

_DEFAULT_PERMISSIONS: dict[Role, dict[Module, set[Action]]] = {
    Role.ADMIN: {
        # Admin has all permissions on all modules
        module: {action for action in Action}
        for module in Module
    },
    Role.MANAGER: {
        Module.MASTERS:    {Action.VIEW, Action.CREATE, Action.EDIT},
        Module.PURCHASES:  {Action.VIEW, Action.CREATE, Action.EDIT, Action.APPROVE, Action.POST, Action.EXPORT},
        Module.SALES:      {Action.VIEW, Action.CREATE, Action.EDIT, Action.APPROVE, Action.POST, Action.EXPORT},
        Module.PAYMENTS:   {Action.VIEW, Action.CREATE, Action.EDIT, Action.APPROVE, Action.POST, Action.EXPORT},
        Module.ACCOUNTS:   {Action.VIEW, Action.CREATE, Action.EDIT, Action.POST, Action.EXPORT},
        Module.HR:         {Action.VIEW, Action.CREATE, Action.EDIT, Action.APPROVE, Action.EXPORT},
        Module.REPORTS:    {Action.VIEW, Action.EXPORT},
        Module.SYSTEM:     set(),
    },
    Role.USER: {
        Module.MASTERS:    {Action.VIEW},
        Module.PURCHASES:  {Action.VIEW, Action.CREATE, Action.EDIT},
        Module.SALES:      {Action.VIEW, Action.CREATE, Action.EDIT},
        Module.PAYMENTS:   {Action.VIEW, Action.CREATE},
        Module.ACCOUNTS:   {Action.VIEW},
        Module.HR:         {Action.VIEW},
        Module.REPORTS:    {Action.VIEW},
        Module.SYSTEM:     set(),
    },
    Role.VIEWER: {
        Module.MASTERS:    {Action.VIEW},
        Module.PURCHASES:  {Action.VIEW},
        Module.SALES:      {Action.VIEW},
        Module.PAYMENTS:   {Action.VIEW},
        Module.ACCOUNTS:   {Action.VIEW},
        Module.HR:         set(),
        Module.REPORTS:    {Action.VIEW},
        Module.SYSTEM:     set(),
    },
}


# ──────────────────────────────────────────────────────────────────────────────
# Permission checker
# ──────────────────────────────────────────────────────────────────────────────

def has_permission(role: str, module: str, action: str) -> bool:
    """Check default permission matrix without DB lookup (fast path)."""
    try:
        role_enum = Role(role)
        module_enum = Module(module)
        action_enum = Action(action)
    except ValueError:
        return False

    module_perms = _DEFAULT_PERMISSIONS.get(role_enum, {})
    allowed_actions = module_perms.get(module_enum, set())
    return action_enum in allowed_actions


def check_permission_or_raise(
    role: str,
    module: str,
    action: str,
) -> None:
    """Raise HTTP 403 if the role does not have the required permission."""
    if not has_permission(role, module, action):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail=f"Permission denied: '{role}' cannot '{action}' in module '{module}'.",
        )


def check_permission_with_overrides(
    role: str,
    user_id: str,
    module: str,
    action: str,
    conn,
) -> None:
    """
    Check permission with optional per-user overrides from master.db.
    User override can GRANT extra permissions or REVOKE default ones.
    """
    # Check per-user grant override first
    override = conn.execute(
        """
        SELECT is_granted FROM user_permissions
        WHERE user_id = ? AND module = ? AND action = ?
        """,
        (user_id, module, action),
    ).fetchone()

    if override is not None:
        if override["is_granted"]:
            return  # explicitly granted
        else:
            raise HTTPException(
                status_code=status.HTTP_403_FORBIDDEN,
                detail=f"Permission explicitly revoked for user in module '{module}' action '{action}'.",
            )

    # Fall back to role-based matrix
    check_permission_or_raise(role, module, action)


# ──────────────────────────────────────────────────────────────────────────────
# FastAPI dependency factories
# ──────────────────────────────────────────────────────────────────────────────

def require_permission(module: str, action: str) -> Callable:
    """
    FastAPI dependency factory for declarative permission checking.

    Usage:
        @router.get("/purchase-orders", dependencies=[Depends(require_permission("purchases", "view"))])
        async def list_pos(): ...
    """
    async def _check(current_user: dict = Depends(get_current_user)) -> dict:
        role = current_user.get("role", "")
        check_permission_or_raise(role, module, action)
        return current_user
    return _check


def require_permission_with_db(module: str, action: str) -> Callable:
    """
    FastAPI dependency factory for permission check with DB override support.
    Slightly heavier than require_permission() — only use when per-user overrides matter.
    """
    from backend.database.connection import get_master_db

    async def _check(
        current_user: dict = Depends(get_current_user),
        conn=Depends(get_master_db),
    ) -> dict:
        role = current_user.get("role", "")
        user_id = current_user.get("sub", "")
        check_permission_with_overrides(role, user_id, module, action, conn)
        return current_user

    return _check


# ──────────────────────────────────────────────────────────────────────────────
# Role hierarchy utilities
# ──────────────────────────────────────────────────────────────────────────────

_ROLE_HIERARCHY = [Role.VIEWER, Role.USER, Role.MANAGER, Role.ADMIN]


def role_rank(role: str) -> int:
    """Return numeric rank of a role (higher = more privileged)."""
    try:
        return _ROLE_HIERARCHY.index(Role(role))
    except ValueError:
        return -1


def is_role_at_least(role: str, minimum_role: str) -> bool:
    """Return True if role is equal to or above minimum_role in hierarchy."""
    return role_rank(role) >= role_rank(minimum_role)


def get_all_permissions_for_role(role: str) -> dict[str, list[str]]:
    """Return a serializable summary of all permissions for a given role."""
    try:
        role_enum = Role(role)
    except ValueError:
        return {}

    result = {}
    role_perms = _DEFAULT_PERMISSIONS.get(role_enum, {})
    for module, actions in role_perms.items():
        result[module.value] = sorted(a.value for a in actions)
    return result
