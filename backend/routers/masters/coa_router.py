"""coa_router.py — Chart of Accounts CRUD. Implemented in Phase 3."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_coa(): return {"detail": "Phase 3 — not yet implemented."}
