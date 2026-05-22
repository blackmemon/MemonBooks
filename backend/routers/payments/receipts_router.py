"""receipts_router.py — Cash/Bank Receipts. Implemented in Phase 6."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_receipts(): return {"detail": "Phase 6 — not yet implemented."}
