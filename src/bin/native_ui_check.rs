//! Render real Iced widgets in a native window and capture every workflow.
//! Uses an in-memory database; never opens a user's accounting database.
use iced::{Element, Subscription, Task};
use memonbooks::{
    backend::{
        organization::companies::Company,
        repositories::organization_repository::OrganizationRepository,
    },
    database::database::Database,
    frontend::{
        components::colors::Theme,
        views::{
            accounting::{ledger_view, maintain::maintain_view},
            dashboard::dashboard_view,
            accounting::maintain::vendors_view,
            Ctx,
        },
    },
};
struct Check {
    ctx: Ctx,
    step: usize,
    frames: u8,
    capturing: bool,
    dashboard: dashboard_view::State,
    ledger: ledger_view::State,
    maintain: maintain_view::State,
    output: std::path::PathBuf,
}
#[derive(Debug, Clone)]
enum Message {
    Frame,
    Captured(iced::window::Screenshot),
    Dashboard(dashboard_view::Message),
    Ledger(ledger_view::Message),
    Maintain(maintain_view::Message),
}
impl Check {
    fn new() -> Self {
        let db = Database::open_in_memory().expect("memory database");
        db.run_migrations(&memonbooks::all_migrations())
            .expect("migrations");
        let company = OrganizationRepository::new(db.clone())
            .create_company(&Company::new("UI Check Company", "PKR"))
            .expect("company");
        let output = std::env::args()
            .nth(1)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("memonbooks-ui"));
        std::fs::create_dir_all(&output).expect("output directory");
        let ctx = Ctx {
            db,
            company_id: company.id.unwrap(),
            current_user_id: None,
            export_dir: output.clone(),
            uploads_dir: output.clone(),
        };
        let dashboard = dashboard_view::load(&ctx);
        let ledger = ledger_view::load(&ctx);
        let maintain = maintain_view::State::default();
        Self {
            ctx,
            step: 0,
            frames: 0,
            capturing: false,
            dashboard,
            ledger,
            maintain,
            output,
        }
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Frame => {
                self.frames = self.frames.saturating_add(1);
                if self.frames >= 20 && !self.capturing {
                    self.capturing = true;
                    return iced::window::get_latest()
                        .and_then(iced::window::screenshot)
                        .map(Message::Captured);
                }
            }
            Message::Captured(screenshot) => {
                let name = match self.step {
                    0 => "home",
                    1 => "general-ledger",
                    2 => "vendors-list",
                    3 => "vendor-form",
                    _ => "vendor-preview",
                };
                let path = self.output.join(format!("{name}.png"));
                let file = std::fs::File::create(&path).expect("screenshot file");
                let mut encoder =
                    png::Encoder::new(file, screenshot.size.width, screenshot.size.height);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .expect("PNG header")
                    .write_image_data(&screenshot.bytes)
                    .expect("PNG data");
                println!(
                    "Rendered {} ({}×{})",
                    path.display(),
                    screenshot.size.width,
                    screenshot.size.height
                );
                self.step += 1;
                if self.step >= 5 {
                    return iced::exit();
                }
                if self.step == 2 {
                    maintain_view::update(
                        &mut self.maintain,
                        maintain_view::Message::Select(maintain_view::Section::Vendors),
                        &self.ctx,
                    );
                } else if self.step == 3 {
                    maintain_view::update(
                        &mut self.maintain,
                        maintain_view::Message::Vendors(vendors_view::Message::OpenForm),
                        &self.ctx,
                    );
                } else if self.step == 4 {
                    maintain_view::update(
                        &mut self.maintain,
                        maintain_view::Message::Vendors(vendors_view::Message::Preview),
                        &self.ctx,
                    );
                }
                self.frames = 0;
                self.capturing = false;
            }
            Message::Dashboard(m) => dashboard_view::update(&mut self.dashboard, m, &self.ctx),
            Message::Ledger(m) => ledger_view::update(&mut self.ledger, m, &self.ctx),
            Message::Maintain(m) => maintain_view::update(&mut self.maintain, m, &self.ctx),
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Message> {
        match self.step {
            0 => dashboard_view::view(&self.dashboard, Theme::Light).map(Message::Dashboard),
            1 => ledger_view::view(&self.ledger, Theme::Light).map(Message::Ledger),
            _ => maintain_view::view(&self.maintain, Theme::Light).map(Message::Maintain),
        }
    }
    fn subscription(&self) -> Subscription<Message> {
        iced::window::frames().map(|_| Message::Frame)
    }
}
fn main() -> iced::Result {
    iced::application(
        "MemonBooks — Native UI Verification",
        Check::update,
        Check::view,
    )
    .subscription(Check::subscription)
    .theme(|_| iced::Theme::Light)
    .window_size((1440., 960.))
    .run_with(|| (Check::new(), Task::none()))
}
