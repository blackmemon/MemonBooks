"""vendors_router.py — Vendors CRUD. Implemented in Phase 3."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_vendors(): return {"detail": "Phase 3 — not yet implemented."}
