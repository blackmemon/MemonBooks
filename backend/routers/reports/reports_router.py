"""reports_router.py — Reports. Implemented in Phase 8."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_reports(): return {"detail": "Phase 8 — not yet implemented."}
