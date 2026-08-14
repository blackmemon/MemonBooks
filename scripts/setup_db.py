#!/usr/bin/env python3
"""Run once after generate_keys.py (spec: Setup Sequence, step 2).

Creates the master.db and user.db schemas and seeds a default admin login
plus a minimal starter Chart of Accounts so the app isn't empty on first run.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from sqlalchemy import select

from database.database import (
    MasterBase,
    UserBase,
    get_master_engine,
    get_user_engine,
    get_user_session,
    open_user_database,
)

# Import every model module so its tables register on Master/UserBase metadata.
from backend.models import account_models, hr_models, inventory_models, system_models, transaction_models  # noqa: F401
from backend.models.account_models import Account
from backend.models.system_models import UserAccount
from security.auth import hash_password
from utils.numbering import generate_number

DEFAULT_ADMIN_USERNAME = "admin"
DEFAULT_ADMIN_PASSWORD = "admin123"

DEFAULT_ACCOUNTS = [
    ("Cash in Hand", "Asset"),
    ("Bank Accounts", "Asset"),
    ("Accounts Receivable", "Asset"),
    ("Inventory", "Asset"),
    ("Accounts Payable", "Liability"),
    ("Sales Revenue", "Revenue"),
    ("Purchase Expense", "Expense"),
]


def main() -> None:
    master_engine = get_master_engine()
    MasterBase.metadata.create_all(master_engine)
    print("master.db schema created.")

    open_user_database()
    UserBase.metadata.create_all(get_user_engine())
    print("user.db schema created.")

    with get_user_session() as db:
        admin = db.execute(
            select(UserAccount).where(UserAccount.username == DEFAULT_ADMIN_USERNAME)
        ).scalar_one_or_none()
        if admin is None:
            db.add(
                UserAccount(
                    username=DEFAULT_ADMIN_USERNAME,
                    full_name="Administrator",
                    password_hash=hash_password(DEFAULT_ADMIN_PASSWORD),
                    role="Admin",
                    is_active=True,
                )
            )
            print(f"Seeded default admin login: {DEFAULT_ADMIN_USERNAME} / {DEFAULT_ADMIN_PASSWORD}")
            print("Change this password after first login.")
        else:
            print("Admin login already exists — skipping.")

        existing_accounts = db.execute(select(Account.id)).first()
        if existing_accounts is None:
            for name, account_type in DEFAULT_ACCOUNTS:
                db.add(
                    Account(
                        account_code=generate_number("account"),
                        account_name=name,
                        account_type=account_type,
                        created_by="system",
                        updated_by="system",
                    )
                )
            print(f"Seeded {len(DEFAULT_ACCOUNTS)} default Chart of Accounts entries.")
        else:
            print("Chart of Accounts already has entries — skipping seed.")

    print("Setup complete.")


if __name__ == "__main__":
    main()
