use std::path::Path;
use std::fs::File;
use walkdir::WalkDir;

pub struct FsAdapter {
    pub target_path: String,
}

impl FsAdapter {
    pub fn new(path: &str) -> Self {
        Self {
            target_path: path.to_string(),
        }
    }

    pub fn fetch_all(&self) -> anyhow::Result<Vec<String>> {
        let mut results = Vec::new();
        let path = Path::new(&self.target_path);

        if !path.exists() {
            anyhow::bail!("Path does not exist: {}", self.target_path);
        }

        if path.is_file() {
            // Process single file
            if let Ok(content) = self.read_file(path) {
                results.push(content);
            }
        } else {
            // Process directory
            for entry in WalkDir::new(&self.target_path).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_file() {
                    if let Ok(content) = self.read_file(p) {
                        results.push(content);
                    }
                }
            }
        }
        
        Ok(results)
    }

    fn read_file(&self, path: &Path) -> anyhow::Result<String> {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        
        match extension.as_str() {
            "md" | "txt" | "rs" | "py" | "js" | "ts" | "json" | "toml" | "yaml" | "yml" | "html" | "css" | "csv" => {
                // Read as plain text
                let content = std::fs::read_to_string(path)?;
                Ok(format!("File: {}\n\n{}", path.display(), content))
            },
            "pdf" => {
                match pdf_extract::extract_text(path) {
                    Ok(content) => Ok(format!("File: {}\n\n{}", path.display(), content)),
                    Err(e) => anyhow::bail!("Failed to extract PDF: {}", e),
                }
            },
            "docx" => {
                match self.extract_docx(path) {
                    Ok(content) => Ok(format!("File: {}\n\n{}", path.display(), content)),
                    Err(e) => anyhow::bail!("Failed to extract DOCX: {}", e),
                }
            },
            "xlsx" | "xls" => {
                match self.extract_excel(path) {
                    Ok(content) => Ok(format!("File: {}\n\n{}", path.display(), content)),
                    Err(e) => anyhow::bail!("Failed to extract Excel: {}", e),
                }
            },
            "pptx" => {
                match self.extract_pptx(path) {
                    Ok(content) => Ok(format!("File: {}\n\n{}", path.display(), content)),
                    Err(e) => anyhow::bail!("Failed to extract PPTX: {}", e),
                }
            },
            _ => {
                // Try reading as text anyway, if it fails, it's probably binary
                match std::fs::read_to_string(path) {
                    Ok(content) => Ok(format!("File: {}\n\n{}", path.display(), content)),
                    Err(_) => anyhow::bail!("Unsupported or binary file format"),
                }
            }
        }
    }

    fn extract_docx(&self, path: &Path) -> anyhow::Result<String> {
        use std::io::Read;
        let file = File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        
        let mut xml_content = String::new();
        let mut document_file = archive.by_name("word/document.xml")?;
        document_file.read_to_string(&mut xml_content)?;
        
        let text = Self::extract_text_from_xml(&xml_content, b"w:t")?;
        Ok(text)
    }

    fn extract_pptx(&self, path: &Path) -> anyhow::Result<String> {
        use std::io::Read;
        let file = File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        
        let mut all_text = String::new();
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            
            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                let mut xml_content = String::new();
                file.read_to_string(&mut xml_content)?;
                if let Ok(slide_text) = Self::extract_text_from_xml(&xml_content, b"a:t") {
                    all_text.push_str(&slide_text);
                    all_text.push_str("\n\n");
                }
            }
        }
        
        Ok(all_text)
    }

    fn extract_text_from_xml(xml: &str, target_tag: &[u8]) -> anyhow::Result<String> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut text = String::new();
        let mut in_target = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) if e.name().as_ref() == target_tag => {
                    in_target = true;
                }
                Ok(Event::Text(e)) if in_target => {
                    if let Ok(t) = std::str::from_utf8(e.as_ref()) {
                        text.push_str(t);
                        text.push(' ');
                    }
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == target_tag => {
                    in_target = false;
                }
                Ok(Event::Eof) => break,
                Err(e) => anyhow::bail!("XML Error: {}", e),
                _ => (),
            }
        }
        Ok(text)
    }

    fn extract_excel(&self, path: &Path) -> anyhow::Result<String> {
        use calamine::{Reader, open_workbook_auto, Data};
        
        let mut workbook = open_workbook_auto(path)
            .map_err(|e| anyhow::anyhow!("Excel open error: {}", e))?;
        
        let mut text = String::new();
        
        // Use clone() to avoid holding a reference to workbook inside the loop
        let sheets = workbook.sheet_names().to_vec();
        for sheet_name in sheets {
            text.push_str(&format!("--- Sheet: {} ---\n", sheet_name));
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                for row in range.rows() {
                    let row_text: Vec<String> = row.iter().map(|cell| {
                        match cell {
                            Data::String(s) => s.clone(),
                            Data::Float(f) => f.to_string(),
                            Data::Int(i) => i.to_string(),
                            Data::Bool(b) => b.to_string(),
                            Data::Error(e) => format!("ERR: {:?}", e),
                            Data::Empty => "".to_string(),
                            Data::DateTime(d) => d.to_string(),
                            Data::DateTimeIso(d) => d.to_string(),
                            Data::DurationIso(d) => d.to_string(),
                        }
                    }).collect();
                    text.push_str(&row_text.join(" | "));
                    text.push('\n');
                }
            }
            text.push('\n');
        }
        
        Ok(text)
    }
}
