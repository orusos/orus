//! The apps that run as a window of their own under the compositor, started as
//! `orus-linux --app <name>`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppKind {
    Terminal,
    Files,
    TextEdit,
    Viewer,
}

impl AppKind {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "terminal" => Some(AppKind::Terminal),
            "files" => Some(AppKind::Files),
            "textedit" => Some(AppKind::TextEdit),
            "viewer" => Some(AppKind::Viewer),
            _ => None,
        }
    }

    /// The name used on the command line.
    pub fn name(self) -> &'static str {
        match self {
            AppKind::Terminal => "terminal",
            AppKind::Files => "files",
            AppKind::TextEdit => "textedit",
            AppKind::Viewer => "viewer",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            AppKind::Terminal => "Terminal",
            AppKind::Files => "Files",
            AppKind::TextEdit => "Text Edit",
            AppKind::Viewer => "Viewer",
        }
    }

    /// The Wayland app id, so the compositor can tell the apps apart.
    pub fn app_id(self) -> &'static str {
        match self {
            AppKind::Terminal => "orus-terminal",
            AppKind::Files => "orus-files",
            AppKind::TextEdit => "orus-textedit",
            AppKind::Viewer => "orus-viewer",
        }
    }
}
