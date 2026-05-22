"""
main.py — MemonBook ERP FastAPI application entry point.
Registers all routers, handles startup/shutdown DB lifecycle,
and configures CORS for Tauri webview.
"""

import logging
from contextlib import asynccontextmanager
from pathlib import Path

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles

from backend.config import settings
from backend.database.connection import open_master_db, open_user_db, close_master_db, close_user_db

# Configure logging
logging.basicConfig(
    level=logging.DEBUG if settings.DEBUG else logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger("memonbook")


# ──────────────────────────────────────────────────────────────────────────────
# Lifespan — DB open/close
# ──────────────────────────────────────────────────────────────────────────────

@asynccontextmanager
async def lifespan(app: FastAPI):
    logger.info(f"Starting {settings.APP_NAME} v{settings.APP_VERSION}")

    # Open master.db
    try:
        open_master_db(settings.APP_SECRET, settings.DB_SALT)
        logger.info("master.db opened successfully.")
    except FileNotFoundError:
        logger.critical("master.db not found. Run scripts/setup_db.py first.")
        raise
    except RuntimeError as exc:
        logger.critical(f"master.db open failed: {exc}")
        raise

    yield

    # Shutdown
    close_master_db()
    close_user_db()
    logger.info("Databases closed. Shutdown complete.")


# ──────────────────────────────────────────────────────────────────────────────
# App instance
# ──────────────────────────────────────────────────────────────────────────────

app = FastAPI(
    title=settings.APP_NAME,
    version=settings.APP_VERSION,
    docs_url="/api/docs" if settings.DEBUG else None,
    redoc_url="/api/redoc" if settings.DEBUG else None,
    openapi_url="/api/openapi.json" if settings.DEBUG else None,
    lifespan=lifespan,
)

# CORS — allow Tauri webview
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.ALLOWED_ORIGINS,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# ──────────────────────────────────────────────────────────────────────────────
# Routers
# ──────────────────────────────────────────────────────────────────────────────

from backend.routers import (
    auth_router,
    device_router,
)

app.include_router(auth_router.router,   prefix="/api/auth",   tags=["Auth"])
app.include_router(device_router.router, prefix="/api/device", tags=["Device"])

# Masters routers
from backend.routers.masters import (
    coa_router,
    customers_router,
    vendors_router,
    items_router,
    masters_router,
)
app.include_router(coa_router.router,       prefix="/api/masters/coa",        tags=["COA"])
app.include_router(customers_router.router, prefix="/api/masters/customers",   tags=["Customers"])
app.include_router(vendors_router.router,   prefix="/api/masters/vendors",     tags=["Vendors"])
app.include_router(items_router.router,     prefix="/api/masters/items",       tags=["Items"])
app.include_router(masters_router.router,   prefix="/api/masters",             tags=["Masters"])

# Purchases routers
from backend.routers.purchases import (
    po_router,
    grn_router,
    pi_router,
)
app.include_router(po_router.router,  prefix="/api/purchases/po",  tags=["Purchase Orders"])
app.include_router(grn_router.router, prefix="/api/purchases/grn", tags=["GRN"])
app.include_router(pi_router.router,  prefix="/api/purchases/pi",  tags=["Purchase Invoices"])

# Sales routers
from backend.routers.sales import (
    so_router,
    dc_router,
    si_router,
)
app.include_router(so_router.router, prefix="/api/sales/so", tags=["Sale Orders"])
app.include_router(dc_router.router, prefix="/api/sales/dc", tags=["Delivery Challans"])
app.include_router(si_router.router, prefix="/api/sales/si", tags=["Sale Invoices"])

# Payments routers
from backend.routers.payments import (
    receipts_router,
    payments_router,
    jv_router,
)
app.include_router(receipts_router.router, prefix="/api/payments/receipts", tags=["Receipts"])
app.include_router(payments_router.router, prefix="/api/payments/payments", tags=["Payments"])
app.include_router(jv_router.router,       prefix="/api/accounts/jv",       tags=["Journal Vouchers"])

# Accounts routers
from backend.routers.accounts import (
    ledger_router,
)
app.include_router(ledger_router.router, prefix="/api/accounts", tags=["Accounts"])

# HR routers
from backend.routers.hr import (
    employees_router,
    attendance_router,
    payroll_router,
)
app.include_router(employees_router.router, prefix="/api/hr/employees",  tags=["Employees"])
app.include_router(attendance_router.router, prefix="/api/hr/attendance", tags=["Attendance"])
app.include_router(payroll_router.router,   prefix="/api/hr/payroll",    tags=["Payroll"])

# Reports router
from backend.routers.reports import reports_router
app.include_router(reports_router.router, prefix="/api/reports", tags=["Reports"])


# ──────────────────────────────────────────────────────────────────────────────
# Health check + device hash endpoint
# ──────────────────────────────────────────────────────────────────────────────

@app.get("/api/health", tags=["System"])
async def health_check():
    return {"status": "ok", "app": settings.APP_NAME, "version": settings.APP_VERSION}


@app.get("/api/device/hash", tags=["Device"])
async def get_device_hash_endpoint():
    from backend.security.device_fingerprint import get_device_hash
    return {"device_hash": get_device_hash()}


# ──────────────────────────────────────────────────────────────────────────────
# Run directly
# ──────────────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(
        "backend.main:app",
        host=settings.HOST,
        port=settings.PORT,
        reload=settings.DEBUG,
        log_level="debug" if settings.DEBUG else "info",
    )
