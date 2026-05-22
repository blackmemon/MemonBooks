"""jv_router.py — Journal Vouchers. Implemented in Phase 6."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_jv(): return {"detail": "Phase 6 — not yet implemented."}
