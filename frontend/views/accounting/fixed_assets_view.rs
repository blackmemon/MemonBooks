//! List fixed assets and record depreciation. Grouped under
//! `accounting/` -- the frozen frontend/views/ structure (Section 2) has
//! no dedicated `fixed_assets/` directory, and depreciation is
//! fundamentally a ledger topic.

use chrono::Utc;

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::backend::fixed_assets::assets::Asset;
use crate::backend::services::fixed_asset_service::FixedAssetService;
use crate::frontend::styles::colors;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    assets: Vec<Asset>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    RecordDepreciation(i64),
    Refresh,
}

pub fn load(ctx: &Ctx) -> State {
    let service = FixedAssetService::new(ctx.db.clone());
    match service.list_assets(ctx.company_id) {
        Ok(assets) => State { assets, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::Refresh => *state = load(ctx),
        Message::RecordDepreciation(asset_id) => {
            let service = FixedAssetService::new(ctx.db.clone());
            let today = Utc::now().date_naive();
            match service.record_depreciation(asset_id, today) {
                Ok(Some(transaction)) => {
                    state.message = Some(format!("Posted depreciation transaction #{}.", transaction.id.unwrap_or_default()));
                    if let Ok(updated) = service.get_asset(asset_id) {
                        if let Some(existing) = state.assets.iter_mut().find(|a| a.id == Some(asset_id)) {
                            *existing = updated;
                        }
                    }
                }
                Ok(None) => state.message = Some("Nothing left to depreciate.".into()),
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mut list = column![row![
        text("Asset").width(Length::Fill),
        text("Cost").width(Length::Fixed(100.0)),
        text("Accum. Depr.").width(Length::Fixed(100.0)),
        text("Book Value").width(Length::Fixed(100.0)),
        text("").width(Length::Fixed(140.0)),
    ]]
    .spacing(4);

    for asset in &state.assets {
        let id = asset.id.unwrap_or_default();
        list = list.push(row![
            text(asset.name.clone()).width(Length::Fill),
            text(asset.acquisition_cost.to_string()).width(Length::Fixed(100.0)),
            text(asset.accumulated_depreciation.to_string()).width(Length::Fixed(100.0)),
            text(asset.book_value().to_string()).width(Length::Fixed(100.0)),
            button(text("Depreciate")).on_press(Message::RecordDepreciation(id)).padding(4).width(Length::Fixed(140.0)),
        ]);
    }

    let mut content = column![text("Fixed Assets").size(24), scrollable(list).height(Length::Fill)]
        .spacing(16)
        .padding(20);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(colors::WHITE_POSITIVE_FRONT));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(colors::WHITE_NEGATIVE_FRONT));
    }

    container(content).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::chart_of_accounts::Account;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use crate::backend::fixed_assets::asset_categories::AssetCategory;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::fixed_asset_repository::{self, FixedAssetRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::database::database::Database;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use std::path::PathBuf;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn ctx_with_one_asset() -> (Ctx, i64) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        db.run_migrations(fixed_asset_repository::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        // Spans acquisition through well past "today" so depreciation
        // (dated `Utc::now()` by the view) always lands inside an open period.
        accounting_repo.create_fiscal_period(&FiscalPeriod::new(company_id, "Open Period", date(2026, 1, 1), date(2030, 12, 31))).unwrap();
        let asset_account = accounting_repo.create_account(&Account::new(company_id, "1500", "Equipment", AccountType::Asset)).unwrap();
        let accum_depr = accounting_repo.create_account(&Account::new(company_id, "1510", "Accumulated Depreciation", AccountType::Asset)).unwrap();
        let depr_expense = accounting_repo.create_account(&Account::new(company_id, "6100", "Depreciation Expense", AccountType::Expense)).unwrap();
        let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();

        let asset_repo = FixedAssetRepository::new(db.clone());
        let category = asset_repo
            .create_category(&AssetCategory::new(company_id, "Equipment", asset_account.id.unwrap(), accum_depr.id.unwrap(), depr_expense.id.unwrap(), 5))
            .unwrap();
        let asset = Asset::new(company_id, category.id.unwrap(), "Office Laptop", date(2026, 1, 1), dec!(6000), dec!(0), 5);
        let (created, _transaction) = asset_repo.acquire_asset(&asset, cash.id.unwrap()).unwrap();

        (Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from(".") }, created.id.unwrap())
    }

    #[test]
    fn refresh_reloads_the_asset_list_from_the_database() {
        let (ctx, _asset_id) = ctx_with_one_asset();
        let mut state = State::default();

        update(&mut state, Message::Refresh, &ctx);

        assert_eq!(state.assets.len(), 1);
        assert_eq!(state.assets[0].name, "Office Laptop");
    }

    #[test]
    fn recording_depreciation_posts_a_transaction_and_updates_the_asset_in_place() {
        let (ctx, asset_id) = ctx_with_one_asset();
        let mut state = load(&ctx);
        assert_eq!(state.assets[0].accumulated_depreciation, dec!(0));

        update(&mut state, Message::RecordDepreciation(asset_id), &ctx);

        assert!(state.message.as_deref().unwrap_or_default().starts_with("Posted depreciation transaction"));
        assert!(state.error.is_none());
        assert_eq!(state.assets[0].accumulated_depreciation, dec!(1200)); // 6000 / 5 years
    }

    #[test]
    fn recording_depreciation_for_an_unknown_asset_sets_an_error() {
        let (ctx, _asset_id) = ctx_with_one_asset();
        let mut state = load(&ctx);

        update(&mut state, Message::RecordDepreciation(999), &ctx);

        assert!(state.error.is_some());
        assert!(state.message.is_none());
    }
}
