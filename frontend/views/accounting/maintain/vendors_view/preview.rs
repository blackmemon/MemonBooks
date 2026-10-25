//! The vendor's full-page preview (opened by the Preview button): a
//! profile column on the left (avatar, quick actions, collapsible contact,
//! payment, notes, custom-field and attachment sections) and a tabbed
//! detail column on the right (overview, transactions, ledger, trial
//! balance, attachments). Both scroll on their own; below the narrow
//! breakpoint they stack. It shows what the form holds right now plus the
//! vendor's read-only ledger; nothing here changes data.

use super::{
    dash, day, money, Ctx, Message, PurchaseService, State, VendorAttachment, VendorLedger,
    COL_ATTACHMENTS, COL_COMPANY_NAME, COL_CREATE_TIMESTAMP, COL_CUSTOM_FIELDS, COL_DESCRIPTION,
    COL_EMAIL, COL_MAILING_ADDRESS, COL_MODIFIED_TIMESTAMP, COL_PAYMENT_METHOD, COL_PAYMENT_TERMS,
    COL_PHONE, COL_SHIPPING_ADDRESS, COL_STATUS, COL_VENDOR_ID, COL_VENDOR_NAME, COL_WEBSITE,
    TITLE, VENDOR_HEADERS, VENDOR_TYPE_LABEL,
};
use crate::backend::accounting::vendors::side;
use crate::backend::purchases::vendor_ledger;
use crate::frontend::components::badges::{self, BadgeTone};
use crate::frontend::components::colors::Theme;
use crate::frontend::components::panels::TimelineEntry;
use crate::frontend::components::table as table_style;
use crate::frontend::components::{
    button as button_style, forms, icons, labels, panels, spacing, tabs, typography,
};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};
use rust_decimal::Decimal;

/// `(icon, label)` of each tab, in order; the `TAB_*` indexes point into it.
pub(super) const PREVIEW_TABS: [(&str, &str); 5] = [
    (icons::TAB_OVERVIEW, "Overview"),
    (icons::TAB_TRANSACTIONS, "Transactions"),
    (icons::TAB_LEDGER, "Ledger"),
    (icons::TAB_TRIAL_BALANCE, "Trial Balance"),
    (icons::TAB_ATTACHMENTS, VENDOR_HEADERS[COL_ATTACHMENTS]),
];
const TAB_OVERVIEW: usize = 0;
const TAB_TRANSACTIONS: usize = 1;
const TAB_LEDGER: usize = 2;
const TAB_ATTACHMENTS: usize = 4;

/// The collapsible sections of the profile column.
pub(super) const SECTION_CONTACT: usize = 0;
pub(super) const SECTION_PAYMENT: usize = 1;
pub(super) const SECTION_NOTES: usize = 2;
pub(super) const SECTION_CUSTOM: usize = 3;
pub(super) const SECTION_ATTACHMENTS: usize = 4;
/// How many of the latest ledger entries the overview's activity lists.
const ACTIVITY_LIMIT: usize = 5;
/// Shown for a value that is not there.
const NONE: &str = "--";

/// Label/value lines, labels taken from `VENDOR_HEADERS` where the column
/// exists there.
type Lines = Vec<(String, String)>;

fn line(label: &str, value: String) -> (String, String) {
    (label.to_string(), value)
}

/// What the preview identifies the vendor by.
struct Profile {
    name: String,
    company: String,
    vendor_id: String,
    status: String,
}

fn profile(state: &State) -> Profile {
    let form = &state.form;
    let saved = state
        .editing_id
        .and_then(|id| state.vendors.iter().find(|vendor| vendor.id == Some(id)));
    Profile {
        name: if form.name.is_empty() {
            "New Vendor".into()
        } else {
            form.name.clone()
        },
        company: dash(&form.company_name),
        vendor_id: saved
            .and_then(|vendor| vendor.reference_code.clone())
            .unwrap_or_else(|| "Assigned on save".into()),
        status: if form.status.is_empty() {
            "Active".into()
        } else {
            form.status.clone()
        },
    }
}

fn contact_lines(state: &State) -> Lines {
    let form = &state.form;
    vec![
        line(VENDOR_HEADERS[COL_EMAIL], dash(&form.email)),
        line(VENDOR_HEADERS[COL_PHONE], dash(&form.phone)),
        line(VENDOR_HEADERS[COL_WEBSITE], dash(&form.website)),
        line(
            VENDOR_HEADERS[COL_MAILING_ADDRESS],
            dash(&form.mailing_address),
        ),
        line(
            VENDOR_HEADERS[COL_SHIPPING_ADDRESS],
            dash(&form.shipping_address),
        ),
    ]
}

fn payment_lines(state: &State) -> Lines {
    let form = &state.form;
    vec![
        line(
            VENDOR_HEADERS[COL_PAYMENT_TERMS],
            format!("{} days", form.payment_terms),
        ),
        line(
            VENDOR_HEADERS[COL_PAYMENT_METHOD],
            dash(&form.payment_method),
        ),
        line("Tax ID", dash(&form.tax_id)),
        line(VENDOR_TYPE_LABEL, dash(&form.vendor_type)),
    ]
}

/// The overview's profile rows, from the ledger where it has the value.
fn overview_lines(state: &State) -> Lines {
    let form = &state.form;
    let profile = profile(state);
    let (opening, last) = match &state.ledger {
        Some(ledger) => (
            format!("{} {}", ledger.currency, money(Decimal::ZERO)),
            ledger
                .last_transaction_date()
                .map(day)
                .unwrap_or_else(|| "No transactions".into()),
        ),
        None => (NONE.into(), NONE.into()),
    };
    vec![
        line(VENDOR_HEADERS[COL_VENDOR_ID], profile.vendor_id),
        line(VENDOR_HEADERS[COL_VENDOR_NAME], dash(&form.name)),
        line(VENDOR_HEADERS[COL_COMPANY_NAME], profile.company),
        line(VENDOR_TYPE_LABEL, dash(&form.vendor_type)),
        line(VENDOR_HEADERS[COL_STATUS], profile.status),
        line(
            VENDOR_HEADERS[COL_PAYMENT_TERMS],
            format!("{} days", form.payment_terms),
        ),
        line(
            VENDOR_HEADERS[COL_PAYMENT_METHOD],
            dash(&form.payment_method),
        ),
        line(VENDOR_HEADERS[COL_WEBSITE], dash(&form.website)),
        line("Opening Balance", opening),
        line("Last Transaction", last),
        line(VENDOR_HEADERS[COL_CREATE_TIMESTAMP], NONE.into()),
        line(VENDOR_HEADERS[COL_MODIFIED_TIMESTAMP], NONE.into()),
    ]
}

/// The whole preview for a page area `width` wide.
pub(super) fn vendor_preview(state: &State, theme: Theme, width: f32) -> Element<'static, Message> {
    let profile = profile(state);
    let inner = (width - spacing::SPACE_20 * 2.0 - spacing::MD * 2.0).max(0.0);
    let body = panels::master_detail(
        inner,
        profile_column(state, theme, &profile),
        detail_column(state, theme),
    );
    panels::page_card(
        theme,
        container(top_bar(theme, &profile.name))
            .padding(spacing::MD)
            .width(Length::Fill)
            .into(),
        container(body)
            .padding(spacing::MD)
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        None,
    )
}

/// Breadcrumb ("Vendors" goes back) and the page's actions.
fn top_bar(theme: Theme, name: &str) -> Element<'static, Message> {
    row![
        container(labels::breadcrumb(
            theme,
            TITLE,
            name.to_string(),
            Message::Preview
        ))
        .width(Length::Fill)
        .clip(true),
        button_style::icon_label(icons::EDIT, "Edit", [spacing::SPACE_8, spacing::MD])
            .style(button_style::mode(theme, true))
            .on_press(Message::Preview),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center)
    .into()
}

/// A value as link text when it holds something, plain `--` otherwise.
fn link_value(theme: Theme, value: String) -> Element<'static, Message> {
    if value == NONE {
        muted(theme, value)
    } else {
        labels::link(theme, value).into()
    }
}

/// Label over value lines, in the narrow profile column.
fn stacked(theme: Theme, lines: Lines, links: &[usize]) -> Element<'static, Message> {
    let mut list = column![].spacing(spacing::SPACE_8);
    for (index, (label, value)) in lines.into_iter().enumerate() {
        let shown: Element<'static, Message> = if links.contains(&index) {
            link_value(theme, value)
        } else {
            labels::strong(theme, value, typography::body()).into()
        };
        list = list.push(labels::field(theme, label, shown));
    }
    list.into()
}

fn section<'a>(
    state: &State,
    theme: Theme,
    which: usize,
    title: &'a str,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    panels::collapsible_section(
        theme,
        title,
        !state.collapsed_sections.contains(&which),
        Message::PreviewSectionToggled(which),
        body,
    )
}

/// Avatar, name, status, quick actions and the collapsible sections.
fn profile_column(state: &State, theme: Theme, profile: &Profile) -> Element<'static, Message> {
    let form = &state.form;
    let tone = if profile.status == "Active" {
        BadgeTone::Positive
    } else {
        BadgeTone::Neutral
    };
    let identity = column![
        badges::initials_avatar(theme, &profile.name, spacing::grown(spacing::AVATAR_LARGE)),
        labels::strong(theme, profile.name.clone(), typography::subtitle()),
        muted(theme, profile.company.clone()),
        badges::status_pill(theme, profile.status.clone(), tone),
        // Edit is the only action the preview has; more would be added
        // here as they are wired up.
        panels::quick_action(theme, icons::EDIT, "Edit", Message::Preview),
    ]
    .spacing(spacing::SM)
    .align_x(Alignment::Center)
    .width(Length::Fill);

    let custom: Element<'static, Message> = if form.custom_fields.is_empty() {
        muted(theme, NONE)
    } else {
        stacked(
            theme,
            form.custom_fields
                .iter()
                .map(|field| line(&dash(&field.name), dash(&field.value)))
                .collect(),
            &[],
        )
    };
    column![
        identity,
        section(
            state,
            theme,
            SECTION_CONTACT,
            "Contact Details",
            stacked(theme, contact_lines(state), &[0, 1, 2])
        ),
        section(
            state,
            theme,
            SECTION_PAYMENT,
            "Payment",
            stacked(theme, payment_lines(state), &[])
        ),
        section(
            state,
            theme,
            SECTION_NOTES,
            "Notes",
            stacked(
                theme,
                vec![line(
                    VENDOR_HEADERS[COL_DESCRIPTION],
                    dash(&form.description_text())
                )],
                &[]
            )
        ),
        section(
            state,
            theme,
            SECTION_CUSTOM,
            VENDOR_HEADERS[COL_CUSTOM_FIELDS],
            custom
        ),
        section(
            state,
            theme,
            SECTION_ATTACHMENTS,
            VENDOR_HEADERS[COL_ATTACHMENTS],
            attachment_list(theme, &form.attachments)
        ),
    ]
    .spacing(spacing::MD)
    .into()
}

fn attachment_list(theme: Theme, attachments: &[VendorAttachment]) -> Element<'static, Message> {
    if attachments.is_empty() {
        return muted(theme, NONE);
    }
    let mut list = column![].spacing(spacing::SPACE_8);
    for attachment in attachments {
        list = list.push(
            row![
                icons::icon(icons::FILE)
                    .size(typography::body())
                    .color(theme.neutral_front()),
                labels::strong(theme, attachment.display_name.clone(), typography::body())
                    .width(Length::Fill),
                muted(theme, forms::file_size(attachment.size_bytes)),
            ]
            .spacing(spacing::SM)
            .align_y(Alignment::Center),
        );
    }
    list.into()
}

/// The tab strip and the tab's body.
fn detail_column(state: &State, theme: Theme) -> Element<'static, Message> {
    let tabs = tabs::icon_bar(
        theme,
        &PREVIEW_TABS,
        state.preview_tab,
        Message::PreviewTabChanged,
    );
    let body = match state.preview_tab {
        TAB_OVERVIEW => overview(state, theme),
        TAB_ATTACHMENTS => panels::titled_card(
            theme,
            VENDOR_HEADERS[COL_ATTACHMENTS],
            attachment_list(theme, &state.form.attachments),
        ),
        tab => accounting_tab(state, theme, tab),
    };
    column![tabs, table_style::divider_line(theme), body]
        .spacing(spacing::SM)
        .into()
}

/// Profile rows, the financial summary bar, open bills and recent activity.
fn overview(state: &State, theme: Theme) -> Element<'static, Message> {
    let mut rows = column![].spacing(spacing::SPACE_10);
    for (label, value) in overview_lines(state) {
        rows = rows.push(labels::pair_row(theme, label, value));
    }
    let mut page = column![
        panels::titled_card(theme, "Profile Overview", rows),
        financial_summary(state, theme),
    ]
    .spacing(spacing::MD);
    if let Some(ledger) = &state.ledger {
        page = page
            .push(outstanding_balances(ledger, theme))
            .push(activity(ledger, theme));
    }
    page.into()
}

/// Debit, credit and outstanding totals, all derived from the ledger.
fn financial_summary(state: &State, theme: Theme) -> Element<'static, Message> {
    let Some(ledger) = &state.ledger else {
        let message = state
            .ledger_error
            .clone()
            .unwrap_or_else(|| "Financial summary appears once the vendor is saved.".into());
        return panels::titled_card(theme, "Financial Summary", muted(theme, message));
    };
    let currency = ledger.currency.as_str();
    let with_currency = |amount: Decimal| format!("{currency} {}", money(amount));
    let outstanding = ledger.outstanding();
    let (label, tone) = balance_label(outstanding);
    let figures = row![
        labels::stat(theme, "Total Debit", with_currency(ledger.total_debit())),
        labels::stat(theme, "Total Credit", with_currency(ledger.total_credit())),
        labels::stat(theme, "Outstanding", with_currency(outstanding.abs())),
        badges::status_pill(theme, label, tone),
    ]
    .spacing(spacing::MD)
    .align_y(Alignment::Center);
    column![
        labels::strong(theme, "Financial Summary", typography::emphasis()),
        panels::tinted_bar(theme, figures),
    ]
    .spacing(spacing::SPACE_8)
    .into()
}

/// The marker and tone of a ledger entry, by what kind of entry it is.
fn activity_marker(kind: &str) -> (&'static str, BadgeTone) {
    match kind {
        "Bill" => (icons::ACTIVITY_BILL, BadgeTone::Warning),
        "Payment" => (icons::ACTIVITY_PAYMENT, BadgeTone::Positive),
        "Debit Note" => (icons::ACTIVITY_DEBIT_NOTE, BadgeTone::Neutral),
        _ => (icons::ACTIVITY_REVERSAL, BadgeTone::Negative),
    }
}

/// The latest ledger entries as a timeline, newest first.
fn activity(ledger: &VendorLedger, theme: Theme) -> Element<'static, Message> {
    let header = row![
        labels::strong(theme, "Activity", typography::emphasis()).width(Length::Fill),
        button(labels::link(theme, "View all"))
            .padding(spacing::SPACE_2)
            .style(button_style::bare(theme))
            .on_press(Message::PreviewTabChanged(TAB_TRANSACTIONS)),
    ]
    .align_y(Alignment::Center);
    if ledger.entries.is_empty() {
        return panels::titled_card(
            theme,
            "Activity",
            muted(theme, "No transactions recorded for this vendor."),
        );
    }
    let latest: Vec<_> = ledger.entries.iter().rev().take(ACTIVITY_LIMIT).collect();
    let mut timeline = column![].spacing(0.0);
    for (index, entry) in latest.iter().enumerate() {
        let (glyph, tone) = activity_marker(&entry.kind);
        let amount = if entry.debit.is_zero() {
            entry.credit
        } else {
            entry.debit
        };
        timeline = timeline.push(panels::timeline_entry(
            theme,
            TimelineEntry {
                glyph,
                tone,
                title: format!("{} {}", entry.kind, entry.reference),
                description: dash(&entry.description),
                amount: format!("{} {}", ledger.currency, money(amount)),
                date: day(entry.date),
            },
            index + 1 == latest.len(),
        ));
    }
    panels::padded(theme, column![header, timeline].spacing(spacing::SPACE_10))
}

/// Transactions, ledger or trial balance, or why there is none.
fn accounting_tab(state: &State, theme: Theme, tab: usize) -> Element<'static, Message> {
    let Some(ledger) = &state.ledger else {
        let message = state
            .ledger_error
            .clone()
            .unwrap_or_else(|| "Accounting appears once the vendor is saved.".into());
        return panels::titled_card(theme, PREVIEW_TABS[tab].1, muted(theme, message));
    };
    let body = match tab {
        TAB_TRANSACTIONS => transaction_history(state, ledger, theme),
        TAB_LEDGER => vendor_ledger_tab(ledger, theme),
        _ => vendor_trial_balance(ledger, theme),
    };
    panels::titled_card(theme, PREVIEW_TABS[tab].1, body)
}

/// Loads the read-only AP ledger for the vendor being previewed. Nothing
/// is posted; an unsaved vendor simply has no ledger.
pub(super) fn refresh_ledger(state: &mut State, ctx: &Ctx) {
    state.ledger = None;
    state.ledger_error = None;
    state.preview_tab = TAB_OVERVIEW;
    let Some(vendor_id) = state.editing_id else {
        return;
    };
    match PurchaseService::new(ctx.db.clone()).vendor_ledger(ctx.company_id, vendor_id) {
        Ok(ledger) => state.ledger = Some(ledger),
        Err(err) => state.ledger_error = Some(err.to_string()),
    }
}

/// Payable / receivable / settled, from the sign of the outstanding balance.
fn balance_label(outstanding: Decimal) -> (&'static str, BadgeTone) {
    if outstanding > Decimal::ZERO {
        ("Payable", BadgeTone::Warning)
    } else if outstanding < Decimal::ZERO {
        ("Receivable", BadgeTone::Positive)
    } else {
        ("Settled", BadgeTone::Neutral)
    }
}

fn muted(theme: Theme, message: impl text::IntoFragment<'static>) -> Element<'static, Message> {
    labels::muted(theme, message, typography::body()).into()
}

fn transaction_history(
    state: &State,
    ledger: &VendorLedger,
    theme: Theme,
) -> Element<'static, Message> {
    if ledger.entries.is_empty() {
        return muted(theme, "No transactions recorded for this vendor.");
    }
    let mut entries: Vec<_> = ledger.entries.iter().collect();
    if state.ledger_newest_first {
        entries.reverse();
    }
    let rows = entries
        .iter()
        .map(|e| {
            vec![
                day(e.date),
                format!("{} (#{})", e.reference, e.transaction_id),
                e.kind.clone(),
                e.description.clone(),
                side(e.debit),
                side(e.credit),
                money(e.running_balance),
            ]
        })
        .collect();
    let order = if state.ledger_newest_first {
        "Date: newest first"
    } else {
        "Date: oldest first"
    };
    column![
        button(text(order).size(typography::label()))
            .padding([spacing::SPACE_4, spacing::SM])
            .style(button_style::outlined(theme, false))
            .on_press(Message::ToggleLedgerOrder),
        table_style::grid(
            theme,
            &vendor_ledger::HISTORY_COLUMNS,
            &[2, 3, 2, 4, 2, 2, 2],
            &[4, 5, 6],
            rows,
            None,
        ),
    ]
    .spacing(spacing::SM)
    .into()
}

/// T-account layout: debits on the left, credits on the right, with the
/// opening and closing balance and each entry traceable to its source.
fn vendor_ledger_tab(ledger: &VendorLedger, theme: Theme) -> Element<'static, Message> {
    if ledger.entries.is_empty() {
        return muted(theme, "No ledger entries for this vendor.");
    }
    let t_side = |title: &str, debit_side: bool| {
        let rows: Vec<Vec<String>> = ledger
            .entries
            .iter()
            .filter(|e| {
                if debit_side {
                    !e.debit.is_zero()
                } else {
                    !e.credit.is_zero()
                }
            })
            .map(|e| {
                vec![
                    day(e.date),
                    format!("{} (#{})", e.reference, e.transaction_id),
                    money(if debit_side { e.debit } else { e.credit }),
                ]
            })
            .collect();
        let total = if debit_side {
            ledger.total_debit()
        } else {
            ledger.total_credit()
        };
        column![
            labels::strong(theme, title.to_string(), typography::label()),
            table_style::grid(
                theme,
                &vendor_ledger::T_ACCOUNT_COLUMNS,
                &[2, 3, 2],
                &[2],
                rows,
                Some(vec!["Total".into(), String::new(), money(total)]),
            ),
        ]
        .spacing(spacing::SPACE_6)
        .width(Length::FillPortion(1))
    };
    let closing = ledger.outstanding();
    let (label, tone) = balance_label(closing);
    let summary = row![
        labels::muted(
            theme,
            format!("Opening balance: {}", money(Decimal::ZERO)),
            typography::body()
        ),
        Space::with_width(Length::Fill),
        labels::strong(
            theme,
            format!(
                "Closing balance: {} {}",
                ledger.currency,
                money(closing.abs())
            ),
            typography::body()
        ),
        badges::status_pill(theme, label, tone),
    ]
    .spacing(spacing::SM)
    .align_y(Alignment::Center);
    column![
        row![t_side("Debit (Dr)", true), t_side("Credit (Cr)", false)].spacing(spacing::LG),
        summary,
    ]
    .spacing(spacing::MD)
    .into()
}

/// Only the accounts this vendor's transactions touched, with their
/// debit/credit totals for the vendor's reporting period.
fn vendor_trial_balance(ledger: &VendorLedger, theme: Theme) -> Element<'static, Message> {
    let (Some(from), Some(to)) = (
        ledger.first_transaction_date(),
        ledger.last_transaction_date(),
    ) else {
        return muted(
            theme,
            "No accounts affected: this vendor has no transactions.",
        );
    };
    let rows = ledger
        .accounts
        .iter()
        .map(|a| {
            let net = a.net();
            vec![
                a.code.clone(),
                a.name.clone(),
                if net > Decimal::ZERO {
                    money(net)
                } else {
                    String::new()
                },
                if net < Decimal::ZERO {
                    money(-net)
                } else {
                    String::new()
                },
            ]
        })
        .collect();
    let debit: Decimal = ledger
        .accounts
        .iter()
        .map(|a| a.net().max(Decimal::ZERO))
        .sum();
    let credit: Decimal = ledger
        .accounts
        .iter()
        .map(|a| (-a.net()).max(Decimal::ZERO))
        .sum();
    column![
        muted(theme, format!("Period: {} to {}", day(from), day(to))),
        table_style::grid(
            theme,
            &vendor_ledger::TRIAL_BALANCE_COLUMNS,
            &[2, 6, 2, 2],
            &[2, 3],
            rows,
            Some(vec!["".into(), "Total".into(), money(debit), money(credit)]),
        ),
    ]
    .spacing(spacing::SM)
    .into()
}

/// Open bills from the payables subledger with overdue/aging information.
/// It breaks down the summary's balance rather than adding to it.
fn outstanding_balances(ledger: &VendorLedger, theme: Theme) -> Element<'static, Message> {
    if ledger.open_payables.is_empty() {
        return panels::titled_card(
            theme,
            "Outstanding Balances",
            muted(theme, "No open bills."),
        );
    }
    let today = chrono::Local::now().date_naive();
    let rows = ledger
        .open_payables
        .iter()
        .map(|p| {
            vec![
                p.reference.clone().unwrap_or_default(),
                day(p.due_date),
                if p.is_overdue(today) {
                    format!("Overdue {}d", (today - p.due_date).num_days())
                } else {
                    "Not due".into()
                },
                money(p.amount),
                money(p.balance()),
            ]
        })
        .collect();
    let aging = ledger.aging(today);
    let overdue = aging.days_1_to_30 + aging.days_31_to_60 + aging.days_61_to_90 + aging.over_90;
    let total = aging.total();
    let overdue_line = if overdue.is_zero() {
        badges::status_pill(theme, "Nothing overdue", BadgeTone::Positive)
    } else {
        badges::status_pill(
            theme,
            format!("Overdue {} {}", ledger.currency, money(overdue)),
            BadgeTone::Negative,
        )
    };
    panels::titled_card(
        theme,
        "Outstanding Balances",
        column![
            overdue_line,
            table_style::grid(
                theme,
                &vendor_ledger::OPEN_BILL_COLUMNS,
                &[3, 2, 2, 2, 2],
                &[3, 4],
                rows,
                Some(vec![
                    "Total".into(),
                    String::new(),
                    String::new(),
                    String::new(),
                    format!("{} {}", ledger.currency, money(total)),
                ]),
            ),
        ]
        .spacing(spacing::SM),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounts_payable::payables::Payable;
    use crate::backend::parties::vendors::CustomField;
    use crate::backend::purchases::vendor_ledger::{VendorAccountTotal, VendorLedgerEntry};
    use crate::frontend::components::font_scale;
    use chrono::NaiveDate;

    /// The `VENDOR_HEADERS` columns the preview shows by name. Debit, credit
    /// and balance appear as the financial summary's figures instead, and the
    /// serial and select columns belong to the table only.
    const PREVIEW_COLUMNS: [usize; 15] = [
        COL_VENDOR_ID,
        COL_VENDOR_NAME,
        COL_COMPANY_NAME,
        COL_PHONE,
        COL_EMAIL,
        COL_WEBSITE,
        COL_MAILING_ADDRESS,
        COL_SHIPPING_ADDRESS,
        COL_PAYMENT_TERMS,
        COL_PAYMENT_METHOD,
        COL_ATTACHMENTS,
        COL_DESCRIPTION,
        COL_STATUS,
        COL_CREATE_TIMESTAMP,
        COL_MODIFIED_TIMESTAMP,
    ];

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, day).unwrap()
    }

    fn entry(kind: &str, day: u32, debit: i64, credit: i64, running: i64) -> VendorLedgerEntry {
        VendorLedgerEntry {
            transaction_id: i64::from(day),
            date: date(day),
            kind: kind.into(),
            reference: format!("REF-{day}"),
            description: "A deliberately long description that should wrap rather than push the amount off the card".into(),
            debit: Decimal::from(debit),
            credit: Decimal::from(credit),
            running_balance: Decimal::from(running),
        }
    }

    fn full_ledger() -> VendorLedger {
        VendorLedger {
            currency: "PKR".into(),
            entries: vec![
                entry("Bill", 1, 0, 5000, 5000),
                entry("Payment", 2, 2000, 0, 3000),
                entry("Debit Note", 3, 500, 0, 2500),
                entry("Reversal", 4, 0, 100, 2600),
            ],
            accounts: vec![VendorAccountTotal {
                code: "2000".into(),
                name: "Accounts Payable".into(),
                debit: Decimal::from(2500),
                credit: Decimal::from(5100),
            }],
            open_payables: vec![Payable::new(1, 1, date(1), date(15), Decimal::from(5000))],
        }
    }

    fn empty_ledger() -> VendorLedger {
        VendorLedger {
            currency: "PKR".into(),
            entries: vec![],
            accounts: vec![],
            open_payables: vec![],
        }
    }

    fn state(ledger: Option<VendorLedger>) -> State {
        let mut state = State::default();
        state.form.name = "Al-Noor Traders International Private Limited".into();
        state.form.email = "accounts@alnoor.example".into();
        state.form.custom_fields = vec![CustomField {
            name: "Region".into(),
            value: "North".into(),
        }];
        state.form.attachments = vec![VendorAttachment {
            file_name: "a.pdf".into(),
            display_name: "Agreement.pdf".into(),
            size_bytes: 2048,
        }];
        state.ledger = ledger;
        state.show_preview = true;
        state
    }

    #[test]
    fn the_preview_builds_for_every_tab_order_theme_and_ledger_shape() {
        let _guard = font_scale::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        for ledger in [None, Some(empty_ledger()), Some(full_ledger())] {
            for tab in 0..PREVIEW_TABS.len() {
                for newest_first in [false, true] {
                    for theme in [Theme::Light, Theme::Dark] {
                        let mut state = state(ledger.clone());
                        state.preview_tab = tab;
                        state.ledger_newest_first = newest_first;
                        let _element = vendor_preview(&state, theme, 1366.0);
                    }
                }
            }
        }
    }

    #[test]
    fn the_preview_builds_at_every_width_with_sections_open_and_folded() {
        let _guard = font_scale::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let before = font_scale::window_width();
        for width in [640.0, 900.0, 1366.0, 1920.0] {
            font_scale::set_window_width(width);
            for folded in [false, true] {
                let mut state = state(Some(full_ledger()));
                if folded {
                    state.collapsed_sections = (0..=SECTION_ATTACHMENTS).collect();
                }
                for theme in [Theme::Light, Theme::Dark] {
                    let _element = vendor_preview(&state, theme, width);
                }
            }
        }
        font_scale::set_window_width(before);
    }

    #[test]
    fn every_vendor_column_the_preview_shows_is_labelled_in_it() {
        let state = state(Some(full_ledger()));
        let shown: Vec<String> = [
            overview_lines(&state),
            contact_lines(&state),
            payment_lines(&state),
            vec![line(VENDOR_HEADERS[COL_DESCRIPTION], String::new())],
        ]
        .concat()
        .into_iter()
        .map(|(label, _)| label)
        .chain([
            VENDOR_HEADERS[COL_ATTACHMENTS].to_string(),
            VENDOR_HEADERS[COL_CUSTOM_FIELDS].to_string(),
        ])
        .collect();
        for column in PREVIEW_COLUMNS {
            assert!(
                shown.iter().any(|label| label == VENDOR_HEADERS[column]),
                "{} is missing from the preview",
                VENDOR_HEADERS[column]
            );
        }
    }

    #[test]
    fn missing_values_read_as_dashes_and_nothing_is_invented() {
        let state = State::default();
        for (label, value) in contact_lines(&state) {
            assert_eq!(value, NONE, "{label}");
        }
        let overview = overview_lines(&state);
        assert!(overview
            .iter()
            .any(|(l, v)| l == "Last Transaction" && v == NONE));
    }

    #[test]
    fn the_overview_takes_its_opening_balance_and_last_transaction_from_the_ledger() {
        let state = state(Some(full_ledger()));
        let lines = overview_lines(&state);
        let find = |label: &str| lines.iter().find(|(l, _)| l == label).unwrap().1.clone();
        assert_eq!(find("Opening Balance"), "PKR 0.00");
        assert_eq!(find("Last Transaction"), day(date(4)));
    }

    #[test]
    fn folding_a_section_toggles_it_in_the_state_set() {
        let mut state = State::default();
        for _ in 0..2 {
            let before = state.collapsed_sections.contains(&SECTION_NOTES);
            if !state.collapsed_sections.remove(&SECTION_NOTES) {
                state.collapsed_sections.insert(SECTION_NOTES);
            }
            assert_ne!(before, state.collapsed_sections.contains(&SECTION_NOTES));
        }
    }
}
