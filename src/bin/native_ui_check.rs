//! Render real Iced widgets in a native window and capture every workflow.
//! Uses an in-memory database; never opens a user's accounting database.
use iced::{Element, Subscription, Task};
use memonbooks::{
    backend::{
        organization::companies::Company,
        repositories::organization_repository::OrganizationRepository,
        transactions::documents::{Kind, ALL},
    },
    database::database::Database,
    frontend::{
        styles::colors::Theme,
        views::{
            accounting::{documents_view, ledger_view},
            dashboard::dashboard_view,
            Ctx,
        },
    },
};
struct Check {
    ctx: Ctx,
    step: usize,
    frames: u8,
    capturing: bool,
    documents: documents_view::State,
    dashboard: dashboard_view::State,
    ledger: ledger_view::State,
    output: std::path::PathBuf,
}
#[derive(Debug, Clone)]
enum Message {
    Frame,
    Captured(iced::window::Screenshot),
    Document(documents_view::Message),
    Dashboard(dashboard_view::Message),
    Ledger(ledger_view::Message),
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
        let documents = documents_view::load(&ctx, Kind::PurchaseOrder);
        Self {
            ctx,
            step: 0,
            frames: 0,
            capturing: false,
            documents,
            dashboard,
            ledger,
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
                let name = if self.step == 0 {
                    "home".to_string()
                } else if self.step == 1 {
                    "general-ledger".into()
                } else {
                    ALL[(self.step - 2) / 2]
                        .label()
                        .to_lowercase()
                        .replace(' ', "-")
                        + if (self.step - 2) % 2 == 0 {
                            "-list"
                        } else {
                            "-form"
                        }
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
                if self.step >= 2 + ALL.len() * 2 {
                    return iced::exit();
                }
                if self.step >= 2 {
                    self.documents = documents_view::load(&self.ctx, ALL[(self.step - 2) / 2]);
                    if (self.step - 2) % 2 == 1 {
                        documents_view::update(
                            &mut self.documents,
                            documents_view::Message::Add,
                            &self.ctx,
                        );
                    }
                }
                self.frames = 0;
                self.capturing = false;
            }
            Message::Document(m) => documents_view::update(&mut self.documents, m, &self.ctx),
            Message::Dashboard(m) => dashboard_view::update(&mut self.dashboard, m, &self.ctx),
            Message::Ledger(m) => ledger_view::update(&mut self.ledger, m, &self.ctx),
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Message> {
        match self.step {
            0 => dashboard_view::view(&self.dashboard, Theme::Light).map(Message::Dashboard),
            1 => ledger_view::view(&self.ledger, Theme::Light).map(Message::Ledger),
            _ => documents_view::view(&self.documents, Theme::Light, ALL).map(Message::Document),
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
