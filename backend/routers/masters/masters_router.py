"""masters_router.py — UOM, Tax, Payment Terms, Currencies, Locations. Phase 3."""
from fastapi import APIRouter
router = APIRouter()

@router.get("/uom")
def list_uom(): return {"detail": "Phase 3 — not yet implemented."}

@router.get("/currencies")
def list_currencies(): return {"detail": "Phase 3 — not yet implemented."}

@router.get("/payment-terms")
def list_payment_terms(): return {"detail": "Phase 3 — not yet implemented."}
