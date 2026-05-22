"""payments_router.py — Cash/Bank Payments. Implemented in Phase 6."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_payments(): return {"detail": "Phase 6 — not yet implemented."}
