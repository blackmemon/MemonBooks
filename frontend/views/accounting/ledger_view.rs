//! Account ledger and party aging using the existing accounting service.
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::services::{party_service::PartyService, report_service::ReportService};
use crate::backend::transactions::documents::parse_date;
use crate::frontend::{
    components::{colors::Theme, dropdown, inputs, panels, spacing, typography},
    views::Ctx,
};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Element, Length};
use rust_decimal::Decimal;
#[derive(Debug, Clone)]
pub struct State {
    pub report: String,
    pub choices: Vec<String>,
    pub selection: Option<String>,
    pub party_type: String,
    pub from: String,
    pub to: String,
    pub filter_date: bool,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub summary: String,
    pub error: String,
}
impl Default for State {
    fn default() -> Self {
        let today = chrono::Utc::now().date_naive();
        Self {
            report: "General Ledger".into(),
            choices: vec![],
            selection: None,
            party_type: "Customer".into(),
            from: (today - chrono::Duration::days(30)).to_string(),
            to: today.to_string(),
            filter_date: false,
            headers: vec![],
            rows: vec![],
            summary: String::new(),
            error: String::new(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum Message {
    Report(String),
    PartyType(String),
    Select(String),
    From(String),
    To(String),
    Filter(bool),
    View,
}
pub fn load(ctx: &Ctx) -> State {
    let mut s = State::default();
    if let Err(e) = choices(&mut s, ctx) {
        s.error = e.to_string();
    }
    s
}
fn choices(s: &mut State, ctx: &Ctx) -> AppResult<()> {
    s.selection = None;
    s.rows.clear();
    s.summary.clear();
    s.choices = if s.report == "AR / AP" {
        let p = PartyService::new(ctx.db.clone());
        if s.party_type == "Customer" {
            p.list_customers(ctx.company_id)?
                .into_iter()
                .filter(|p| p.is_active)
                .map(|p| format!("{} · {}", p.id.unwrap_or_default(), p.name))
                .collect()
        } else {
            p.list_vendors(ctx.company_id)?
                .into_iter()
                .filter(|p| p.is_active)
                .map(|p| format!("{} · {}", p.id.unwrap_or_default(), p.name))
                .collect()
        }
    } else {
        ReportService::new(ctx.db.clone())
            .list_accounts(ctx.company_id)?
            .into_iter()
            .map(|a| {
                format!(
                    "{} · {} — {}",
                    a.id.unwrap_or_default(),
                    a.account_code,
                    a.account_name
                )
            })
            .collect()
    };
    Ok(())
}
pub fn update(s: &mut State, m: Message, ctx: &Ctx) {
    let result = match m {
        Message::Report(v) => {
            s.report = v;
            choices(s, ctx)
        }
        Message::PartyType(v) => {
            s.party_type = v;
            choices(s, ctx)
        }
        Message::Select(v) => {
            s.selection = Some(v);
            Ok(())
        }
        Message::From(v) => {
            s.from = v;
            Ok(())
        }
        Message::To(v) => {
            s.to = v;
            Ok(())
        }
        Message::Filter(v) => {
            s.filter_date = v;
            Ok(())
        }
        Message::View => run(s, ctx),
    };
    s.error = result.err().map(|e| e.to_string()).unwrap_or_default();
}
fn run(s: &mut State, ctx: &Ctx) -> AppResult<()> {
    let service = ReportService::new(ctx.db.clone());
    let to = parse_date(&s.to)?;
    s.rows.clear();
    if s.report == "Trial Balance" {
        let tb = service.trial_balance_as_of(ctx.company_id, to)?;
        s.headers = ["Account Code", "Account Name", "Debit", "Credit"]
            .map(str::to_string)
            .to_vec();
        s.summary = format!(
            "Debits {} · Credits {} · {}",
            tb.total_debits(),
            tb.total_credits(),
            if tb.total_debits() == tb.total_credits() {
                "Balanced"
            } else {
                "Unbalanced"
            }
        );
        s.rows = tb
            .lines
            .iter()
            .map(|l| {
                vec![
                    l.account_code.clone(),
                    l.account_name.clone(),
                    l.debit.to_string(),
                    l.credit.to_string(),
                ]
            })
            .collect();
        return Ok(());
    }
    let id = s
        .selection
        .as_deref()
        .and_then(|v| v.split(" · ").next())
        .and_then(|v| v.parse::<i64>().ok())
        .ok_or_else(|| AppError::Validation("Select an account or party first".into()))?;
    if s.report == "General Ledger" {
        let from = parse_date(&s.from)?;
        if s.filter_date && from > to {
            return Err(AppError::Validation(
                "From date must not follow To date".into(),
            ));
        }
        let ledger = service.general_ledger(ctx.company_id, id, to)?;
        s.summary = format!(
            "Closing balance: {}",
            ledger.last().map(|e| e.running_balance).unwrap_or_default()
        );
        s.headers = [
            "Date",
            "Entry No",
            "Narration",
            "Debit",
            "Credit",
            "Balance",
        ]
        .map(str::to_string)
        .to_vec();
        s.rows = ledger
            .iter()
            .filter(|e| !s.filter_date || e.transaction_date >= from)
            .map(|e| {
                vec![
                    e.transaction_date.to_string(),
                    e.transaction_id.to_string(),
                    e.description.clone().unwrap_or_default(),
                    e.debit.to_string(),
                    e.credit.to_string(),
                    e.running_balance.to_string(),
                ]
            })
            .collect();
    } else {
        let items = service.party_open_items(ctx.company_id, &s.party_type, id, to)?;
        let mut buckets = [Decimal::ZERO; 4];
        s.headers = [
            "Date",
            "Reference",
            "Original",
            "Outstanding",
            "Age (days)",
            "Bucket",
        ]
        .map(str::to_string)
        .to_vec();
        for (date, reference, original, outstanding) in items {
            let age = (to - date).num_days().max(0);
            let bucket = if age <= 30 {
                0
            } else if age <= 60 {
                1
            } else if age <= 90 {
                2
            } else {
                3
            };
            buckets[bucket] += outstanding;
            s.rows.push(vec![
                date.to_string(),
                reference,
                original.to_string(),
                outstanding.to_string(),
                age.to_string(),
                ["0–30", "31–60", "61–90", "90+"][bucket].into(),
            ]);
        }
        s.summary = format!(
            "0–30: {} · 31–60: {} · 61–90: {} · 90+: {} · Outstanding: {}",
            buckets[0],
            buckets[1],
            buckets[2],
            buckets[3],
            buckets.iter().sum::<Decimal>()
        );
    }
    Ok(())
}
pub fn view(s: &State, t: Theme) -> Element<'_, Message> {
    let mut page = column![row![
        text(&s.report).size(typography::title()),
        pick_list(
            vec![
                "General Ledger".to_string(),
                "Trial Balance".into(),
                "AR / AP".into()
            ],
            Some(s.report.clone()),
            Message::Report
        )
        .style(dropdown::style(t))
    ]
    .spacing(spacing::SPACE_18)]
    .spacing(spacing::SPACE_18)
    .padding(spacing::SPACE_20);
    let mut filters = row![].spacing(spacing::SPACE_12);
    if s.report == "AR / AP" {
        filters = filters.push(pick_list(
            vec!["Customer".to_string(), "Vendor".into()],
            Some(s.party_type.clone()),
            Message::PartyType,
        ));
    }
    if s.report != "Trial Balance" {
        filters = filters.push(
            pick_list(s.choices.clone(), s.selection.clone(), Message::Select)
                .placeholder("Select account or party")
                .width(Length::Fill)
                .style(dropdown::style(t)),
        );
    }
    if s.report == "General Ledger" {
        filters = filters
            .push(checkbox("Filter by date", s.filter_date).on_toggle(Message::Filter))
            .push(
                text_input("From YYYY-MM-DD", &s.from)
                    .on_input(Message::From)
                    .width(170)
                    .style(inputs::text(t)),
            );
    }
    filters = filters
        .push(
            text_input("As of YYYY-MM-DD", &s.to)
                .on_input(Message::To)
                .width(170)
                .style(inputs::text(t)),
        )
        .push(button("View").on_press(Message::View));
    page = page
        .push(filters)
        .push(text(&s.error).color(t.negative_front()))
        .push(text(&s.summary));
    let mut grid = column![].spacing(spacing::SPACE_1);
    let mut header = row![].spacing(spacing::SPACE_8);
    for h in &s.headers {
        header = header.push(text(h).width(Length::Fill));
    }
    grid = grid.push(container(header).padding(spacing::SPACE_12));
    for data in &s.rows {
        let mut r = row![].spacing(spacing::SPACE_8);
        for v in data {
            r = r.push(text(v).size(typography::body()).width(Length::Fill));
        }
        grid = grid.push(
            container(r)
                .padding(spacing::SPACE_10)
                .style(panels::flat_card(t, 0.)),
        );
    }
    page = page.push(container(grid).style(panels::flat_card(t, 8.)));
    scrollable(page).into()
}
