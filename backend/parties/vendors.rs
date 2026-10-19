//! A vendor: who bills (Development Order step 10) and payables (this
//! step) are owed to.

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::reference_code;
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

pub const VENDOR_TYPES: [&str; 5] = [
    "Supplier",
    "Contractor",
    "Distributor",
    "Manufacturer",
    "Other",
];

#[derive(Debug, Clone, PartialEq)]
pub struct Vendor {
    pub id: Option<i64>,
    pub company_id: i64,
    pub name: String,
    pub company_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub tax_id: Option<String>,
    pub vendor_type: String,
    pub payment_terms_days: i32,
    pub is_active: bool,
    /// Auto-generated, collision-safe internal reference id (`VEND-
    /// YYYYMMDDHHMMSSmmm`) -- see `core::reference_code`. `None` only
    /// for vendors created before this field existed; every vendor
    /// created through `Vendor::new` gets one automatically.
    pub reference_code: Option<String>,
}

impl Vendor {
    pub fn new(company_id: i64, name: impl Into<String>) -> Self {
        Self {
            id: None,
            company_id,
            name: name.into(),
            company_name: None,
            email: None,
            phone: None,
            tax_id: None,
            vendor_type: "Other".into(),
            payment_terms_days: 30,
            is_active: true,
            reference_code: Some(reference_code::generate_code("VEND")),
        }
    }
}

impl Validate for Vendor {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "vendor must reference a valid company".into(),
            ));
        }
        require_non_empty("vendor name", &self.name)?;
        require_max_length("vendor name", &self.name, 200)?;
        if let Some(company_name) = &self.company_name {
            require_max_length("company name", company_name, 200)?;
        }
        if self.payment_terms_days < 0 {
            return Err(AppError::Validation(
                "payment terms (days) must not be negative".into(),
            ));
        }
        if !VENDOR_TYPES.contains(&self.vendor_type.as_str()) {
            return Err(AppError::Validation("vendor type is not supported".into()));
        }
        if let Some(email) = &self.email {
            if !email.contains('@') {
                return Err(AppError::Validation(
                    "vendor email is not a valid email address".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_blank_name() {
        let vendor = Vendor::new(1, "");
        assert!(vendor.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_vendor() {
        let vendor = Vendor::new(1, "Acme Supplies Ltd");
        assert!(vendor.validate().is_ok());
    }

    #[test]
    fn validate_rejects_an_unsupported_vendor_type() {
        let mut vendor = Vendor::new(1, "Acme Supplies Ltd");
        vendor.vendor_type = "All".into();
        assert!(vendor.validate().is_err());
    }
}
