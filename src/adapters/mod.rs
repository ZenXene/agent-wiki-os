pub mod fs;
pub mod history;

pub use fs::FsAdapter;
pub use history::HistoryAdapter;

pub trait Adapter {
    fn fetch(&self) -> anyhow::Result<String>;
}
