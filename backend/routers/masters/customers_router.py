"""customers_router.py — Customers CRUD. Implemented in Phase 3."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_customers(): return {"detail": "Phase 3 — not yet implemented."}
