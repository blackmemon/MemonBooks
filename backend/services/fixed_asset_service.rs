//! The fixed-assets domain's service layer, for `accounting/fixed_assets_view.rs`.

use chrono::NaiveDate;

use crate::backend::core::errors::AppResult;
use crate::backend::fixed_assets::assets::Asset;
use crate::backend::repositories::fixed_asset_repository::FixedAssetRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::Database;

#[derive(Clone)]
pub struct FixedAssetService {
    repo: FixedAssetRepository,
}

impl FixedAssetService {
    pub fn new(db: Database) -> Self {
        Self {
            repo: FixedAssetRepository::new(db),
        }
    }

    pub fn list_assets(&self, company_id: i64) -> AppResult<Vec<Asset>> {
        self.repo.list_assets_for_company(company_id)
    }

    pub fn get_asset(&self, asset_id: i64) -> AppResult<Asset> {
        self.repo.get_asset(asset_id)
    }

    pub fn record_depreciation(
        &self,
        asset_id: i64,
        depreciation_date: NaiveDate,
    ) -> AppResult<Option<Transaction>> {
        self.repo.record_depreciation(asset_id, depreciation_date)
    }
}
