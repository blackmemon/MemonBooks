"""pi_router.py — Purchase Invoices. Implemented in Phase 4."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_pi(): return {"detail": "Phase 4 — not yet implemented."}
