"""so_router.py — Sale Orders. Implemented in Phase 5."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_so(): return {"detail": "Phase 5 — not yet implemented."}
