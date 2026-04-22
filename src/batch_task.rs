pub struct BatchInfo {
    pub task_name: String,
    pub max_concurrent: u32,
    pub batch_files: FilesInfo,
}

impl BatchInfo {
    pub fn new(name: String, max_concurrent: Option<u32>) -> Self {
        Self {
            task_name: name,
            max_concurrent: max_concurrent.unwrap_or(20),
            batch_files: FilesInfo::new(),
        }
    }

    pub fn add_file(&mut self, file: FileItem) {
        self.batch_files.add_file(file);
    }
}

pub struct FilesInfo {
    pub file_count: u32,
    pub file_items: Vec<FileItem>,
}

impl FilesInfo {
    fn new() -> Self {
        Self {
            file_count: 0,
            file_items: vec![],
        }
    }
    fn add_file(&mut self, file: FileItem) {
        self.file_items.push(file);
        self.file_count = self.file_items.len() as u32;
    }
}

pub struct FileItem {
    pub url: String,
    pub save_path: String,
    pub save_name: String,
    pub file_hash: Option<String>,
}
