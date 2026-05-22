"""attendance_router.py — Attendance. Implemented in Phase 7."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/")
def list_attendance(): return {"detail": "Phase 7 — not yet implemented."}
