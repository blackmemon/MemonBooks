"""employees_router.py — Employees. Implemented in Phase 7."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_employees(): return {"detail": "Phase 7 — not yet implemented."}
