//! A warehouse's bin grid: the warehouse (the big box) is divided into
//! numbered lines, each holding numbered positions; every (line,
//! position) is a bin that stock is placed into. Fullness and the
//! per-category totals are derived from the placements, never stored.

use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::inventory::warehouses::Warehouse;

/// Largest grid a warehouse may be given, per side.
pub const MAX_LINES: u32 = 200;
pub const MAX_POSITIONS: u32 = 200;
/// The category shown for a product that has none.
pub const UNCATEGORIZED: &str = "Uncategorized";

/// `L3-P5`: the bin at line 3, position 5.
pub fn bin_label(line: u32, position: u32) -> String {
    format!("L{line}-P{position}")
}

#[derive(Debug, Clone, PartialEq)]
pub struct WarehouseLayout {
    pub warehouse_id: i64,
    pub lines: u32,
    pub positions: u32,
    /// The capacity new bins get; `None` leaves them without one.
    pub default_capacity: Option<Decimal>,
}

impl WarehouseLayout {
    pub fn new(
        warehouse_id: i64,
        lines: u32,
        positions: u32,
        default_capacity: Option<Decimal>,
    ) -> Self {
        Self {
            warehouse_id,
            lines,
            positions,
            default_capacity,
        }
    }

    pub fn holds(&self, line: u32, position: u32) -> bool {
        (1..=self.lines).contains(&line) && (1..=self.positions).contains(&position)
    }
}

impl Validate for WarehouseLayout {
    fn validate(&self) -> AppResult<()> {
        if !(1..=MAX_LINES).contains(&self.lines) {
            return Err(AppError::Validation(format!(
                "lines must be between 1 and {MAX_LINES}"
            )));
        }
        if !(1..=MAX_POSITIONS).contains(&self.positions) {
            return Err(AppError::Validation(format!(
                "positions per line must be between 1 and {MAX_POSITIONS}"
            )));
        }
        if self.default_capacity.is_some_and(|c| c <= Decimal::ZERO) {
            return Err(AppError::Validation(
                "default bin capacity must be positive".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bin {
    pub id: Option<i64>,
    pub warehouse_id: i64,
    pub line: u32,
    pub position: u32,
    pub label: String,
    pub capacity: Option<Decimal>,
}

/// How much of a bin (or warehouse) is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fullness {
    Empty,
    NotFull,
    Full,
}

impl Fullness {
    /// Empty when nothing is held; Full at or above the capacity; Not Full
    /// otherwise -- including stock held where no capacity is set.
    pub fn of(used: Decimal, capacity: Option<Decimal>) -> Self {
        if used <= Decimal::ZERO {
            Fullness::Empty
        } else if capacity.is_some_and(|capacity| used >= capacity) {
            Fullness::Full
        } else {
            Fullness::NotFull
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Fullness::Empty => "Empty",
            Fullness::NotFull => "Not Full",
            Fullness::Full => "Full",
        }
    }
}

pub const FULLNESS_LEVELS: [Fullness; 3] = [Fullness::Full, Fullness::Empty, Fullness::NotFull];

/// One product held in a bin.
#[derive(Debug, Clone, PartialEq)]
pub struct BinItem {
    pub product_id: i64,
    pub product_name: String,
    pub category_id: Option<i64>,
    pub category: String,
    pub quantity: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinContents {
    pub bin: Bin,
    /// Sorted by category, then product name.
    pub items: Vec<BinItem>,
}

impl BinContents {
    pub fn used(&self) -> Decimal {
        self.items.iter().map(|item| item.quantity).sum()
    }

    pub fn fullness(&self) -> Fullness {
        Fullness::of(self.used(), self.bin.capacity)
    }

    /// Room left, when the bin has a capacity (never below zero).
    pub fn free(&self) -> Option<Decimal> {
        self.bin
            .capacity
            .map(|capacity| (capacity - self.used()).max(Decimal::ZERO))
    }

    pub fn holds_category(&self, category_id: Option<i64>) -> bool {
        self.items
            .iter()
            .any(|item| item.category_id == category_id)
    }

    /// The items grouped by category, in category order.
    pub fn by_category(&self) -> Vec<(&str, Vec<&BinItem>)> {
        let mut groups: Vec<(&str, Vec<&BinItem>)> = Vec::new();
        for item in &self.items {
            match groups.iter_mut().find(|(name, _)| *name == item.category) {
                Some((_, items)) => items.push(item),
                None => groups.push((&item.category, vec![item])),
            }
        }
        groups
    }
}

/// One category across a warehouse: its total and the bins holding it.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryTotal {
    pub category_id: Option<i64>,
    pub category: String,
    pub quantity: Decimal,
    pub bins: Vec<String>,
}

/// A warehouse with its company, layout and every bin's contents.
#[derive(Debug, Clone, PartialEq)]
pub struct WarehouseOverview {
    pub warehouse: Warehouse,
    pub company_name: String,
    /// `None` for a warehouse that has not been given a grid yet.
    pub layout: Option<WarehouseLayout>,
    /// Ordered by line, then position.
    pub bins: Vec<BinContents>,
}

impl WarehouseOverview {
    pub fn total_bins(&self) -> usize {
        self.bins.len()
    }

    pub fn used_bins(&self) -> usize {
        self.bins.iter().filter(|b| !b.items.is_empty()).count()
    }

    pub fn empty_bins(&self) -> usize {
        self.total_bins() - self.used_bins()
    }

    pub fn full_bins(&self) -> usize {
        self.bins
            .iter()
            .filter(|b| b.fullness() == Fullness::Full)
            .count()
    }

    pub fn total_quantity(&self) -> Decimal {
        self.bins.iter().map(BinContents::used).sum()
    }

    /// Capacity of the bins that have one.
    pub fn total_capacity(&self) -> Decimal {
        self.bins.iter().filter_map(|b| b.bin.capacity).sum()
    }

    /// Quantity held in the bins that have a capacity.
    pub fn used_capacity(&self) -> Decimal {
        self.bins
            .iter()
            .filter(|b| b.bin.capacity.is_some())
            .map(BinContents::used)
            .sum()
    }

    pub fn available_capacity(&self) -> Decimal {
        self.bins.iter().filter_map(BinContents::free).sum()
    }

    /// Used / total capacity, `None` when no bin has a capacity.
    pub fn utilization(&self) -> Option<Decimal> {
        let total = self.total_capacity();
        (total > Decimal::ZERO).then(|| self.used_capacity() / total)
    }

    pub fn fullness(&self) -> Fullness {
        let capacity = (self.total_capacity() > Decimal::ZERO
            && self.bins.iter().all(|b| b.bin.capacity.is_some()))
        .then(|| self.total_capacity());
        Fullness::of(self.total_quantity(), capacity)
    }

    pub fn has_stock(&self) -> bool {
        self.total_quantity() > Decimal::ZERO
    }

    /// Every category held in the warehouse, by name.
    pub fn category_totals(&self) -> Vec<CategoryTotal> {
        let mut totals: Vec<CategoryTotal> = Vec::new();
        for contents in &self.bins {
            for item in &contents.items {
                let index = match totals
                    .iter()
                    .position(|t| t.category_id == item.category_id)
                {
                    Some(index) => index,
                    None => {
                        totals.push(CategoryTotal {
                            category_id: item.category_id,
                            category: item.category.clone(),
                            quantity: Decimal::ZERO,
                            bins: Vec::new(),
                        });
                        totals.len() - 1
                    }
                };
                let total = &mut totals[index];
                total.quantity += item.quantity;
                if !total.bins.contains(&contents.bin.label) {
                    total.bins.push(contents.bin.label.clone());
                }
            }
        }
        totals.sort_by(|a, b| a.category.cmp(&b.category));
        totals
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn bin(line: u32, position: u32, capacity: Option<Decimal>) -> Bin {
        Bin {
            id: Some(i64::from(line * 100 + position)),
            warehouse_id: 1,
            line,
            position,
            label: bin_label(line, position),
            capacity,
        }
    }

    fn item(category: Option<i64>, name: &str, quantity: Decimal) -> BinItem {
        BinItem {
            product_id: 1,
            product_name: name.into(),
            category_id: category,
            category: category.map_or(UNCATEGORIZED.into(), |id| format!("Cat {id}")),
            quantity,
        }
    }

    #[test]
    fn fullness_is_empty_partial_full_and_not_full_without_capacity() {
        assert_eq!(Fullness::of(dec!(0), Some(dec!(10))), Fullness::Empty);
        assert_eq!(Fullness::of(dec!(4), Some(dec!(10))), Fullness::NotFull);
        assert_eq!(Fullness::of(dec!(10), Some(dec!(10))), Fullness::Full);
        assert_eq!(Fullness::of(dec!(12), Some(dec!(10))), Fullness::Full);
        assert_eq!(Fullness::of(dec!(4), None), Fullness::NotFull);
        assert_eq!(Fullness::of(dec!(0), None), Fullness::Empty);
    }

    #[test]
    fn layout_validation_rejects_zero_sizes_and_non_positive_capacity() {
        assert!(WarehouseLayout::new(1, 0, 5, None).validate().is_err());
        assert!(WarehouseLayout::new(1, 5, 0, None).validate().is_err());
        assert!(WarehouseLayout::new(1, 5, 5, Some(dec!(0)))
            .validate()
            .is_err());
        assert!(WarehouseLayout::new(1, MAX_LINES + 1, 5, None)
            .validate()
            .is_err());
        assert!(WarehouseLayout::new(1, 5, 5, Some(dec!(10)))
            .validate()
            .is_ok());
    }

    #[test]
    fn the_overview_counts_bins_and_groups_categories() {
        let overview = WarehouseOverview {
            warehouse: Warehouse::new(1, "Main", "WH-1"),
            company_name: "Acme".into(),
            layout: Some(WarehouseLayout::new(1, 1, 3, Some(dec!(10)))),
            bins: vec![
                BinContents {
                    bin: bin(1, 1, Some(dec!(10))),
                    items: vec![item(Some(1), "TV", dec!(10))],
                },
                BinContents {
                    bin: bin(1, 2, Some(dec!(10))),
                    items: vec![item(Some(1), "Radio", dec!(3)), item(None, "Box", dec!(1))],
                },
                BinContents {
                    bin: bin(1, 3, Some(dec!(10))),
                    items: vec![],
                },
            ],
        };
        assert_eq!(overview.total_bins(), 3);
        assert_eq!(overview.used_bins(), 2);
        assert_eq!(overview.empty_bins(), 1);
        assert_eq!(overview.full_bins(), 1);
        assert_eq!(overview.total_quantity(), dec!(14));
        assert_eq!(overview.available_capacity(), dec!(16));
        assert_eq!(overview.utilization(), Some(dec!(14) / dec!(30)));
        assert_eq!(overview.fullness(), Fullness::NotFull);
        let totals = overview.category_totals();
        assert_eq!(totals.len(), 2);
        assert_eq!(totals[0].category, "Cat 1");
        assert_eq!(totals[0].quantity, dec!(13));
        assert_eq!(totals[0].bins, vec!["L1-P1", "L1-P2"]);
        assert_eq!(totals[1].category, UNCATEGORIZED);
        assert_eq!(overview.bins[1].by_category().len(), 2);
        assert!(overview.bins[1].holds_category(None));
        assert_eq!(overview.bins[0].free(), Some(dec!(0)));
    }

    #[test]
    fn a_warehouse_without_capacities_has_no_utilization() {
        let overview = WarehouseOverview {
            warehouse: Warehouse::new(1, "Main", "WH-1"),
            company_name: "Acme".into(),
            layout: None,
            bins: vec![BinContents {
                bin: bin(1, 1, None),
                items: vec![item(Some(1), "TV", dec!(5))],
            }],
        };
        assert_eq!(overview.utilization(), None);
        assert_eq!(overview.fullness(), Fullness::NotFull);
        assert_eq!(overview.bins[0].free(), None);
    }
}
