//! The auth domain's service layer: login and session concerns, for
//! `auth/login_view.rs` (Architecture Rules, Section 3). Not part of the
//! frozen `services/` file list in `PROJECT_SPEC.md` Section 2 -- that
//! list had no dedicated auth/user service, unlike every other domain.
//! Added here rather than folding login into `organization_service.rs`,
//! since authentication is its own concern, not organization data; see
//! `CHANGELOG.md` for the record of this deviation.
//!
//! Wraps `UserRepository` for lookup plus the free functions in
//! `core::auth::authentication` that decide the login outcome.

use rand::Rng;

use crate::backend::core::auth::authentication::{authenticate, verify_two_factor, AuthOutcome};
use crate::backend::core::auth::passwords::{hash_password, verify_password};
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::roles::Role;
use crate::backend::core::users::User;
use crate::backend::repositories::user_repository::UserRepository;
use crate::database::database::Database;

const USERS_MANAGE: &str = "users.manage";

/// Same scheme as `main.rs`'s `ensure_default_admin`: a 16-character
/// random alphanumeric string, shown to the administrator exactly once
/// (the caller gets it back as plaintext right after this call) since
/// only the hash is ever persisted.
fn generate_temporary_password() -> String {
    rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

#[derive(Clone)]
pub struct AuthService {
    repo: UserRepository,
}

impl AuthService {
    pub fn new(db: Database) -> Self {
        Self {
            repo: UserRepository::new(db),
        }
    }

    /// Every other active account, for the basic user-switcher --
    /// excludes `exclude_user_id` (the currently signed-in account)
    /// and anyone deactivated, since neither belongs in a "switch to"
    /// list.
    pub fn list_other_active_users(&self, exclude_user_id: i64) -> AppResult<Vec<User>> {
        Ok(self
            .repo
            .list_users()?
            .into_iter()
            .filter(|user| user.is_active && user.id != Some(exclude_user_id))
            .collect())
    }

    /// Step one of login: username + password. Looks the user up and
    /// delegates the outcome to `authenticate` -- see its doc comment
    /// for why "no such user" and "wrong password" share one error.
    pub fn login_with_credentials(&self, username: &str, password: &str) -> AppResult<AuthOutcome> {
        let user = self.repo.find_user_by_username(username)?;
        authenticate(user.as_ref(), password)
    }

    /// Step two of login, only reached when `login_with_credentials`
    /// returned `TwoFactorRequired`.
    pub fn login_with_two_factor_code(&self, user: &User, code: &str) -> AppResult<User> {
        verify_two_factor(user, code)
    }

    /// Verifies `current_password` against the stored hash, then hashes
    /// and stores `new_password` and clears `must_change_password` --
    /// the one path (forced or voluntary) for a user to change their
    /// own password. Requiring the current password even on the forced
    /// first-login change catches a mistyped assumption about what the
    /// temporary password was, not just a rubber-stamp confirmation.
    pub fn change_password(
        &self,
        user: &User,
        current_password: &str,
        new_password: &str,
    ) -> AppResult<User> {
        let user_id = user.id.ok_or_else(|| {
            AppError::Validation("cannot change the password of a user without an id".into())
        })?;
        if !verify_password(current_password, &user.password_hash)? {
            return Err(AppError::Validation("current password is incorrect".into()));
        }
        let new_hash = hash_password(new_password)?;
        self.repo.set_password(user_id, &new_hash)?;
        Ok(User {
            password_hash: new_hash,
            must_change_password: false,
            ..user.clone()
        })
    }

    /// True if `user`'s role currently grants `permission`.
    pub fn has_permission(&self, user: &User, permission: &str) -> AppResult<bool> {
        Ok(self.repo.get_role(user.role_id)?.has_permission(permission))
    }

    fn require_permission(
        &self,
        acting_user: &User,
        permission: &str,
        action: &str,
    ) -> AppResult<()> {
        if self.has_permission(acting_user, permission)? {
            Ok(())
        } else {
            Err(AppError::Validation(format!(
                "only an administrator can {action}"
            )))
        }
    }

    /// Every user account, for the admin-only Users screen -- unlike
    /// `list_other_active_users`, this includes the acting admin
    /// themselves and deactivated accounts, since managing those is the
    /// point of that screen.
    pub fn list_users(&self) -> AppResult<Vec<User>> {
        self.repo.list_users()
    }

    pub fn list_roles(&self) -> AppResult<Vec<Role>> {
        self.repo.list_roles()
    }

    /// Creates a user with a freshly generated temporary password,
    /// forced to change it on first login -- same shape as the
    /// bootstrap admin account `main.rs` creates on first run, just
    /// reachable from the Users screen instead of only at startup.
    /// Returns the plaintext password once: only its hash is ever
    /// stored, so this is the one and only chance to hand it to the new
    /// user. Requires `acting_user` to have `users.manage`.
    pub fn create_user(
        &self,
        acting_user: &User,
        username: &str,
        display_name: &str,
        role_id: i64,
    ) -> AppResult<(User, String)> {
        self.require_permission(acting_user, USERS_MANAGE, "create users")?;

        let password = generate_temporary_password();
        let password_hash = hash_password(&password)?;
        let mut user = User::new(username, display_name, password_hash, role_id);
        user.must_change_password = true;
        let created = self.repo.create_user(&user)?;
        Ok((created, password))
    }

    /// Updates a user's role, active status, or other editable fields.
    /// Requires `acting_user` to have `users.manage`.
    pub fn update_user(&self, acting_user: &User, user: &User) -> AppResult<()> {
        self.require_permission(acting_user, USERS_MANAGE, "update users")?;
        self.repo.update_user(user)
    }

    /// An administrator-triggered reset: generates a fresh temporary
    /// password (same scheme as `create_user`), forces a change on next
    /// login, and clears any pending reset request. Returns the
    /// plaintext password once, for `acting_user` to relay to the user
    /// directly. Requires `acting_user` to have `users.manage`.
    pub fn reset_password(&self, acting_user: &User, target_user_id: i64) -> AppResult<String> {
        self.require_permission(acting_user, USERS_MANAGE, "reset a user's password")?;

        let password = generate_temporary_password();
        let password_hash = hash_password(&password)?;
        self.repo
            .reset_password_by_admin(target_user_id, &password_hash)?;
        Ok(password)
    }

    /// Flags `username`'s account as having requested a password reset
    /// -- reached from the logged-out "Forgot Password" screen, so it
    /// can't require an authenticated session or acting-user permission
    /// check. Always succeeds from the caller's point of view, even for
    /// an unknown username: silently doing nothing rather than
    /// returning `NotFound` keeps this from being usable to probe which
    /// usernames exist, same reasoning as login's shared "invalid
    /// username or password" error.
    pub fn request_password_reset(&self, username: &str) -> AppResult<()> {
        if let Some(user) = self.repo.find_user_by_username(username)? {
            self.repo
                .set_reset_requested(user.id.unwrap_or_default(), true)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::database::Database;

    fn service_with_admin_and_staff_roles() -> (AuthService, User, Role) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(crate::backend::repositories::user_repository::MIGRATIONS)
            .unwrap();

        let service = AuthService::new(db);
        let mut admin_role = Role::new("Administrator");
        admin_role.permissions.push(USERS_MANAGE.into());
        let admin_role = service.repo.create_role(&admin_role).unwrap();
        let staff_role = service.repo.create_role(&Role::new("Staff")).unwrap();

        let admin = service
            .repo
            .create_user(&User::new(
                "admin",
                "Admin User",
                hash_password("correct horse battery staple").unwrap(),
                admin_role.id.unwrap(),
            ))
            .unwrap();

        (service, admin, staff_role)
    }

    #[test]
    fn has_permission_reflects_the_user_s_role() {
        let (service, admin, staff_role) = service_with_admin_and_staff_roles();
        let staff = User::new("bob", "Bob Jones", "hash", staff_role.id.unwrap());

        assert!(service.has_permission(&admin, USERS_MANAGE).unwrap());
        assert!(!service.has_permission(&staff, USERS_MANAGE).unwrap());
    }

    #[test]
    fn create_user_is_rejected_for_an_acting_user_without_users_manage() {
        let (service, _admin, staff_role) = service_with_admin_and_staff_roles();
        let staff = User::new("bob", "Bob Jones", "hash", staff_role.id.unwrap());

        let result = service.create_user(&staff, "carol", "Carol Smith", staff_role.id.unwrap());

        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn create_user_generates_a_random_password_that_actually_logs_in_and_forces_a_change() {
        let (service, admin, staff_role) = service_with_admin_and_staff_roles();

        let (created, password) = service
            .create_user(&admin, "carol", "Carol Smith", staff_role.id.unwrap())
            .unwrap();

        assert_eq!(password.len(), 16);
        assert!(created.must_change_password);
        assert_ne!(
            created.password_hash, password,
            "the stored hash must never equal the plaintext password"
        );

        // Proves the generated password is real and actually usable to
        // log in, not just a string that looks plausible.
        let outcome = service.login_with_credentials("carol", &password).unwrap();
        assert!(matches!(outcome, AuthOutcome::Authenticated(user) if user.username == "carol"));
    }

    #[test]
    fn update_user_is_rejected_for_an_acting_user_without_users_manage() {
        let (service, admin, staff_role) = service_with_admin_and_staff_roles();
        let (created, _password) = service
            .create_user(&admin, "carol", "Carol Smith", staff_role.id.unwrap())
            .unwrap();
        let non_admin = User::new("bob", "Bob Jones", "hash", staff_role.id.unwrap());

        let result = service.update_user(
            &non_admin,
            &User {
                display_name: "Carol Renamed".into(),
                ..created
            },
        );

        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn update_user_can_change_role_and_deactivate() {
        let (service, admin, staff_role) = service_with_admin_and_staff_roles();
        let (created, _password) = service
            .create_user(&admin, "carol", "Carol Smith", staff_role.id.unwrap())
            .unwrap();
        let another_role = service.repo.create_role(&Role::new("Manager")).unwrap();

        service
            .update_user(
                &admin,
                &User {
                    role_id: another_role.id.unwrap(),
                    is_active: false,
                    ..created.clone()
                },
            )
            .unwrap();

        let reloaded = service.repo.get_user(created.id.unwrap()).unwrap();
        assert_eq!(reloaded.role_id, another_role.id.unwrap());
        assert!(!reloaded.is_active);
    }

    #[test]
    fn a_deactivated_user_cannot_log_in() {
        let (service, admin, staff_role) = service_with_admin_and_staff_roles();
        let (created, password) = service
            .create_user(&admin, "carol", "Carol Smith", staff_role.id.unwrap())
            .unwrap();
        service
            .update_user(
                &admin,
                &User {
                    is_active: false,
                    ..created
                },
            )
            .unwrap();

        let result = service.login_with_credentials("carol", &password);

        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn reset_password_is_rejected_for_an_acting_user_without_users_manage() {
        let (service, admin, staff_role) = service_with_admin_and_staff_roles();
        let (created, _password) = service
            .create_user(&admin, "carol", "Carol Smith", staff_role.id.unwrap())
            .unwrap();
        let non_admin = User::new("bob", "Bob Jones", "hash", staff_role.id.unwrap());

        let result = service.reset_password(&non_admin, created.id.unwrap());

        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn reset_password_generates_a_new_password_that_logs_in_and_clears_a_pending_request() {
        let (service, admin, staff_role) = service_with_admin_and_staff_roles();
        let (created, old_password) = service
            .create_user(&admin, "carol", "Carol Smith", staff_role.id.unwrap())
            .unwrap();
        // Simulate carol changing her own password and then requesting
        // a reset, so there's a real pending flag to prove gets cleared.
        service
            .repo
            .set_password(created.id.unwrap(), &hash_password(&old_password).unwrap())
            .unwrap();
        service.request_password_reset("carol").unwrap();
        assert!(
            service
                .repo
                .get_user(created.id.unwrap())
                .unwrap()
                .is_reset_requested
        );

        let new_password = service.reset_password(&admin, created.id.unwrap()).unwrap();

        assert_ne!(new_password, old_password);
        let outcome = service
            .login_with_credentials("carol", &new_password)
            .unwrap();
        assert!(matches!(
            outcome,
            AuthOutcome::Authenticated(_) | AuthOutcome::TwoFactorRequired(_)
        ));
        let reloaded = service.repo.get_user(created.id.unwrap()).unwrap();
        assert!(reloaded.must_change_password);
        assert!(!reloaded.is_reset_requested);
    }

    #[test]
    fn request_password_reset_flags_a_known_username() {
        let (service, admin, staff_role) = service_with_admin_and_staff_roles();
        let (created, _password) = service
            .create_user(&admin, "carol", "Carol Smith", staff_role.id.unwrap())
            .unwrap();

        service.request_password_reset("CAROL").unwrap();

        assert!(
            service
                .repo
                .get_user(created.id.unwrap())
                .unwrap()
                .is_reset_requested
        );
    }

    #[test]
    fn request_password_reset_silently_succeeds_for_an_unknown_username() {
        let (service, _admin, _staff_role) = service_with_admin_and_staff_roles();

        let result = service.request_password_reset("nobody-by-this-name");

        assert!(result.is_ok());
    }

    /// Deactivating a user (the only thing `update_user` does to a
    /// staff account short of a role change -- there's no hard delete,
    /// same principle as posted-entry immutability) must never touch
    /// records that account already created. `posted_by_user_id` has no
    /// foreign key back to `users` by design (see its column in
    /// `accounting_repository`'s schema) specifically so a deactivation
    /// can't cascade into the ledger; this proves that guarantee holds
    /// end to end, not just by schema inspection.
    #[test]
    fn deactivating_a_user_leaves_their_previously_posted_transactions_fully_intact() {
        use crate::backend::accounting::accounting_rules::AccountType;
        use crate::backend::accounting::chart_of_accounts::Account;
        use crate::backend::accounting::fiscal_periods::FiscalPeriod;
        use crate::backend::organization::companies::Company;
        use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
        use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
        use crate::backend::transactions::transaction::Transaction;
        use crate::backend::transactions::transaction_lines::TransactionLine;
        use crate::backend::transactions::transaction_status::TransactionStatus;
        use crate::backend::transactions::transaction_types::TransactionType;
        use chrono::NaiveDate;
        use rust_decimal_macros::dec;

        let db = Database::open_in_memory().unwrap();
        db.run_migrations(crate::backend::repositories::user_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();

        let service = AuthService::new(db.clone());
        let mut admin_role = Role::new("Administrator");
        admin_role.permissions.push(USERS_MANAGE.into());
        let admin_role = service.repo.create_role(&admin_role).unwrap();
        let staff_role = service.repo.create_role(&Role::new("Staff")).unwrap();
        let admin = service
            .repo
            .create_user(&User::new(
                "admin",
                "Admin User",
                hash_password("correct horse battery staple").unwrap(),
                admin_role.id.unwrap(),
            ))
            .unwrap();

        let (carol, _password) = service
            .create_user(&admin, "carol", "Carol Smith", staff_role.id.unwrap())
            .unwrap();
        let carol_id = carol.id.unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap()
            .id
            .unwrap();
        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo
            .create_fiscal_period(&FiscalPeriod::new(
                company_id,
                "Open Period",
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            ))
            .unwrap();
        let cash = accounting_repo
            .create_account(&Account::new(
                company_id,
                "1000",
                "Cash",
                AccountType::Asset,
            ))
            .unwrap();
        let capital = accounting_repo
            .create_account(&Account::new(
                company_id,
                "3000",
                "Owner's Capital",
                AccountType::Equity,
            ))
            .unwrap();

        let mut transaction = Transaction::new(
            company_id,
            TransactionType::ManualJournal,
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        )
        .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(500)))
        .with_line(TransactionLine::credit(capital.id.unwrap(), dec!(500)));
        transaction.posted_by_user_id = Some(carol_id);
        let posted = accounting_repo.post_transaction(&transaction).unwrap();
        let transaction_id = posted.id.unwrap();

        service
            .update_user(
                &admin,
                &User {
                    is_active: false,
                    ..carol
                },
            )
            .unwrap();

        let reloaded_user = service.repo.get_user(carol_id).unwrap();
        assert!(
            !reloaded_user.is_active,
            "the account is deactivated, not deleted"
        );

        let reloaded_transaction = accounting_repo.get_transaction(transaction_id).unwrap();
        assert_eq!(reloaded_transaction.status, TransactionStatus::Posted);
        assert_eq!(reloaded_transaction.posted_by_user_id, Some(carol_id));
        assert_eq!(reloaded_transaction.total_debits(), dec!(500));
        assert_eq!(reloaded_transaction.total_credits(), dec!(500));
        assert_eq!(reloaded_transaction.lines.len(), 2);
    }
}
