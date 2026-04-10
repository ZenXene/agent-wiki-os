pub mod fs;
pub use fs::FsAdapter;

pub trait Adapter {
    fn fetch(&self) -> anyhow::Result<String>;
}

pub struct CursorAdapter;
impl Adapter for CursorAdapter {
    fn fetch(&self) -> anyhow::Result<String> {
        Ok("Mock Cursor History".to_string())
    }
}
