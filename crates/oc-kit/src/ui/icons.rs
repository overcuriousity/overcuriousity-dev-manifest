use std::fmt;

macro_rules! icons {
    ($($variant:ident => $file:literal),* $(,)?) => {
        /// The fixed icon vocabulary: one meaning per icon, one icon per meaning.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Icon { $($variant),* }

        impl Icon {
            pub const ALL: &'static [Icon] = &[$(Icon::$variant),*];

            pub fn file(self) -> &'static str {
                match self { $(Icon::$variant => $file),* }
            }

            fn raw(self) -> &'static str {
                match self { $(Icon::$variant => include_str!(concat!("../../assets/icons/", $file, ".svg"))),* }
            }
        }
    };
}

icons! {
    Copy => "copy", Check => "check", FilterTo => "filter", Exclude => "filter-x", Pin => "pin",
    Archive => "archive", Delete => "trash-2", Restore => "undo-2", Edit => "pencil", Share => "share-2",
    Download => "download", Search => "search", Settings => "settings", Close => "x", New => "plus",
    History => "history", Link => "link", Tag => "tag", Show => "eye", Hide => "eye-off",
    ThemeLight => "sun", ThemeDark => "moon", ThemeSystem => "monitor", TextSize => "a-large-small",
    Info => "info", Success => "circle-check", Warning => "triangle-alert", Danger => "circle-x",
    More => "ellipsis", ChevronRight => "chevron-right", ChevronDown => "chevron-down", Notes => "notebook",
    Cases => "folder", Account => "user", SignOut => "log-out", Facets => "panel-left", Story => "book-open",
    Passkey => "key-round", Timeline => "list",
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconSize {
    Row,
    #[default]
    Tool,
    Tab,
}

impl Icon {
    pub fn sized(self, size: IconSize) -> super::Html {
        let raw = self.raw().trim_end();
        let class = match size {
            IconSize::Row => "oc-icon oc-icon--row",
            IconSize::Tool => "oc-icon",
            IconSize::Tab => "oc-icon oc-icon--tab",
        };
        let svg = raw.replacen("class=\"oc-icon\"", &format!("class=\"{class}\" data-icon=\"{}\"", self.file()), 1);
        super::Html(svg)
    }
}

impl fmt::Display for Icon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.sized(IconSize::Tool).0)
    }
}

impl askama::filters::HtmlSafe for Icon {}
