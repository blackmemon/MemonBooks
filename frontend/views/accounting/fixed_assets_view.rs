//! List fixed assets and record depreciation. Grouped under
//! `accounting/` -- the frozen frontend/views/ structure (Section 2) has
//! no dedicated `fixed_assets/` directory, and depreciation is
//! fundamentally a ledger topic.

use chrono::Utc;

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::backend::fixed_assets::assets::Asset;
use crate::backend::repositories::fixed_asset_repository::FixedAssetRepository;
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
    let repo = FixedAssetRepository::new(ctx.db.clone());
    match repo.list_assets_for_company(ctx.company_id) {
        Ok(assets) => State { assets, ..Default::default() },
        Err(err) => State { error: Some(err.to_string()), ..Default::default() },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::Refresh => *state = load(ctx),
        Message::RecordDepreciation(asset_id) => {
            let repo = FixedAssetRepository::new(ctx.db.clone());
            let today = Utc::now().date_naive();
            match repo.record_depreciation(asset_id, today) {
                Ok(Some(transaction)) => {
                    state.message = Some(format!("Posted depreciation transaction #{}.", transaction.id.unwrap_or_default()));
                    if let Ok(updated) = repo.get_asset(asset_id) {
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
