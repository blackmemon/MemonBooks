"""payroll_router.py — Payroll. Implemented in Phase 7."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_payroll(): return {"detail": "Phase 7 — not yet implemented."}
