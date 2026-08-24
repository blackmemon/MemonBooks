//! Persistence for the organization hierarchy (companies, branches,
//! departments) and the currency reference data that companies and
//! exchange rates depend on. Bundled into one repository because none of
//! these five entities is large enough to warrant its own file, and they
//! share a migration set that must apply together.
//!
//! Also HR/payroll (Development Order step 17): no hr_repository.rs
//! exists in the frozen repositories list, and an employee is
//! fundamentally an organizational entity (who works where), so it
//! lives here rather than in user_repository.rs (which stays scoped to
//! system login accounts -- most employees never need one).

use chrono::NaiveDate;
use rusqlite::{params, OptionalExtension, Row};

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::Validate;
use crate::backend::currency::currencies::Currency;
use crate::backend::currency::exchange_rates::ExchangeRate;
use crate::backend::hr::attendance::{AttendanceRecord, AttendanceStatus};
use crate::backend::hr::employees::Employee;
use crate::backend::hr::leave::{LeaveRequest, LeaveStatus, LeaveType};
use crate::backend::hr::payroll::PayrollRun;
use crate::backend::hr::salary::{SalaryComponent, SalaryStructure};
use crate::backend::organization::branches::Branch;
use crate::backend::organization::companies::Company;
use crate::backend::organization::departments::Department;
use crate::backend::repositories::accounting_repository::AccountingRepository;
use crate::backend::transactions::transaction::Transaction;
use crate::database::database::{Database, Migration};

// Migration versions are `<development order step> * 100 + <sequence
// within that step>`, so each step has headroom for more than one
// migration without renumbering anything that shipped earlier.
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 300,
    name: "organization_and_currency_schema",
    sql: "
        CREATE TABLE companies (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            name                TEXT NOT NULL,
            legal_name          TEXT,
            base_currency_code  TEXT NOT NULL,
            created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );

        CREATE TABLE branches (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id  INTEGER NOT NULL REFERENCES companies(id),
            name        TEXT NOT NULL,
            code        TEXT NOT NULL,
            address     TEXT,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            UNIQUE (company_id, code)
        );

        CREATE TABLE departments (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id  INTEGER NOT NULL REFERENCES companies(id),
            branch_id   INTEGER REFERENCES branches(id),
            name        TEXT NOT NULL,
            code        TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
            UNIQUE (company_id, code)
        );

        CREATE TABLE currencies (
            code            TEXT PRIMARY KEY,
            name            TEXT NOT NULL,
            symbol          TEXT NOT NULL,
            decimal_places  INTEGER NOT NULL
        );

        CREATE TABLE exchange_rates (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            from_currency  TEXT NOT NULL REFERENCES currencies(code),
            to_currency    TEXT NOT NULL REFERENCES currencies(code),
            rate           TEXT NOT NULL,
            as_of          TEXT NOT NULL,
            UNIQUE (from_currency, to_currency, as_of)
        );
    ",
}, Migration {
    // Above 600 (not just 400): employees references users(id) from
    // step 4's migration, and payroll_runs references transactions(id)
    // from step 6's -- both must already exist.
    version: 1700,
    name: "hr_and_payroll_schema",
    sql: "
        CREATE TABLE employees (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id          INTEGER NOT NULL REFERENCES companies(id),
            branch_id           INTEGER REFERENCES branches(id),
            department_id       INTEGER REFERENCES departments(id),
            user_id             INTEGER REFERENCES users(id),
            employee_code       TEXT NOT NULL,
            full_name           TEXT NOT NULL,
            email               TEXT,
            phone               TEXT,
            hire_date           TEXT NOT NULL,
            termination_date    TEXT,
            is_active           INTEGER NOT NULL DEFAULT 1,
            UNIQUE (company_id, employee_code)
        );

        CREATE TABLE salary_structures (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            employee_id     INTEGER NOT NULL REFERENCES employees(id),
            effective_date  TEXT NOT NULL
        );

        CREATE TABLE salary_components (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            salary_structure_id INTEGER NOT NULL REFERENCES salary_structures(id),
            name                TEXT NOT NULL,
            amount              TEXT NOT NULL,
            is_deduction        INTEGER NOT NULL
        );

        CREATE TABLE attendance_records (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            employee_id     INTEGER NOT NULL REFERENCES employees(id),
            date            TEXT NOT NULL,
            status          TEXT NOT NULL,
            hours_worked    TEXT,
            UNIQUE (employee_id, date)
        );

        CREATE TABLE leave_requests (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            employee_id     INTEGER NOT NULL REFERENCES employees(id),
            leave_type      TEXT NOT NULL,
            start_date      TEXT NOT NULL,
            end_date        TEXT NOT NULL,
            status          TEXT NOT NULL
        );

        CREATE TABLE payroll_runs (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            company_id          INTEGER NOT NULL REFERENCES companies(id),
            pay_period_start    TEXT NOT NULL,
            pay_period_end      TEXT NOT NULL,
            payment_date        TEXT NOT NULL,
            transaction_id      INTEGER REFERENCES transactions(id)
        );

        CREATE TABLE payroll_entries (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            payroll_run_id      INTEGER NOT NULL REFERENCES payroll_runs(id),
            employee_id         INTEGER NOT NULL REFERENCES employees(id),
            gross_pay           TEXT NOT NULL,
            total_deductions    TEXT NOT NULL
        );
    ",
}];

#[derive(Clone)]
pub struct OrganizationRepository {
    db: Database,
    accounting: AccountingRepository,
}

impl OrganizationRepository {
    pub fn new(db: Database) -> Self {
        let accounting = AccountingRepository::new(db.clone());
        Self { db, accounting }
    }

    // -- companies --------------------------------------------------

    pub fn create_company(&self, company: &Company) -> AppResult<Company> {
        company.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO companies (name, legal_name, base_currency_code) VALUES (?1, ?2, ?3)",
                params![company.name, company.legal_name, company.base_currency_code],
            )?;
            Ok(Company {
                id: Some(conn.last_insert_rowid()),
                ..company.clone()
            })
        })
    }

    pub fn get_company(&self, id: i64) -> AppResult<Company> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, name, legal_name, base_currency_code FROM companies WHERE id = ?1",
                params![id],
                map_company,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("company {id} not found")))
        })
    }

    pub fn list_companies(&self) -> AppResult<Vec<Company>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT id, name, legal_name, base_currency_code FROM companies ORDER BY name")?;
            let rows = stmt.query_map([], map_company)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_company(&self, company: &Company) -> AppResult<()> {
        company.validate()?;
        let id = company
            .id
            .ok_or_else(|| AppError::Validation("cannot update a company without an id".into()))?;
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE companies SET name = ?1, legal_name = ?2, base_currency_code = ?3 WHERE id = ?4",
                params![company.name, company.legal_name, company.base_currency_code, id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("company {id} not found")));
            }
            Ok(())
        })
    }

    // -- branches -----------------------------------------------------

    pub fn create_branch(&self, branch: &Branch) -> AppResult<Branch> {
        branch.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO branches (company_id, name, code, address) VALUES (?1, ?2, ?3, ?4)",
                params![branch.company_id, branch.name, branch.code, branch.address],
            )?;
            Ok(Branch {
                id: Some(conn.last_insert_rowid()),
                ..branch.clone()
            })
        })
    }

    pub fn get_branch(&self, id: i64) -> AppResult<Branch> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, name, code, address FROM branches WHERE id = ?1",
                params![id],
                map_branch,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("branch {id} not found")))
        })
    }

    pub fn list_branches_for_company(&self, company_id: i64) -> AppResult<Vec<Branch>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, name, code, address FROM branches WHERE company_id = ?1 ORDER BY name",
            )?;
            let rows = stmt.query_map(params![company_id], map_branch)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_branch(&self, branch: &Branch) -> AppResult<()> {
        branch.validate()?;
        let id = branch
            .id
            .ok_or_else(|| AppError::Validation("cannot update a branch without an id".into()))?;
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE branches SET company_id = ?1, name = ?2, code = ?3, address = ?4 WHERE id = ?5",
                params![branch.company_id, branch.name, branch.code, branch.address, id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("branch {id} not found")));
            }
            Ok(())
        })
    }

    // -- departments --------------------------------------------------

    pub fn create_department(&self, department: &Department) -> AppResult<Department> {
        department.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO departments (company_id, branch_id, name, code) VALUES (?1, ?2, ?3, ?4)",
                params![department.company_id, department.branch_id, department.name, department.code],
            )?;
            Ok(Department {
                id: Some(conn.last_insert_rowid()),
                ..department.clone()
            })
        })
    }

    pub fn get_department(&self, id: i64) -> AppResult<Department> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, branch_id, name, code FROM departments WHERE id = ?1",
                params![id],
                map_department,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("department {id} not found")))
        })
    }

    pub fn list_departments_for_company(&self, company_id: i64) -> AppResult<Vec<Department>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, branch_id, name, code FROM departments WHERE company_id = ?1 ORDER BY name",
            )?;
            let rows = stmt.query_map(params![company_id], map_department)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_department(&self, department: &Department) -> AppResult<()> {
        department.validate()?;
        let id = department
            .id
            .ok_or_else(|| AppError::Validation("cannot update a department without an id".into()))?;
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE departments SET company_id = ?1, branch_id = ?2, name = ?3, code = ?4 WHERE id = ?5",
                params![department.company_id, department.branch_id, department.name, department.code, id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("department {id} not found")));
            }
            Ok(())
        })
    }

    // -- currencies -----------------------------------------------------

    /// Currencies are keyed by their code, so creating one that already
    /// exists overwrites its display fields instead of erroring.
    pub fn upsert_currency(&self, currency: &Currency) -> AppResult<()> {
        currency.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO currencies (code, name, symbol, decimal_places) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (code) DO UPDATE SET name = excluded.name, symbol = excluded.symbol, decimal_places = excluded.decimal_places",
                params![currency.code, currency.name, currency.symbol, currency.decimal_places],
            )?;
            Ok(())
        })
    }

    pub fn get_currency(&self, code: &str) -> AppResult<Currency> {
        let code = code.to_ascii_uppercase();
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT code, name, symbol, decimal_places FROM currencies WHERE code = ?1",
                params![code],
                map_currency,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("currency {code} not found")))
        })
    }

    pub fn list_currencies(&self) -> AppResult<Vec<Currency>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT code, name, symbol, decimal_places FROM currencies ORDER BY code")?;
            let rows = stmt.query_map([], map_currency)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    // -- exchange rates -----------------------------------------------

    pub fn record_exchange_rate(&self, rate: &ExchangeRate) -> AppResult<ExchangeRate> {
        rate.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO exchange_rates (from_currency, to_currency, rate, as_of) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (from_currency, to_currency, as_of) DO UPDATE SET rate = excluded.rate",
                params![rate.from_currency, rate.to_currency, rate.rate.to_string(), rate.as_of],
            )?;
            let id: i64 = conn.query_row(
                "SELECT id FROM exchange_rates WHERE from_currency = ?1 AND to_currency = ?2 AND as_of = ?3",
                params![rate.from_currency, rate.to_currency, rate.as_of],
                |row| row.get(0),
            )?;
            Ok(ExchangeRate {
                id: Some(id),
                ..rate.clone()
            })
        })
    }

    /// The most recent rate on or before `as_of` for the given pair, if any.
    pub fn latest_exchange_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        as_of: NaiveDate,
    ) -> AppResult<Option<ExchangeRate>> {
        let from_currency = from_currency.to_ascii_uppercase();
        let to_currency = to_currency.to_ascii_uppercase();
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, from_currency, to_currency, rate, as_of FROM exchange_rates
                 WHERE from_currency = ?1 AND to_currency = ?2 AND as_of <= ?3
                 ORDER BY as_of DESC LIMIT 1",
                params![from_currency, to_currency, as_of],
                map_exchange_rate,
            )
            .optional()
            .map_err(AppError::from)
        })
    }

    // -- HR: employees ----------------------------------------------------

    pub fn create_employee(&self, employee: &Employee) -> AppResult<Employee> {
        employee.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO employees (company_id, branch_id, department_id, user_id, employee_code, full_name, email, phone, hire_date, termination_date, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    employee.company_id,
                    employee.branch_id,
                    employee.department_id,
                    employee.user_id,
                    employee.employee_code,
                    employee.full_name,
                    employee.email,
                    employee.phone,
                    employee.hire_date,
                    employee.termination_date,
                    employee.is_active,
                ],
            )?;
            Ok(Employee { id: Some(conn.last_insert_rowid()), ..employee.clone() })
        })
    }

    pub fn get_employee(&self, id: i64) -> AppResult<Employee> {
        self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, company_id, branch_id, department_id, user_id, employee_code, full_name, email, phone, hire_date, termination_date, is_active
                 FROM employees WHERE id = ?1",
                params![id],
                map_employee,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("employee {id} not found")))
        })
    }

    pub fn update_employee(&self, employee: &Employee) -> AppResult<()> {
        employee.validate()?;
        let id = employee.id.ok_or_else(|| AppError::Validation("cannot update an employee without an id".into()))?;
        self.db.with_connection(|conn| {
            let affected = conn.execute(
                "UPDATE employees SET branch_id = ?1, department_id = ?2, user_id = ?3, employee_code = ?4, full_name = ?5, email = ?6, phone = ?7, hire_date = ?8, termination_date = ?9, is_active = ?10
                 WHERE id = ?11",
                params![
                    employee.branch_id,
                    employee.department_id,
                    employee.user_id,
                    employee.employee_code,
                    employee.full_name,
                    employee.email,
                    employee.phone,
                    employee.hire_date,
                    employee.termination_date,
                    employee.is_active,
                    id,
                ],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("employee {id} not found")));
            }
            Ok(())
        })
    }

    /// Rejects with a clear message, rather than a raw SQLite one, when
    /// the employee is still referenced elsewhere -- salary structures,
    /// payroll run lines, and expense claims all reference an employee
    /// by id, and every one of those foreign keys is enforced.
    pub fn delete_employee(&self, id: i64) -> AppResult<()> {
        self.db
            .with_connection(|conn| {
                let affected = conn.execute("DELETE FROM employees WHERE id = ?1", params![id])?;
                if affected == 0 {
                    return Err(AppError::NotFound(format!("employee {id} not found")));
                }
                Ok(())
            })
            .map_err(|err| {
                if err.is_constraint_violation() {
                    AppError::Validation(format!("cannot delete employee {id}: it's still referenced by payroll, salary, or other records"))
                } else {
                    err
                }
            })
    }

    pub fn list_employees_for_company(&self, company_id: i64) -> AppResult<Vec<Employee>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, company_id, branch_id, department_id, user_id, employee_code, full_name, email, phone, hire_date, termination_date, is_active
                 FROM employees WHERE company_id = ?1 ORDER BY full_name",
            )?;
            let rows = stmt.query_map(params![company_id], map_employee)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    // -- HR: salary structures -------------------------------------------

    pub fn create_salary_structure(&self, structure: &SalaryStructure) -> AppResult<SalaryStructure> {
        structure.validate()?;
        self.db.with_transaction(|tx| {
            tx.execute(
                "INSERT INTO salary_structures (employee_id, effective_date) VALUES (?1, ?2)",
                params![structure.employee_id, structure.effective_date],
            )?;
            let structure_id = tx.last_insert_rowid();
            for component in &structure.components {
                tx.execute(
                    "INSERT INTO salary_components (salary_structure_id, name, amount, is_deduction) VALUES (?1, ?2, ?3, ?4)",
                    params![structure_id, component.name, component.amount.to_string(), component.is_deduction],
                )?;
            }
            Ok(structure_id)
        })
        .map(|id| SalaryStructure { id: Some(id), ..structure.clone() })
    }

    /// The most recent salary structure effective on or before `as_of`.
    pub fn current_salary_structure(&self, employee_id: i64, as_of: NaiveDate) -> AppResult<Option<SalaryStructure>> {
        let structure_id: Option<i64> = self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id FROM salary_structures WHERE employee_id = ?1 AND effective_date <= ?2 ORDER BY effective_date DESC LIMIT 1",
                params![employee_id, as_of],
                |row| row.get(0),
            )
            .optional()
            .map_err(AppError::from)
        })?;

        let Some(structure_id) = structure_id else {
            return Ok(None);
        };

        let effective_date: NaiveDate = self.db.with_connection(|conn| {
            conn.query_row("SELECT effective_date FROM salary_structures WHERE id = ?1", params![structure_id], |row| row.get(0))
                .map_err(AppError::from)
        })?;

        let components = self.db.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT name, amount, is_deduction FROM salary_components WHERE salary_structure_id = ?1")?;
            let rows = stmt.query_map(params![structure_id], map_salary_component)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })?;

        Ok(Some(SalaryStructure {
            id: Some(structure_id),
            employee_id,
            effective_date,
            components,
        }))
    }

    // -- HR: attendance and leave -----------------------------------------

    pub fn record_attendance(&self, record: &AttendanceRecord) -> AppResult<AttendanceRecord> {
        record.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO attendance_records (employee_id, date, status, hours_worked) VALUES (?1, ?2, ?3, ?4)",
                params![record.employee_id, record.date, record.status.as_str(), record.hours_worked.map(|h| h.to_string())],
            )?;
            Ok(AttendanceRecord { id: Some(conn.last_insert_rowid()), ..record.clone() })
        })
    }

    pub fn list_attendance_for_employee(&self, employee_id: i64) -> AppResult<Vec<AttendanceRecord>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, employee_id, date, status, hours_worked FROM attendance_records WHERE employee_id = ?1 ORDER BY date",
            )?;
            let rows = stmt.query_map(params![employee_id], map_attendance_record)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn create_leave_request(&self, request: &LeaveRequest) -> AppResult<LeaveRequest> {
        request.validate()?;
        self.db.with_connection(|conn| {
            conn.execute(
                "INSERT INTO leave_requests (employee_id, leave_type, start_date, end_date, status) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![request.employee_id, request.leave_type.as_str(), request.start_date, request.end_date, request.status.as_str()],
            )?;
            Ok(LeaveRequest { id: Some(conn.last_insert_rowid()), ..request.clone() })
        })
    }

    fn transition_leave_request(&self, id: i64, to: LeaveStatus) -> AppResult<LeaveRequest> {
        let request = self.db.with_connection(|conn| {
            conn.query_row(
                "SELECT id, employee_id, leave_type, start_date, end_date, status FROM leave_requests WHERE id = ?1",
                params![id],
                map_leave_request,
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound(format!("leave request {id} not found")))
        })?;
        if !request.status.can_transition_to(to) {
            return Err(AppError::Validation(format!(
                "cannot transition leave request from {} to {}",
                request.status.as_str(),
                to.as_str()
            )));
        }
        self.db.with_connection(|conn| {
            conn.execute("UPDATE leave_requests SET status = ?1 WHERE id = ?2", params![to.as_str(), id])?;
            Ok(())
        })?;
        Ok(LeaveRequest { status: to, ..request })
    }

    pub fn approve_leave_request(&self, id: i64) -> AppResult<LeaveRequest> {
        self.transition_leave_request(id, LeaveStatus::Approved)
    }

    pub fn reject_leave_request(&self, id: i64) -> AppResult<LeaveRequest> {
        self.transition_leave_request(id, LeaveStatus::Rejected)
    }

    // -- HR: payroll ------------------------------------------------------

    /// Posts the run's GL transaction and stores the run + its per-employee
    /// entries, atomically linked -- same pattern as `post_invoice`
    /// (Development Order step 10).
    pub fn post_payroll_run(&self, run: &PayrollRun, salary_expense_account_id: i64, deductions_payable_account_id: i64, net_pay_account_id: i64) -> AppResult<(PayrollRun, Transaction)> {
        run.validate()?;
        let transaction = run.build_transaction(salary_expense_account_id, deductions_payable_account_id, net_pay_account_id);
        let posted = self.accounting.post_transaction(&transaction)?;

        let run_id = self.db.with_transaction(|tx| {
            tx.execute(
                "INSERT INTO payroll_runs (company_id, pay_period_start, pay_period_end, payment_date, transaction_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![run.company_id, run.pay_period_start, run.pay_period_end, run.payment_date, posted.id],
            )?;
            let run_id = tx.last_insert_rowid();
            for entry in &run.entries {
                tx.execute(
                    "INSERT INTO payroll_entries (payroll_run_id, employee_id, gross_pay, total_deductions) VALUES (?1, ?2, ?3, ?4)",
                    params![run_id, entry.employee_id, entry.gross_pay.to_string(), entry.total_deductions.to_string()],
                )?;
            }
            Ok(run_id)
        })?;

        Ok((PayrollRun { id: Some(run_id), ..run.clone() }, posted))
    }
}

fn map_employee(row: &Row) -> rusqlite::Result<Employee> {
    Ok(Employee {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        branch_id: row.get(2)?,
        department_id: row.get(3)?,
        user_id: row.get(4)?,
        employee_code: row.get(5)?,
        full_name: row.get(6)?,
        email: row.get(7)?,
        phone: row.get(8)?,
        hire_date: row.get(9)?,
        termination_date: row.get(10)?,
        is_active: row.get(11)?,
    })
}

fn map_salary_component(row: &Row) -> rusqlite::Result<SalaryComponent> {
    let amount_text: String = row.get(1)?;
    Ok(SalaryComponent {
        name: row.get(0)?,
        amount: amount_text.parse().unwrap_or(rust_decimal::Decimal::ZERO),
        is_deduction: row.get(2)?,
    })
}

fn map_attendance_record(row: &Row) -> rusqlite::Result<AttendanceRecord> {
    let status_text: String = row.get(3)?;
    let status = AttendanceStatus::parse(&status_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, "invalid attendance status".into()))?;
    let hours_text: Option<String> = row.get(4)?;
    Ok(AttendanceRecord {
        id: Some(row.get(0)?),
        employee_id: row.get(1)?,
        date: row.get(2)?,
        status,
        hours_worked: hours_text.and_then(|h| h.parse().ok()),
    })
}

fn map_leave_request(row: &Row) -> rusqlite::Result<LeaveRequest> {
    let leave_type_text: String = row.get(2)?;
    let leave_type = LeaveType::parse(&leave_type_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, "invalid leave type".into()))?;
    let status_text: String = row.get(5)?;
    let status = LeaveStatus::parse(&status_text)
        .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, "invalid leave status".into()))?;
    Ok(LeaveRequest {
        id: Some(row.get(0)?),
        employee_id: row.get(1)?,
        leave_type,
        start_date: row.get(3)?,
        end_date: row.get(4)?,
        status,
    })
}

fn map_company(row: &Row) -> rusqlite::Result<Company> {
    Ok(Company {
        id: Some(row.get(0)?),
        name: row.get(1)?,
        legal_name: row.get(2)?,
        base_currency_code: row.get(3)?,
    })
}

fn map_branch(row: &Row) -> rusqlite::Result<Branch> {
    Ok(Branch {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        name: row.get(2)?,
        code: row.get(3)?,
        address: row.get(4)?,
    })
}

fn map_department(row: &Row) -> rusqlite::Result<Department> {
    Ok(Department {
        id: Some(row.get(0)?),
        company_id: row.get(1)?,
        branch_id: row.get(2)?,
        name: row.get(3)?,
        code: row.get(4)?,
    })
}

fn map_currency(row: &Row) -> rusqlite::Result<Currency> {
    let decimal_places: i64 = row.get(3)?;
    Ok(Currency {
        code: row.get(0)?,
        name: row.get(1)?,
        symbol: row.get(2)?,
        decimal_places: decimal_places as u32,
    })
}

fn map_exchange_rate(row: &Row) -> rusqlite::Result<ExchangeRate> {
    let rate_text: String = row.get(3)?;
    Ok(ExchangeRate {
        id: Some(row.get(0)?),
        from_currency: row.get(1)?,
        to_currency: row.get(2)?,
        rate: rate_text.parse().map_err(|_| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, "invalid decimal".into())
        })?,
        as_of: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn repo_with_schema() -> OrganizationRepository {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(crate::backend::repositories::user_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::repositories::accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();
        db.run_migrations(MIGRATIONS).unwrap();
        OrganizationRepository::new(db)
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn creates_and_fetches_a_company() {
        let repo = repo_with_schema();
        let created = repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();
        assert!(created.id.is_some());

        let fetched = repo.get_company(created.id.unwrap()).unwrap();
        assert_eq!(fetched.name, "Acme Trading Co");
        assert_eq!(fetched.base_currency_code, "USD");
    }

    #[test]
    fn rejects_invalid_company_before_hitting_the_database() {
        let repo = repo_with_schema();
        let result = repo.create_company(&Company::new("", "USD"));
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn getting_a_missing_company_returns_not_found() {
        let repo = repo_with_schema();
        assert!(matches!(repo.get_company(999), Err(AppError::NotFound(_))));
    }

    #[test]
    fn branch_and_department_round_trip_under_a_company() {
        let repo = repo_with_schema();
        let company = repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();
        let company_id = company.id.unwrap();

        let branch = repo
            .create_branch(&Branch::new(company_id, "Main Branch", "MAIN"))
            .unwrap();
        assert_eq!(repo.list_branches_for_company(company_id).unwrap().len(), 1);

        let mut department = Department::new(company_id, "Finance", "FIN");
        department.branch_id = branch.id;
        let department = repo.create_department(&department).unwrap();
        assert_eq!(department.branch_id, branch.id);
        assert_eq!(repo.list_departments_for_company(company_id).unwrap().len(), 1);
    }

    #[test]
    fn update_employee_persists_changed_fields() {
        let repo = repo_with_schema();
        let company_id = repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();
        let created = repo.create_employee(&Employee::new(company_id, "EMP-001", "Jane Doe", date(2026, 1, 1))).unwrap();

        repo.update_employee(&Employee { full_name: "Jane Smith".into(), ..created.clone() }).unwrap();

        assert_eq!(repo.get_employee(created.id.unwrap()).unwrap().full_name, "Jane Smith");
    }

    #[test]
    fn delete_employee_removes_an_unreferenced_employee() {
        let repo = repo_with_schema();
        let company_id = repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();
        let created = repo.create_employee(&Employee::new(company_id, "EMP-001", "Jane Doe", date(2026, 1, 1))).unwrap();

        repo.delete_employee(created.id.unwrap()).unwrap();

        assert!(repo.get_employee(created.id.unwrap()).is_err());
    }

    #[test]
    fn delete_employee_is_rejected_with_a_clear_message_when_still_referenced_by_a_salary_structure() {
        let repo = repo_with_schema();
        let company_id = repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();
        let employee = repo.create_employee(&Employee::new(company_id, "EMP-001", "Jane Doe", date(2026, 1, 1))).unwrap();
        let mut structure = SalaryStructure::new(employee.id.unwrap(), date(2026, 1, 1));
        structure.components.push(crate::backend::hr::salary::SalaryComponent::addition("Base Salary", dec!(1000)));
        repo.create_salary_structure(&structure).unwrap();

        let result = repo.delete_employee(employee.id.unwrap());

        assert!(matches!(result, Err(AppError::Validation(_))));
        assert!(result.unwrap_err().to_string().contains("still referenced"));
        assert!(repo.get_employee(employee.id.unwrap()).is_ok());
    }

    #[test]
    fn upserting_a_currency_twice_updates_rather_than_duplicates() {
        let repo = repo_with_schema();
        repo.upsert_currency(&Currency::new("USD", "US Dollar", "$", 2)).unwrap();
        repo.upsert_currency(&Currency::new("USD", "United States Dollar", "US$", 2))
            .unwrap();

        let currencies = repo.list_currencies().unwrap();
        assert_eq!(currencies.len(), 1);
        assert_eq!(currencies[0].name, "United States Dollar");
    }

    #[test]
    fn latest_exchange_rate_picks_the_most_recent_on_or_before_the_date() {
        let repo = repo_with_schema();
        repo.upsert_currency(&Currency::new("USD", "US Dollar", "$", 2)).unwrap();
        repo.upsert_currency(&Currency::new("PKR", "Pakistani Rupee", "Rs", 2)).unwrap();

        repo.record_exchange_rate(&ExchangeRate::new("USD", "PKR", dec!(278.0), date(2026, 1, 1)))
            .unwrap();
        repo.record_exchange_rate(&ExchangeRate::new("USD", "PKR", dec!(279.5), date(2026, 2, 1)))
            .unwrap();

        let rate = repo
            .latest_exchange_rate("USD", "PKR", date(2026, 3, 1))
            .unwrap()
            .expect("a rate should exist on or before 2026-03-01");
        assert_eq!(rate.rate, dec!(279.5));

        let rate = repo
            .latest_exchange_rate("USD", "PKR", date(2026, 1, 15))
            .unwrap()
            .expect("a rate should exist on or before 2026-01-15");
        assert_eq!(rate.rate, dec!(278.0));

        assert!(repo
            .latest_exchange_rate("USD", "PKR", date(2025, 12, 31))
            .unwrap()
            .is_none());
    }

    fn repo_with_company_and_accounts() -> (OrganizationRepository, i64, i64, i64, i64) {
        let repo = repo_with_schema();
        let company = repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap();
        let company_id = company.id.unwrap();

        let acc_repo = crate::backend::repositories::accounting_repository::AccountingRepository::new(repo.db.clone());
        acc_repo
            .create_fiscal_period(&crate::backend::accounting::fiscal_periods::FiscalPeriod::new(company_id, "January 2026", date(2026, 1, 1), date(2026, 1, 31)))
            .unwrap();
        let salary_expense = acc_repo
            .create_account(&crate::backend::accounting::chart_of_accounts::Account::new(company_id, "6200", "Salary Expense", crate::backend::accounting::accounting_rules::AccountType::Expense))
            .unwrap();
        let deductions_payable = acc_repo
            .create_account(&crate::backend::accounting::chart_of_accounts::Account::new(company_id, "2200", "Payroll Deductions Payable", crate::backend::accounting::accounting_rules::AccountType::Liability))
            .unwrap();
        let cash = acc_repo
            .create_account(&crate::backend::accounting::chart_of_accounts::Account::new(company_id, "1000", "Cash", crate::backend::accounting::accounting_rules::AccountType::Asset))
            .unwrap();

        (repo, company_id, salary_expense.id.unwrap(), deductions_payable.id.unwrap(), cash.id.unwrap())
    }

    #[test]
    fn employee_round_trips_and_lists_for_company() {
        let (repo, company_id, ..) = repo_with_company_and_accounts();
        repo.create_employee(&Employee::new(company_id, "EMP-1", "Jane Doe", date(2026, 1, 1))).unwrap();
        assert_eq!(repo.list_employees_for_company(company_id).unwrap().len(), 1);
    }

    #[test]
    fn current_salary_structure_picks_the_most_recent_effective_on_or_before_the_date() {
        let (repo, company_id, ..) = repo_with_company_and_accounts();
        let employee = repo.create_employee(&Employee::new(company_id, "EMP-1", "Jane Doe", date(2026, 1, 1))).unwrap();

        let mut old = SalaryStructure::new(employee.id.unwrap(), date(2026, 1, 1));
        old.components.push(SalaryComponent::addition("Base Salary", dec!(4000)));
        repo.create_salary_structure(&old).unwrap();

        let mut new_structure = SalaryStructure::new(employee.id.unwrap(), date(2026, 6, 1));
        new_structure.components.push(SalaryComponent::addition("Base Salary", dec!(5000)));
        repo.create_salary_structure(&new_structure).unwrap();

        let current = repo.current_salary_structure(employee.id.unwrap(), date(2026, 3, 1)).unwrap().unwrap();
        assert_eq!(current.gross_pay(), dec!(4000));

        let current = repo.current_salary_structure(employee.id.unwrap(), date(2026, 12, 31)).unwrap().unwrap();
        assert_eq!(current.gross_pay(), dec!(5000));
    }

    #[test]
    fn leave_request_lifecycle_submit_then_approve() {
        let (repo, company_id, ..) = repo_with_company_and_accounts();
        let employee = repo.create_employee(&Employee::new(company_id, "EMP-1", "Jane Doe", date(2026, 1, 1))).unwrap();

        let request = repo
            .create_leave_request(&LeaveRequest::new(employee.id.unwrap(), LeaveType::Annual, date(2026, 2, 1), date(2026, 2, 5)))
            .unwrap();
        assert_eq!(request.status, LeaveStatus::Requested);

        let approved = repo.approve_leave_request(request.id.unwrap()).unwrap();
        assert_eq!(approved.status, LeaveStatus::Approved);

        assert!(repo.reject_leave_request(approved.id.unwrap()).is_err());
    }

    #[test]
    fn posting_a_payroll_run_posts_the_transaction_and_stores_entries() {
        let (repo, company_id, salary_expense_id, deductions_payable_id, cash_id) = repo_with_company_and_accounts();
        let employee = repo.create_employee(&Employee::new(company_id, "EMP-1", "Jane Doe", date(2026, 1, 1))).unwrap();

        let mut run = PayrollRun::new(company_id, date(2026, 1, 1), date(2026, 1, 31), date(2026, 1, 31));
        run.entries.push(crate::backend::hr::payroll::PayrollEntry {
            employee_id: employee.id.unwrap(),
            gross_pay: dec!(6000),
            total_deductions: dec!(800),
        });

        let (recorded, transaction) = repo.post_payroll_run(&run, salary_expense_id, deductions_payable_id, cash_id).unwrap();
        assert!(transaction.is_balanced());
        assert!(recorded.id.is_some());

        let trial_balance = crate::backend::repositories::accounting_repository::AccountingRepository::new(repo.db.clone())
            .trial_balance_as_of(company_id, date(2026, 1, 31))
            .unwrap();
        assert!(trial_balance.is_balanced());
    }
}
