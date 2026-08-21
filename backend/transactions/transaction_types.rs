//! What kind of business event produced a transaction. Every domain that
//! posts through the transaction engine (Architecture Rules, Section 3:
//! sales, purchases, POS, expenses, banking, inventory, payroll) tags its
//! postings with one of these, plus a manual journal entry and a reversal.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransactionType {
    ManualJournal,
    OpeningBalance,
    SalesInvoice,
    PurchaseBill,
    CustomerPayment,
    VendorPayment,
    InventoryAdjustment,
    BankTransaction,
    Expense,
    Payroll,
    PosSale,
    Reversal,
}

impl TransactionType {
    pub fn as_str(self) -> &'static str {
        match self {
            TransactionType::ManualJournal => "manual_journal",
            TransactionType::OpeningBalance => "opening_balance",
            TransactionType::SalesInvoice => "sales_invoice",
            TransactionType::PurchaseBill => "purchase_bill",
            TransactionType::CustomerPayment => "customer_payment",
            TransactionType::VendorPayment => "vendor_payment",
            TransactionType::InventoryAdjustment => "inventory_adjustment",
            TransactionType::BankTransaction => "bank_transaction",
            TransactionType::Expense => "expense",
            TransactionType::Payroll => "payroll",
            TransactionType::PosSale => "pos_sale",
            TransactionType::Reversal => "reversal",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "manual_journal" => Some(TransactionType::ManualJournal),
            "opening_balance" => Some(TransactionType::OpeningBalance),
            "sales_invoice" => Some(TransactionType::SalesInvoice),
            "purchase_bill" => Some(TransactionType::PurchaseBill),
            "customer_payment" => Some(TransactionType::CustomerPayment),
            "vendor_payment" => Some(TransactionType::VendorPayment),
            "inventory_adjustment" => Some(TransactionType::InventoryAdjustment),
            "bank_transaction" => Some(TransactionType::BankTransaction),
            "expense" => Some(TransactionType::Expense),
            "payroll" => Some(TransactionType::Payroll),
            "pos_sale" => Some(TransactionType::PosSale),
            "reversal" => Some(TransactionType::Reversal),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: &[TransactionType] = &[
        TransactionType::ManualJournal,
        TransactionType::OpeningBalance,
        TransactionType::SalesInvoice,
        TransactionType::PurchaseBill,
        TransactionType::CustomerPayment,
        TransactionType::VendorPayment,
        TransactionType::InventoryAdjustment,
        TransactionType::BankTransaction,
        TransactionType::Expense,
        TransactionType::Payroll,
        TransactionType::PosSale,
        TransactionType::Reversal,
    ];

    #[test]
    fn as_str_and_parse_round_trip_for_every_variant() {
        for transaction_type in ALL {
            assert_eq!(TransactionType::parse(transaction_type.as_str()), Some(*transaction_type));
        }
    }

    #[test]
    fn parse_rejects_unknown_values() {
        assert_eq!(TransactionType::parse("not-a-type"), None);
    }
}
