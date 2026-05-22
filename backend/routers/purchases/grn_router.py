"""grn_router.py — GRN. Implemented in Phase 4."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_grn(): return {"detail": "Phase 4 — not yet implemented."}
