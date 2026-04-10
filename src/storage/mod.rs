use std::path::PathBuf;

pub struct WikiStorage {
    pub global_path: PathBuf,
    pub local_path: Option<PathBuf>,
}

impl WikiStorage {
    pub fn new(local_path: Option<PathBuf>) -> Self {
        let global_path = dirs::home_dir().unwrap().join(".agent-wiki-os");
        std::fs::create_dir_all(&global_path).unwrap();
        
        if let Some(ref lp) = local_path {
            std::fs::create_dir_all(lp).unwrap();
        }

        Self {
            global_path,
            local_path,
        }
    }
}
