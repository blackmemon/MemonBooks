//! Persistence for warehouse layouts, their bins, and the stock placed in
//! each bin. The warehouses, products and categories themselves stay in
//! `InventoryRepository`'s tables; this only adds the grid on top of them.

use rusqlite::{params, Connection, OptionalExtension};
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::warehouse_layout::layout::{
    bin_label, Bin, BinContents, BinItem, WarehouseLayout, UNCATEGORIZED,
};
use crate::database::database::{Database, Migration};

pub const MIGRATIONS: &[Migration] = &[Migration {
    // 1810: after inventory (1100, 1600), which owns the warehouses and
    // products these reference. New tables only -- existing warehouses
    // simply have no layout until one is saved.
    version: 1810,
    name: "warehouse_layout_schema",
    sql: "
        CREATE TABLE warehouse_layouts (
            warehouse_id      INTEGER PRIMARY KEY REFERENCES warehouses(id),
            lines             INTEGER NOT NULL,
            positions         INTEGER NOT NULL,
            default_capacity  TEXT
        );

        CREATE TABLE warehouse_bins (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            warehouse_id  INTEGER NOT NULL REFERENCES warehouses(id),
            line          INTEGER NOT NULL,
            position      INTEGER NOT NULL,
            label         TEXT NOT NULL,
            capacity      TEXT,
            UNIQUE (warehouse_id, line, position)
        );

        CREATE TABLE bin_placements (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            bin_id      INTEGER NOT NULL REFERENCES warehouse_bins(id),
            product_id  INTEGER NOT NULL REFERENCES products(id),
            quantity    TEXT NOT NULL,
            UNIQUE (bin_id, product_id)
        );
    ",
}];

#[derive(Clone)]
pub struct WarehouseLayoutRepository {
    db: Database,
}

impl WarehouseLayoutRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn get_layout(&self, warehouse_id: i64) -> AppResult<Option<WarehouseLayout>> {
        self.db
            .with_connection(|conn| layout_of(conn, warehouse_id))
    }

    /// Saves the layout and brings the bins in line with it: missing bins
    /// are created, bins outside the new size removed -- unless one of
    /// them holds stock, which rejects the whole change. Bins still on the
    /// old default capacity move to the new one.
    pub fn save_layout(&self, layout: &WarehouseLayout) -> AppResult<()> {
        layout.validate()?;
        self.db.with_transaction(|tx| {
            let previous = layout_of(tx, layout.warehouse_id)?;
            let bins = bins_of(tx, layout.warehouse_id)?;
            let removed: Vec<&Bin> = bins
                .iter()
                .filter(|bin| !layout.holds(bin.line, bin.position))
                .collect();
            let mut blocked = Vec::new();
            for bin in &removed {
                if holds_stock(tx, bin.id.unwrap_or_default())? {
                    blocked.push(bin.label.clone());
                }
            }
            if !blocked.is_empty() {
                return Err(AppError::Validation(format!(
                    "cannot remove bins that hold stock: {}",
                    blocked.join(", ")
                )));
            }
            for bin in removed {
                tx.execute("DELETE FROM warehouse_bins WHERE id = ?1", params![bin.id])?;
            }
            let old_default = previous.as_ref().and_then(|p| p.default_capacity);
            for bin in bins.iter().filter(|b| layout.holds(b.line, b.position)) {
                if bin.capacity == old_default && old_default != layout.default_capacity {
                    tx.execute(
                        "UPDATE warehouse_bins SET capacity = ?1 WHERE id = ?2",
                        params![layout.default_capacity.map(|c| c.to_string()), bin.id],
                    )?;
                }
            }
            for line in 1..=layout.lines {
                for position in 1..=layout.positions {
                    if !bins.iter().any(|b| b.line == line && b.position == position) {
                        tx.execute(
                            "INSERT INTO warehouse_bins (warehouse_id, line, position, label, capacity) VALUES (?1, ?2, ?3, ?4, ?5)",
                            params![
                                layout.warehouse_id,
                                line,
                                position,
                                bin_label(line, position),
                                layout.default_capacity.map(|c| c.to_string()),
                            ],
                        )?;
                    }
                }
            }
            tx.execute(
                "INSERT INTO warehouse_layouts (warehouse_id, lines, positions, default_capacity) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (warehouse_id) DO UPDATE SET lines = excluded.lines, positions = excluded.positions, default_capacity = excluded.default_capacity",
                params![
                    layout.warehouse_id,
                    layout.lines,
                    layout.positions,
                    layout.default_capacity.map(|c| c.to_string()),
                ],
            )?;
            Ok(())
        })
    }

    /// Every bin of the warehouse with what it holds, by line and position.
    pub fn bins(&self, warehouse_id: i64) -> AppResult<Vec<BinContents>> {
        self.db.with_connection(|conn| {
            let bins = bins_of(conn, warehouse_id)?;
            let mut stmt = conn.prepare(
                "SELECT bp.bin_id, p.id, p.name, p.category_id, c.name, bp.quantity
                 FROM bin_placements bp
                 JOIN warehouse_bins b ON b.id = bp.bin_id
                 JOIN products p ON p.id = bp.product_id
                 LEFT JOIN categories c ON c.id = p.category_id
                 WHERE b.warehouse_id = ?1
                 ORDER BY COALESCE(c.name, ?2), p.name",
            )?;
            let rows = stmt.query_map(params![warehouse_id, UNCATEGORIZED], |r| {
                let quantity: String = r.get(5)?;
                Ok((
                    r.get::<_, i64>(0)?,
                    BinItem {
                        product_id: r.get(1)?,
                        product_name: r.get(2)?,
                        category_id: r.get(3)?,
                        category: r
                            .get::<_, Option<String>>(4)?
                            .unwrap_or_else(|| UNCATEGORIZED.into()),
                        quantity: quantity.parse().unwrap_or(Decimal::ZERO),
                    },
                ))
            })?;
            let mut contents: Vec<BinContents> = bins
                .into_iter()
                .map(|bin| BinContents {
                    bin,
                    items: Vec::new(),
                })
                .collect();
            for row in rows {
                let (bin_id, item) = row?;
                if let Some(target) = contents.iter_mut().find(|c| c.bin.id == Some(bin_id)) {
                    target.items.push(item);
                }
            }
            Ok(contents)
        })
    }

    /// Sets how much of a product a bin holds; zero removes it. The
    /// product must belong to the bin's warehouse's company.
    pub fn set_placement(&self, bin_id: i64, product_id: i64, quantity: Decimal) -> AppResult<()> {
        if quantity.is_sign_negative() {
            return Err(AppError::Validation("quantity must not be negative".into()));
        }
        self.db.with_connection(|conn| {
            let same_company: Option<bool> = conn
                .query_row(
                    "SELECT w.company_id = p.company_id
                     FROM warehouse_bins b JOIN warehouses w ON w.id = b.warehouse_id, products p
                     WHERE b.id = ?1 AND p.id = ?2",
                    params![bin_id, product_id],
                    |r| r.get(0),
                )
                .optional()?;
            match same_company {
                None => return Err(AppError::NotFound("bin or item not found".into())),
                Some(false) => {
                    return Err(AppError::Validation(
                        "the item belongs to another company than the warehouse".into(),
                    ))
                }
                Some(true) => {}
            }
            if quantity.is_zero() {
                conn.execute(
                    "DELETE FROM bin_placements WHERE bin_id = ?1 AND product_id = ?2",
                    params![bin_id, product_id],
                )?;
            } else {
                conn.execute(
                    "INSERT INTO bin_placements (bin_id, product_id, quantity) VALUES (?1, ?2, ?3)
                     ON CONFLICT (bin_id, product_id) DO UPDATE SET quantity = excluded.quantity",
                    params![bin_id, product_id, quantity.to_string()],
                )?;
            }
            Ok(())
        })
    }

    /// Whether any bin of the warehouse holds stock.
    pub fn has_placements(&self, warehouse_id: i64) -> AppResult<bool> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT EXISTS (SELECT 1 FROM bin_placements bp JOIN warehouse_bins b ON b.id = bp.bin_id WHERE b.warehouse_id = ?1)",
                params![warehouse_id],
                |r| r.get(0),
            )
            .map_err(AppError::from)
        })
    }

    /// Removes the layout and its (empty) bins, so the warehouse itself
    /// can be deleted.
    pub fn delete_layout(&self, warehouse_id: i64) -> AppResult<()> {
        if self.has_placements(warehouse_id)? {
            return Err(AppError::Validation(
                "cannot remove a layout whose bins hold stock".into(),
            ));
        }
        self.db.with_transaction(|tx| {
            tx.execute(
                "DELETE FROM warehouse_bins WHERE warehouse_id = ?1",
                params![warehouse_id],
            )?;
            tx.execute(
                "DELETE FROM warehouse_layouts WHERE warehouse_id = ?1",
                params![warehouse_id],
            )?;
            Ok(())
        })
    }
}

fn layout_of(conn: &Connection, warehouse_id: i64) -> AppResult<Option<WarehouseLayout>> {
    conn.query_row(
        "SELECT warehouse_id, lines, positions, default_capacity FROM warehouse_layouts WHERE warehouse_id = ?1",
        params![warehouse_id],
        |r| {
            let capacity: Option<String> = r.get(3)?;
            Ok(WarehouseLayout {
                warehouse_id: r.get(0)?,
                lines: r.get(1)?,
                positions: r.get(2)?,
                default_capacity: capacity.and_then(|c| c.parse().ok()),
            })
        },
    )
    .optional()
    .map_err(AppError::from)
}

fn bins_of(conn: &Connection, warehouse_id: i64) -> AppResult<Vec<Bin>> {
    let mut stmt = conn.prepare(
        "SELECT id, warehouse_id, line, position, label, capacity FROM warehouse_bins
         WHERE warehouse_id = ?1 ORDER BY line, position",
    )?;
    let rows = stmt.query_map(params![warehouse_id], |r| {
        let capacity: Option<String> = r.get(5)?;
        Ok(Bin {
            id: Some(r.get(0)?),
            warehouse_id: r.get(1)?,
            line: r.get(2)?,
            position: r.get(3)?,
            label: r.get(4)?,
            capacity: capacity.and_then(|c| c.parse().ok()),
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

fn holds_stock(conn: &Connection, bin_id: i64) -> AppResult<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM bin_placements WHERE bin_id = ?1)",
        params![bin_id],
        |r| r.get(0),
    )
    .map_err(AppError::from)
}
