"""dc_router.py — Delivery Challans. Implemented in Phase 5."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_dc(): return {"detail": "Phase 5 — not yet implemented."}
