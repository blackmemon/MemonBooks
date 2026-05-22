"""items_router.py — Items CRUD. Implemented in Phase 3."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_items(): return {"detail": "Phase 3 — not yet implemented."}
