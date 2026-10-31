use crate::editor::EditorBuffer;

impl EditorBuffer {
    /// Open a file by path and load its contents.
    pub fn open_file(&mut self, path: &str) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.file_path = Some(path.to_string());
        self.set_text(content);
        Ok(())
    }

    /// Save current buffer to disk. Returns Ok(true) if saved, Ok(false) if no path.
    pub fn save(&mut self) -> std::io::Result<bool> {
        if let Some(ref path) = self.file_path {
            std::fs::write(path, &self.text)?;
            self.dirty = false;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Save to a specific path.
    pub fn save_as(&mut self, path: &str) -> std::io::Result<()> {
        self.file_path = Some(path.to_string());
        std::fs::write(path, &self.text)?;
        self.dirty = false;
        Ok(())
    }

    /// Reset to empty untitled document.
    pub fn new_file(&mut self) {
        self.file_path = None;
        self.set_text(String::new());
    }

    /// Open a file using a native file dialog. Returns true if a file was opened.
    pub fn open_file_dialog(&mut self) -> bool {
        let file = rfd::FileDialog::new()
            .add_filter("Text & Markdown", &["txt", "md", "markdown"])
            .add_filter("All files", &["*"])
            .pick_file();
        if let Some(path) = file {
            let path_str = path.to_string_lossy().to_string();
            if self.open_file(&path_str).is_ok() {
                return true;
            }
        }
        false
    }

    /// Save-as using a native file dialog. Returns true if saved.
    pub fn save_as_dialog(&mut self) -> bool {
        let file = rfd::FileDialog::new()
            .add_filter("Markdown", &["md"])
            .add_filter("Text", &["txt"])
            .add_filter("All files", &["*"])
            .save_file();
        if let Some(path) = file {
            let path_str = path.to_string_lossy().to_string();
            if self.save_as(&path_str).is_ok() {
                return true;
            }
        }
        false
    }

    /// Try to save; if no path, use save-as dialog. Returns true if saved.
    pub fn save_or_save_as(&mut self) -> bool {
        match self.save() {
            Ok(true) => true,
            Ok(false) => self.save_as_dialog(),
            Err(_) => false,
        }
    }

    /// Build the window title from file state.
    pub fn window_title(&self) -> String {
        let name = self
            .file_path
            .as_ref()
            .and_then(|p| {
                std::path::Path::new(p)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "untitled".to_string());
        let dirty = if self.dirty { " *" } else { "" };
        format!("detir-textedit — {name}{dirty}")
    }
}
