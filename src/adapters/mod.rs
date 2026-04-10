pub mod fs;
pub mod history;
pub mod web;

pub use fs::FsAdapter;
pub use history::HistoryAdapter;
pub use web::WebAdapter;

pub trait Adapter {
    fn fetch(&self) -> anyhow::Result<String>;
}
