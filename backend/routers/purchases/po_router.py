"""po_router.py — Purchase Orders. Implemented in Phase 4."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_po(): return {"detail": "Phase 4 — not yet implemented."}
