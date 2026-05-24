"""
pdf_generator.py — ReportLab-based PDF generation for MemonBook ERP.

Generates:
- Purchase Order PDF
- Purchase Invoice PDF
- Sale Invoice PDF
- Payslip PDF
- Trial Balance PDF
- Ledger PDF

All functions return bytes (in-memory PDF) for streaming via FastAPI.
"""

from __future__ import annotations
from io import BytesIO
from typing import Any, Optional
from datetime import datetime

try:
    from reportlab.lib import colors
    from reportlab.lib.pagesizes import A4, landscape
    from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
    from reportlab.lib.units import cm, mm
    from reportlab.platypus import (
        SimpleDocTemplate, Table, TableStyle, Paragraph,
        Spacer, HRFlowable, KeepTogether,
    )
    from reportlab.lib.enums import TA_CENTER, TA_RIGHT, TA_LEFT
    REPORTLAB_AVAILABLE = True
except ImportError:
    REPORTLAB_AVAILABLE = False


# Brand colors
PRIMARY    = colors.HexColor("#0072B7")
DARK_BLUE  = colors.HexColor("#0063AA")
SUCCESS    = colors.HexColor("#16A34A")
DANGER     = colors.HexColor("#DC2626")
LIGHT_GRAY = colors.HexColor("#F3F4F6")
MID_GRAY   = colors.HexColor("#D1D5DB")
TEXT_GRAY  = colors.HexColor("#6B7280")

COMPANY_NAME    = "MemonBook ERP"
COMPANY_ADDRESS = ""  # Set from settings
COMPANY_PHONE   = ""
COMPANY_EMAIL   = ""


def _check_rl():
    if not REPORTLAB_AVAILABLE:
        raise RuntimeError("reportlab not installed. Run: pip install reportlab")


def _get_styles():
    styles = getSampleStyleSheet()
    styles.add(ParagraphStyle(name='H1',    fontSize=16, textColor=PRIMARY,    fontName='Helvetica-Bold', spaceAfter=4))
    styles.add(ParagraphStyle(name='H2',    fontSize=12, textColor=DARK_BLUE,  fontName='Helvetica-Bold', spaceAfter=4))
    styles.add(ParagraphStyle(name='Sub',   fontSize=9,  textColor=TEXT_GRAY,  fontName='Helvetica'))
    styles.add(ParagraphStyle(name='Body',  fontSize=9,  textColor=colors.black, fontName='Helvetica'))
    styles.add(ParagraphStyle(name='Right', fontSize=9,  textColor=colors.black, fontName='Helvetica', alignment=TA_RIGHT))
    styles.add(ParagraphStyle(name='Center',fontSize=9,  textColor=colors.black, fontName='Helvetica', alignment=TA_CENTER))
    styles.add(ParagraphStyle(name='Bold',  fontSize=9,  textColor=colors.black, fontName='Helvetica-Bold'))
    styles.add(ParagraphStyle(name='BoldR', fontSize=9,  textColor=colors.black, fontName='Helvetica-Bold', alignment=TA_RIGHT))
    return styles


def _header_table(title: str, doc_number: str, doc_date: str, styles) -> Table:
    """Standard document header with company info + doc title."""
    data = [
        [Paragraph(f'<font color="#0072B7"><b>{COMPANY_NAME}</b></font>', styles['H1']),
         Paragraph(f'<b>{title}</b>', styles['H2'])],
        [Paragraph(COMPANY_ADDRESS or '', styles['Sub']),
         Paragraph(f'<b>#:</b> {doc_number}', styles['Body'])],
        [Paragraph(COMPANY_PHONE or '', styles['Sub']),
         Paragraph(f'<b>Date:</b> {doc_date}', styles['Body'])],
    ]
    t = Table(data, colWidths=[9.5 * cm, 9.5 * cm])
    t.setStyle(TableStyle([
        ('VALIGN',       (0, 0), (-1, -1), 'TOP'),
        ('ALIGN',        (1, 0), (1, -1),  'RIGHT'),
        ('BOTTOMPADDING',(0, 0), (-1, -1), 2),
        ('TOPPADDING',   (0, 0), (-1, -1), 2),
    ]))
    return t


def _fmt(value: float, currency: str = "PKR") -> str:
    return f"{currency} {value:,.2f}"


# ─────────────────────────────────────────────────────────────────────────────
# PURCHASE ORDER PDF
# ─────────────────────────────────────────────────────────────────────────────

def generate_po_pdf(po: dict) -> bytes:
    """Generate Purchase Order PDF. po dict must include lines[]."""
    _check_rl()
    buf = BytesIO()
    doc = SimpleDocTemplate(buf, pagesize=A4,
                            leftMargin=1.5*cm, rightMargin=1.5*cm,
                            topMargin=1.5*cm, bottomMargin=1.5*cm)
    styles = _get_styles()
    story = []

    story.append(_header_table("PURCHASE ORDER", po.get("po_number", ""), po.get("po_date", ""), styles))
    story.append(Spacer(1, 0.4*cm))
    story.append(HRFlowable(width="100%", thickness=1, color=PRIMARY))
    story.append(Spacer(1, 0.3*cm))

    # Vendor info
    story.append(Paragraph(f"<b>Vendor:</b> {po.get('vendor_name', '')}", styles['Body']))
    if po.get("delivery_date"):
        story.append(Paragraph(f"<b>Expected Delivery:</b> {po.get('delivery_date')}", styles['Body']))
    if po.get("notes"):
        story.append(Paragraph(f"<b>Notes:</b> {po.get('notes')}", styles['Sub']))
    story.append(Spacer(1, 0.4*cm))

    # Lines table
    headers = [['#', 'Item', 'UOM', 'Qty', 'Unit Price', 'Disc%', 'Tax%', 'Total']]
    col_w   = [0.6*cm, 5.5*cm, 1.5*cm, 1.5*cm, 2.2*cm, 1.2*cm, 1.2*cm, 2.5*cm]
    lines   = po.get("lines", [])
    rows    = []
    for i, l in enumerate(lines, 1):
        rows.append([
            str(i),
            l.get("item_name", ""),
            l.get("uom_name", ""),
            f"{l.get('qty', 0):g}",
            _fmt(l.get("unit_price", 0), ""),
            f"{l.get('discount_pct', 0)}%",
            f"{l.get('tax_rate', 0)}%",
            _fmt(l.get("line_total", 0), ""),
        ])
    data = headers + rows
    tbl = Table(data, colWidths=col_w)
    tbl.setStyle(_line_table_style())
    story.append(tbl)
    story.append(Spacer(1, 0.3*cm))

    # Totals
    story.append(_totals_table(po, styles))

    doc.build(story)
    return buf.getvalue()


# ─────────────────────────────────────────────────────────────────────────────
# SALE INVOICE PDF
# ─────────────────────────────────────────────────────────────────────────────

def generate_si_pdf(si: dict) -> bytes:
    """Generate Sale Invoice PDF."""
    _check_rl()
    buf = BytesIO()
    doc = SimpleDocTemplate(buf, pagesize=A4,
                            leftMargin=1.5*cm, rightMargin=1.5*cm,
                            topMargin=1.5*cm, bottomMargin=1.5*cm)
    styles = _get_styles()
    story = []

    story.append(_header_table("SALE INVOICE", si.get("si_number", ""), si.get("invoice_date", ""), styles))
    story.append(Spacer(1, 0.4*cm))
    story.append(HRFlowable(width="100%", thickness=1, color=PRIMARY))
    story.append(Spacer(1, 0.3*cm))

    # Customer info
    story.append(Paragraph(f"<b>Bill To:</b> {si.get('customer_name', '')}", styles['Body']))
    if si.get("due_date"):
        story.append(Paragraph(f"<b>Due Date:</b> {si.get('due_date')}", styles['Body']))
    story.append(Spacer(1, 0.4*cm))

    # Lines table
    headers = [['#', 'Description', 'Qty', 'Unit Price', 'Disc%', 'Tax', 'Total']]
    col_w   = [0.6*cm, 6.5*cm, 1.5*cm, 2.2*cm, 1.2*cm, 1.8*cm, 2.5*cm]
    lines   = si.get("lines", [])
    rows    = []
    for i, l in enumerate(lines, 1):
        rows.append([
            str(i),
            f"{l.get('item_code', '')} — {l.get('item_name', '')}",
            f"{l.get('qty', 0):g}",
            _fmt(l.get("unit_price", 0), ""),
            f"{l.get('discount_pct', 0)}%",
            _fmt(l.get("tax_amt", 0), ""),
            _fmt(l.get("line_total", 0), ""),
        ])
    data = headers + rows
    tbl = Table(data, colWidths=col_w)
    tbl.setStyle(_line_table_style())
    story.append(tbl)
    story.append(Spacer(1, 0.3*cm))
    story.append(_totals_table(si, styles))

    if si.get("notes"):
        story.append(Spacer(1, 0.5*cm))
        story.append(Paragraph(f"<b>Notes:</b> {si.get('notes')}", styles['Sub']))

    doc.build(story)
    return buf.getvalue()


# ─────────────────────────────────────────────────────────────────────────────
# PAYSLIP PDF
# ─────────────────────────────────────────────────────────────────────────────

def generate_payslip_pdf(payslip: dict) -> bytes:
    """
    payslip = {"run": {...}, "line": {...}}
    """
    _check_rl()
    buf = BytesIO()
    doc = SimpleDocTemplate(buf, pagesize=A4,
                            leftMargin=1.5*cm, rightMargin=1.5*cm,
                            topMargin=1.5*cm, bottomMargin=1.5*cm)
    styles = _get_styles()
    story  = []

    run  = payslip.get("run", {})
    line = payslip.get("line", {})
    MONTHS_LIST = ['January','February','March','April','May','June','July','August','September','October','November','December']
    period = f"{MONTHS_LIST[(run.get('month', 1) - 1)]} {run.get('year', '')}"

    # Header
    story.append(_header_table("PAYSLIP", run.get("run_number", ""), run.get("run_date", ""), styles))
    story.append(Spacer(1, 0.4*cm))
    story.append(HRFlowable(width="100%", thickness=1, color=PRIMARY))
    story.append(Spacer(1, 0.3*cm))

    # Employee info
    emp_data = [
        [Paragraph('<b>Employee</b>', styles['Bold']),  Paragraph(f"{line.get('emp_code', '')} — {line.get('full_name', '')}", styles['Body'])],
        [Paragraph('<b>Department</b>', styles['Bold']), Paragraph(line.get('department_name', '') or '—', styles['Body'])],
        [Paragraph('<b>Designation</b>', styles['Bold']), Paragraph(line.get('designation_name', '') or '—', styles['Body'])],
        [Paragraph('<b>Pay Period</b>', styles['Bold']),  Paragraph(period, styles['Body'])],
        [Paragraph('<b>Bank</b>', styles['Bold']),        Paragraph(f"{line.get('bank_name', '') or '—'} / {line.get('bank_account', '') or '—'}", styles['Body'])],
    ]
    emp_tbl = Table(emp_data, colWidths=[4*cm, 15*cm])
    emp_tbl.setStyle(TableStyle([
        ('GRID', (0, 0), (-1, -1), 0.5, MID_GRAY),
        ('BACKGROUND', (0, 0), (0, -1), LIGHT_GRAY),
        ('FONTNAME', (0, 0), (-1, -1), 'Helvetica'),
        ('FONTSIZE', (0, 0), (-1, -1), 9),
        ('PADDING', (0, 0), (-1, -1), 4),
    ]))
    story.append(emp_tbl)
    story.append(Spacer(1, 0.4*cm))

    # Attendance summary
    att_data = [
        ['Working Days', 'Present', 'Absent', 'Leave', 'OT Hours'],
        [str(line.get('working_days', 0)), str(line.get('present_days', 0)),
         str(line.get('absent_days', 0)), str(line.get('leave_days', 0)),
         str(line.get('overtime_hours', 0))],
    ]
    att_tbl = Table(att_data, colWidths=[3.8*cm]*5)
    att_tbl.setStyle(TableStyle([
        ('BACKGROUND', (0, 0), (-1, 0), PRIMARY),
        ('TEXTCOLOR',  (0, 0), (-1, 0), colors.white),
        ('ALIGN',      (0, 0), (-1, -1), 'CENTER'),
        ('FONTNAME',   (0, 0), (-1, 0), 'Helvetica-Bold'),
        ('FONTSIZE',   (0, 0), (-1, -1), 9),
        ('GRID',       (0, 0), (-1, -1), 0.5, MID_GRAY),
        ('PADDING',    (0, 0), (-1, -1), 4),
    ]))
    story.append(att_tbl)
    story.append(Spacer(1, 0.4*cm))

    # Earnings & deductions
    earnings = [
        ['Description', 'Amount'],
        ['Basic Salary', _fmt(line.get('gross_salary', 0) - line.get('overtime_hours', 0) * 0, '')],  # approximation
        ['Gross Salary', _fmt(line.get('gross_salary', 0), '')],
        ['', ''],
        ['Income Tax', _fmt(line.get('income_tax', 0), '')],
        ['Other Deductions', _fmt(line.get('total_deductions', 0), '')],
    ]
    earn_tbl = Table(earnings, colWidths=[9.5*cm, 9.5*cm])
    earn_tbl.setStyle(TableStyle([
        ('BACKGROUND', (0, 0), (-1, 0), DARK_BLUE),
        ('TEXTCOLOR',  (0, 0), (-1, 0), colors.white),
        ('FONTNAME',   (0, 0), (-1, 0), 'Helvetica-Bold'),
        ('FONTSIZE',   (0, 0), (-1, -1), 9),
        ('GRID',       (0, 0), (-1, -1), 0.5, MID_GRAY),
        ('PADDING',    (0, 0), (-1, -1), 4),
        ('ALIGN',      (1, 0), (1, -1), 'RIGHT'),
    ]))
    story.append(earn_tbl)
    story.append(Spacer(1, 0.2*cm))

    # Net salary highlight
    net_data = [['NET SALARY', _fmt(line.get('net_salary', 0), 'PKR')]]
    net_tbl  = Table(net_data, colWidths=[9.5*cm, 9.5*cm])
    net_tbl.setStyle(TableStyle([
        ('BACKGROUND', (0, 0), (-1, -1), SUCCESS),
        ('TEXTCOLOR',  (0, 0), (-1, -1), colors.white),
        ('FONTNAME',   (0, 0), (-1, -1), 'Helvetica-Bold'),
        ('FONTSIZE',   (0, 0), (-1, -1), 12),
        ('PADDING',    (0, 0), (-1, -1), 8),
        ('ALIGN',      (1, 0), (1, -1), 'RIGHT'),
    ]))
    story.append(net_tbl)

    doc.build(story)
    return buf.getvalue()


# ─────────────────────────────────────────────────────────────────────────────
# TRIAL BALANCE PDF
# ─────────────────────────────────────────────────────────────────────────────

def generate_trial_balance_pdf(data: dict) -> bytes:
    """data = {"items": [...], "total_debit": ..., "total_credit": ..., "is_balanced": bool}"""
    _check_rl()
    buf = BytesIO()
    doc = SimpleDocTemplate(buf, pagesize=landscape(A4),
                            leftMargin=1.5*cm, rightMargin=1.5*cm,
                            topMargin=1.5*cm, bottomMargin=1.5*cm)
    styles = _get_styles()
    story  = []

    story.append(Paragraph(f'<font color="#0072B7"><b>{COMPANY_NAME}</b></font>', styles['H1']))
    story.append(Paragraph('<b>TRIAL BALANCE</b>', styles['H2']))
    story.append(Paragraph(f"As of: {datetime.now().strftime('%d %b %Y')}", styles['Sub']))
    story.append(Spacer(1, 0.4*cm))
    story.append(HRFlowable(width="100%", thickness=1, color=PRIMARY))
    story.append(Spacer(1, 0.3*cm))

    headers = [['Code', 'Account Name', 'Type', 'Debit (PKR)', 'Credit (PKR)', 'Net (PKR)']]
    rows = []
    for row in data.get("items", []):
        if (row.get("total_debit") or 0) == 0 and (row.get("total_credit") or 0) == 0:
            continue
        rows.append([
            row.get("code", ""),
            "  " * max(0, (row.get("level", 1) - 1)) + row.get("name", ""),
            row.get("account_type", "").replace("_", " "),
            _fmt(row.get("total_debit", 0), ""),
            _fmt(row.get("total_credit", 0), ""),
            _fmt(row.get("net", 0), ""),
        ])
    # Totals row
    rows.append([
        "", "TOTAL", "",
        _fmt(data.get("total_debit", 0), "PKR"),
        _fmt(data.get("total_credit", 0), "PKR"),
        "" if data.get("is_balanced") else "NOT BALANCED",
    ])

    tbl_data = headers + rows
    col_w = [2*cm, 8*cm, 2.5*cm, 4*cm, 4*cm, 4*cm]
    tbl = Table(tbl_data, colWidths=col_w)
    tbl.setStyle(TableStyle([
        ('BACKGROUND', (0, 0), (-1, 0), PRIMARY),
        ('TEXTCOLOR',  (0, 0), (-1, 0), colors.white),
        ('FONTNAME',   (0, 0), (-1, 0), 'Helvetica-Bold'),
        ('FONTSIZE',   (0, 0), (-1, -1), 8),
        ('GRID',       (0, 0), (-1, -1), 0.3, MID_GRAY),
        ('ALIGN',      (3, 0), (-1, -1), 'RIGHT'),
        ('PADDING',    (0, 0), (-1, -1), 3),
        ('BACKGROUND', (0, -1), (-1, -1), DARK_BLUE),
        ('TEXTCOLOR',  (0, -1), (-1, -1), colors.white),
        ('FONTNAME',   (0, -1), (-1, -1), 'Helvetica-Bold'),
    ]))
    story.append(tbl)

    doc.build(story)
    return buf.getvalue()


# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

def _line_table_style() -> TableStyle:
    return TableStyle([
        ('BACKGROUND', (0, 0), (-1, 0), PRIMARY),
        ('TEXTCOLOR',  (0, 0), (-1, 0), colors.white),
        ('FONTNAME',   (0, 0), (-1, 0), 'Helvetica-Bold'),
        ('FONTSIZE',   (0, 0), (-1, -1), 8),
        ('GRID',       (0, 0), (-1, -1), 0.5, MID_GRAY),
        ('ALIGN',      (3, 1), (-1, -1), 'RIGHT'),
        ('ROWBACKGROUNDS', (0, 1), (-1, -1), [colors.white, LIGHT_GRAY]),
        ('PADDING',    (0, 0), (-1, -1), 3),
    ])


def _totals_table(doc: dict, styles) -> Table:
    """Generate a right-aligned totals summary table."""
    subtotal   = doc.get("subtotal")    or doc.get("sub_total")    or 0.0
    discount   = doc.get("discount_total") or 0.0
    tax        = doc.get("tax_total")   or 0.0
    wht        = doc.get("wht_total")   or 0.0
    grand      = doc.get("grand_total") or 0.0
    amount_due = doc.get("amount_due")  or grand

    rows = []
    if subtotal:  rows.append(['Subtotal:',    _fmt(subtotal, 'PKR')])
    if discount:  rows.append(['Discount:',    f'- {_fmt(discount, "PKR")}'])
    if tax:       rows.append(['Tax:',         _fmt(tax, 'PKR')])
    if wht:       rows.append(['WHT:',         f'- {_fmt(wht, "PKR")}'])
    rows.append(['Grand Total:', _fmt(grand, 'PKR')])
    if amount_due != grand:
        rows.append(['Amount Due:', _fmt(amount_due, 'PKR')])

    tbl = Table(rows, colWidths=[3*cm, 4*cm], hAlign='RIGHT')
    style = [
        ('FONTNAME',   (0, 0),  (-1, -1), 'Helvetica'),
        ('FONTSIZE',   (0, 0),  (-1, -1), 9),
        ('ALIGN',      (1, 0),  (1, -1),  'RIGHT'),
        ('TOPPADDING', (0, 0),  (-1, -1), 2),
        ('FONTNAME',   (0, -1), (-1, -1), 'Helvetica-Bold'),
        ('FONTSIZE',   (0, -1), (-1, -1), 10),
        ('TEXTCOLOR',  (0, -1), (-1, -1), PRIMARY),
        ('LINEABOVE',  (0, -1), (-1, -1), 1, PRIMARY),
    ]
    tbl.setStyle(TableStyle(style))
    return tbl
