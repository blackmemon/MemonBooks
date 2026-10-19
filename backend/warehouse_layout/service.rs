//! The warehouse page's service layer: warehouses (stored by
//! `InventoryRepository`), their companies, and the bin grid
//! (`WarehouseLayoutRepository`) behind
//! `accounting/maintain/warehouse_view.rs`.

use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::inventory::products::Product;
use crate::backend::inventory::warehouses::Warehouse;
use crate::backend::organization::companies::Company;
use crate::backend::repositories::inventory_repository::InventoryRepository;
use crate::backend::repositories::organization_repository::OrganizationRepository;
use crate::backend::warehouse_layout::layout::{WarehouseLayout, WarehouseOverview};
use crate::backend::warehouse_layout::repository::WarehouseLayoutRepository;
use crate::database::database::Database;

/// What the add/edit panel saves.
#[derive(Debug, Clone, PartialEq)]
pub struct WarehouseDraft {
    /// `None` adds a warehouse.
    pub id: Option<i64>,
    pub company_id: i64,
    pub name: String,
    pub lines: u32,
    pub positions: u32,
    pub default_capacity: Option<Decimal>,
}

/// What [`WarehouseService::remove_warehouse`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    Deleted,
    /// The warehouse holds stock or has stock movements, so it was made
    /// inactive instead.
    Archived,
}

#[derive(Clone)]
pub struct WarehouseService {
    inventory: InventoryRepository,
    layouts: WarehouseLayoutRepository,
    organization: OrganizationRepository,
}

impl WarehouseService {
    pub fn new(db: Database) -> Self {
        Self {
            inventory: InventoryRepository::new(db.clone()),
            layouts: WarehouseLayoutRepository::new(db.clone()),
            organization: OrganizationRepository::new(db),
        }
    }

    pub fn list_companies(&self) -> AppResult<Vec<Company>> {
        self.organization.list_companies()
    }

    /// The items a bin of a warehouse of `company_id` may hold.
    pub fn list_products(&self, company_id: i64) -> AppResult<Vec<Product>> {
        Ok(self
            .inventory
            .list_products_for_company(company_id)?
            .into_iter()
            .filter(|p| p.track_inventory)
            .collect())
    }

    /// Every warehouse of every company, with its grid and contents.
    pub fn list_overviews(&self) -> AppResult<Vec<WarehouseOverview>> {
        let mut overviews = Vec::new();
        for company in self.list_companies()? {
            let Some(company_id) = company.id else {
                continue;
            };
            for warehouse in self.inventory.list_warehouses_for_company(company_id)? {
                let id = warehouse.id.unwrap_or_default();
                overviews.push(WarehouseOverview {
                    layout: self.layouts.get_layout(id)?,
                    bins: self.layouts.bins(id)?,
                    company_name: company.name.clone(),
                    warehouse,
                });
            }
        }
        Ok(overviews)
    }

    fn find_warehouse(&self, company_id: i64, id: i64) -> AppResult<Warehouse> {
        self.inventory
            .list_warehouses_for_company(company_id)?
            .into_iter()
            .find(|w| w.id == Some(id))
            .ok_or_else(|| AppError::NotFound(format!("warehouse {id} not found")))
    }

    /// Adds or edits a warehouse and lays out its bins. The name must be
    /// unique within the company (ignoring case); a new warehouse gets
    /// the next free `WH-<n>` code. The company of an existing warehouse
    /// does not change.
    pub fn save_warehouse(&self, draft: &WarehouseDraft) -> AppResult<i64> {
        let name = draft.name.trim();
        let siblings = self
            .inventory
            .list_warehouses_for_company(draft.company_id)?;
        if siblings
            .iter()
            .any(|w| w.id != draft.id && w.name.trim().eq_ignore_ascii_case(name))
        {
            return Err(AppError::Validation(format!(
                "a warehouse named {name} already exists in this company"
            )));
        }
        let layout_for =
            |id| WarehouseLayout::new(id, draft.lines, draft.positions, draft.default_capacity);
        layout_for(0).validate()?;
        match draft.id {
            Some(id) => {
                let existing = self.find_warehouse(draft.company_id, id)?;
                self.layouts.save_layout(&layout_for(id))?;
                self.inventory.update_warehouse(&Warehouse {
                    name: name.to_string(),
                    ..existing
                })?;
                Ok(id)
            }
            None => {
                let code = (1..)
                    .map(|n| format!("WH-{n}"))
                    .find(|code| !siblings.iter().any(|w| w.code.eq_ignore_ascii_case(code)))
                    .unwrap_or_default();
                let created = self.inventory.create_warehouse(&Warehouse::new(
                    draft.company_id,
                    name,
                    code,
                ))?;
                let id = created.id.unwrap_or_default();
                if let Err(err) = self.layouts.save_layout(&layout_for(id)) {
                    let _ = self.inventory.delete_warehouse(id);
                    return Err(err);
                }
                Ok(id)
            }
        }
    }

    /// Whether any stock movement was recorded against the warehouse,
    /// read through the company's products' movement history.
    fn has_movements(&self, company_id: i64, warehouse_id: i64) -> AppResult<bool> {
        for product in self.inventory.list_products_for_company(company_id)? {
            let movements = self
                .inventory
                .list_movements_for_product(company_id, product.id.unwrap_or_default())?;
            if movements.iter().any(|m| m.warehouse_id == warehouse_id) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Deletes a warehouse with no stock; one whose bins hold stock or
    /// that has stock movements is archived instead.
    pub fn remove_warehouse(&self, company_id: i64, id: i64) -> AppResult<Removal> {
        let warehouse = self.find_warehouse(company_id, id)?;
        if self.layouts.has_placements(id)? || self.has_movements(company_id, id)? {
            self.inventory.update_warehouse(&Warehouse {
                is_active: false,
                ..warehouse
            })?;
            return Ok(Removal::Archived);
        }
        self.layouts.delete_layout(id)?;
        self.inventory.delete_warehouse(id)?;
        Ok(Removal::Deleted)
    }

    /// Sets how much of an item a bin holds (zero removes it). This
    /// records where stock sits only; it is not a stock movement.
    pub fn place_stock(&self, bin_id: i64, product_id: i64, quantity: Decimal) -> AppResult<()> {
        self.layouts.set_placement(bin_id, product_id, quantity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::inventory::categories::Category;
    use crate::backend::repositories::{inventory_repository, organization_repository};
    use crate::backend::warehouse_layout::layout::{Fullness, UNCATEGORIZED};
    use crate::backend::warehouse_layout::repository;
    use rust_decimal_macros::dec;

    struct Fixture {
        service: WarehouseService,
        inventory: InventoryRepository,
        company_id: i64,
        other_company_id: i64,
    }

    fn fixture() -> Fixture {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(inventory_repository::MIGRATIONS).unwrap();
        db.run_migrations(repository::MIGRATIONS).unwrap();
        let org = OrganizationRepository::new(db.clone());
        let company_id = org
            .create_company(&Company::new("Acme Trading Co", "PKR"))
            .unwrap()
            .id
            .unwrap();
        let other_company_id = org
            .create_company(&Company::new("Beta Traders", "PKR"))
            .unwrap()
            .id
            .unwrap();
        Fixture {
            service: WarehouseService::new(db.clone()),
            inventory: InventoryRepository::new(db),
            company_id,
            other_company_id,
        }
    }

    fn draft(company_id: i64, name: &str, lines: u32, positions: u32) -> WarehouseDraft {
        WarehouseDraft {
            id: None,
            company_id,
            name: name.into(),
            lines,
            positions,
            default_capacity: Some(dec!(10)),
        }
    }

    fn overview(f: &Fixture, id: i64) -> WarehouseOverview {
        f.service
            .list_overviews()
            .unwrap()
            .into_iter()
            .find(|o| o.warehouse.id == Some(id))
            .unwrap()
    }

    fn product(f: &Fixture, sku: &str, category: Option<&str>) -> i64 {
        let category_id = category.map(|name| {
            f.inventory
                .create_category(&Category::new(f.company_id, name))
                .unwrap()
                .id
                .unwrap()
        });
        let mut product = Product::new(f.company_id, sku, format!("Item {sku}"));
        product.category_id = category_id;
        f.inventory.create_product(&product).unwrap().id.unwrap()
    }

    fn bin_id(overview: &WarehouseOverview, label: &str) -> i64 {
        overview
            .bins
            .iter()
            .find(|b| b.bin.label == label)
            .unwrap()
            .bin
            .id
            .unwrap()
    }

    #[test]
    fn creating_a_warehouse_generates_lines_times_positions_bins() {
        let f = fixture();
        let id = f
            .service
            .save_warehouse(&draft(f.company_id, "Main", 3, 4))
            .unwrap();
        let overview = overview(&f, id);
        assert_eq!(overview.total_bins(), 12);
        assert_eq!(overview.bins[0].bin.label, "L1-P1");
        assert_eq!(overview.bins[11].bin.label, "L3-P4");
        assert_eq!(overview.bins[0].bin.capacity, Some(dec!(10)));
        assert_eq!(overview.warehouse.code, "WH-1");
        assert_eq!(overview.company_name, "Acme Trading Co");
        assert_eq!(overview.layout.unwrap().positions, 4);
    }

    #[test]
    fn names_are_unique_per_company_but_may_repeat_across_companies() {
        let f = fixture();
        f.service
            .save_warehouse(&draft(f.company_id, "Main", 1, 1))
            .unwrap();
        let clash = f
            .service
            .save_warehouse(&draft(f.company_id, " main ", 1, 1));
        assert!(matches!(clash, Err(AppError::Validation(_))));
        let elsewhere = f
            .service
            .save_warehouse(&draft(f.other_company_id, "Main", 1, 1));
        assert!(elsewhere.is_ok());
        assert!(f
            .service
            .save_warehouse(&draft(f.company_id, "Zero", 0, 1))
            .is_err());
    }

    #[test]
    fn growing_adds_bins_and_shrinking_cannot_remove_bins_with_stock() {
        let f = fixture();
        let id = f
            .service
            .save_warehouse(&draft(f.company_id, "Main", 2, 2))
            .unwrap();
        let item = product(&f, "TV-1", Some("Electronics"));
        let held = bin_id(&overview(&f, id), "L2-P2");
        f.service.place_stock(held, item, dec!(4)).unwrap();

        let shrink = WarehouseDraft {
            id: Some(id),
            ..draft(f.company_id, "Main", 1, 2)
        };
        let blocked = f.service.save_warehouse(&shrink);
        assert!(blocked.unwrap_err().to_string().contains("L2-P2"));
        assert_eq!(overview(&f, id).total_bins(), 4);

        let grow = WarehouseDraft {
            id: Some(id),
            default_capacity: Some(dec!(20)),
            ..draft(f.company_id, "Main Store", 3, 2)
        };
        f.service.save_warehouse(&grow).unwrap();
        let grown = overview(&f, id);
        assert_eq!(grown.total_bins(), 6);
        assert_eq!(grown.warehouse.name, "Main Store");
        assert!(grown.bins.iter().all(|b| b.bin.capacity == Some(dec!(20))));

        f.service.place_stock(held, item, dec!(0)).unwrap();
        let shrink_empty = WarehouseDraft {
            id: Some(id),
            ..draft(f.company_id, "Main Store", 1, 1)
        };
        f.service.save_warehouse(&shrink_empty).unwrap();
        assert_eq!(overview(&f, id).total_bins(), 1);
    }

    #[test]
    fn fullness_follows_quantity_against_capacity() {
        let f = fixture();
        let mut no_capacity = draft(f.company_id, "Open", 1, 1);
        no_capacity.default_capacity = None;
        let open = f.service.save_warehouse(&no_capacity).unwrap();
        let id = f
            .service
            .save_warehouse(&draft(f.company_id, "Main", 1, 3))
            .unwrap();
        let item = product(&f, "TV-1", Some("Electronics"));
        let warehouse = overview(&f, id);
        f.service
            .place_stock(bin_id(&warehouse, "L1-P2"), item, dec!(4))
            .unwrap();
        f.service
            .place_stock(bin_id(&warehouse, "L1-P3"), item, dec!(10))
            .unwrap();
        f.service
            .place_stock(bin_id(&overview(&f, open), "L1-P1"), item, dec!(500))
            .unwrap();

        let warehouse = overview(&f, id);
        let levels: Vec<Fullness> = warehouse.bins.iter().map(|b| b.fullness()).collect();
        assert_eq!(
            levels,
            vec![Fullness::Empty, Fullness::NotFull, Fullness::Full]
        );
        assert_eq!(warehouse.full_bins(), 1);
        assert_eq!(warehouse.available_capacity(), dec!(16));
        let open = overview(&f, open);
        assert_eq!(open.bins[0].fullness(), Fullness::NotFull);
        assert_eq!(open.utilization(), None);
    }

    #[test]
    fn contents_group_by_category_and_never_mix_in_another_warehouse() {
        let f = fixture();
        let main = f
            .service
            .save_warehouse(&draft(f.company_id, "Main", 1, 2))
            .unwrap();
        let other = f
            .service
            .save_warehouse(&draft(f.company_id, "Other", 1, 1))
            .unwrap();
        let tv = product(&f, "TV-1", Some("Electronics"));
        let rice = product(&f, "RICE-1", Some("Food"));
        let loose = product(&f, "BOX-1", None);
        let warehouse = overview(&f, main);
        let first = bin_id(&warehouse, "L1-P1");
        f.service.place_stock(first, tv, dec!(3)).unwrap();
        f.service.place_stock(first, rice, dec!(2)).unwrap();
        f.service
            .place_stock(bin_id(&warehouse, "L1-P2"), tv, dec!(5))
            .unwrap();
        f.service
            .place_stock(bin_id(&overview(&f, other), "L1-P1"), loose, dec!(99))
            .unwrap();
        // Setting a quantity again replaces it rather than adding.
        f.service.place_stock(first, rice, dec!(6)).unwrap();

        let warehouse = overview(&f, main);
        let groups = warehouse.bins[0].by_category();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "Electronics");
        assert_eq!(groups[1].1[0].quantity, dec!(6));
        let totals = warehouse.category_totals();
        assert_eq!(totals.len(), 2);
        assert_eq!(totals[0].quantity, dec!(8));
        assert_eq!(totals[0].bins, vec!["L1-P1", "L1-P2"]);
        assert_eq!(warehouse.total_quantity(), dec!(14));
        assert!(!totals.iter().any(|t| t.category == UNCATEGORIZED));
        let other = overview(&f, other);
        assert_eq!(other.category_totals()[0].category, UNCATEGORIZED);
        assert_eq!(other.total_quantity(), dec!(99));
    }

    #[test]
    fn an_item_of_another_company_cannot_be_placed_and_quantities_must_not_be_negative() {
        let f = fixture();
        let id = f
            .service
            .save_warehouse(&draft(f.company_id, "Main", 1, 1))
            .unwrap();
        let foreign = f
            .inventory
            .create_product(&Product::new(f.other_company_id, "X-1", "Foreign"))
            .unwrap()
            .id
            .unwrap();
        let bin = bin_id(&overview(&f, id), "L1-P1");
        assert!(f.service.place_stock(bin, foreign, dec!(1)).is_err());
        let item = product(&f, "TV-1", None);
        assert!(f.service.place_stock(bin, item, dec!(-1)).is_err());
    }

    #[test]
    fn removing_deletes_an_unused_warehouse_and_archives_one_with_stock() {
        let f = fixture();
        let unused = f
            .service
            .save_warehouse(&draft(f.company_id, "Unused", 2, 2))
            .unwrap();
        assert_eq!(
            f.service.remove_warehouse(f.company_id, unused).unwrap(),
            Removal::Deleted
        );
        let stocked = f
            .service
            .save_warehouse(&draft(f.company_id, "Stocked", 1, 1))
            .unwrap();
        let item = product(&f, "TV-1", None);
        f.service
            .place_stock(bin_id(&overview(&f, stocked), "L1-P1"), item, dec!(1))
            .unwrap();
        assert_eq!(
            f.service.remove_warehouse(f.company_id, stocked).unwrap(),
            Removal::Archived
        );
        let overviews = f.service.list_overviews().unwrap();
        assert_eq!(overviews.len(), 1);
        assert!(!overviews[0].warehouse.is_active);
    }

    #[test]
    fn a_warehouse_from_before_the_layout_migration_loads_without_a_grid() {
        let f = fixture();
        let legacy = f
            .inventory
            .create_warehouse(&Warehouse::new(f.company_id, "Legacy", "OLD"))
            .unwrap();
        let overview = overview(&f, legacy.id.unwrap());
        assert!(overview.layout.is_none());
        assert_eq!(overview.total_bins(), 0);
        assert_eq!(overview.fullness(), Fullness::Empty);
    }
}
