"""ledger_router.py — Ledger + Trial Balance. Implemented in Phase 6."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/ledger")
def ledger(): return {"detail": "Phase 6 — not yet implemented."}

@router.get("/trial-balance")
def trial_balance(): return {"detail": "Phase 6 — not yet implemented."}
