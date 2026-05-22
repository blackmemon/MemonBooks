"""si_router.py — Sale Invoices. Implemented in Phase 5."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_si(): return {"detail": "Phase 5 — not yet implemented."}
