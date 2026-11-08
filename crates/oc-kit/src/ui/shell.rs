use askama::Template;

use super::{Html, Icon, IconButton, Notice, SearchBox, StatusBar, Tab, TabBar, html_safe};
use crate::assets::AssetSet;
use crate::auth::Branding;
use crate::identity::User;
use crate::prefs::{Density, Prefs, PrefsSource, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
    Workbench,
    #[default]
    Everyday,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/topbar.html")]
pub struct TopBar {
    pub branding: Branding,
    pub home: String,
    pub breadcrumb: Vec<(String, String)>,
    pub nav: Vec<Tab>,
    pub search: Option<SearchBox>,
    pub prefs: Prefs,
    pub user: Option<User>,
}

impl TopBar {
    pub fn new(
        branding: Branding,
        home: impl Into<String>,
        prefs: Prefs,
        user: Option<User>,
    ) -> Self {
        Self {
            branding,
            home: home.into(),
            breadcrumb: Vec::new(),
            nav: Vec::new(),
            search: None,
            prefs,
            user,
        }
    }

    fn theme_label(&self) -> &'static str {
        match self.prefs.theme.next() {
            Theme::System => "Use the system theme",
            Theme::Light => "Switch to light theme",
            Theme::Dark => "Switch to dark theme",
        }
    }

    fn density_button(&self) -> IconButton {
        let compact = self.prefs.density == Density::Compact;
        let label = if compact {
            "Use default text size"
        } else {
            "Use compact text size"
        };
        IconButton::new(Icon::TextSize, label)
            .pressed(compact)
            .attrs(
                super::HxAttrs::new()
                    .with("data-oc-pref", "density")
                    .with("data-oc-value", self.prefs.density.as_str()),
            )
    }
}

/// Values read from the token file so the `theme-color` meta tags never drift from it.
fn theme_colors() -> &'static (String, String) {
    static C: std::sync::OnceLock<(String, String)> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        let (light, dark) =
            crate::tokens::parse_theme_blocks(include_str!("../../assets/css/00-tokens.css"));
        let get = |t: &crate::tokens::Theme| t.get("--theme-color").cloned().unwrap_or_default();
        (get(&light), get(&dark))
    })
}

pub const KIT_MODULES: [&str; 10] = [
    "prefs", "copy", "notices", "menu", "keys", "dialog", "rows", "pane", "busy", "passkey",
];

#[derive(Debug, Clone, Template)]
#[template(path = "oc/page.html")]
pub struct Page {
    pub title: String,
    pub profile: Profile,
    pub prefs: Prefs,
    pub prefs_source: PrefsSource,
    pub branding: Branding,
    pub top: TopBar,
    pub body: Html,
    pub rail: Vec<IconButton>,
    pub facets: Option<Html>,
    pub inspector: Option<Html>,
    pub pane: Option<Html>,
    pub statusbar: Option<StatusBar>,
    pub tabbar: Option<TabBar>,
    pub shortcuts: Vec<(&'static str, &'static str)>,
    pub extra_css: Vec<String>,
    pub extra_js: Vec<String>,
    pub head: Html,
    pub shell_attrs: super::HxAttrs,
    pub notices: Html,
}

pub const BASE_SHORTCUTS: [(&str, &str); 3] = [
    ("?", "Show shortcuts"),
    ("/", "Focus search"),
    ("Esc", "Close the open layer"),
];

impl Page {
    pub fn new(
        title: impl Into<String>,
        profile: Profile,
        top: TopBar,
        body: impl Into<Html>,
        source: PrefsSource,
    ) -> Self {
        Self {
            title: title.into(),
            profile,
            prefs: top.prefs,
            prefs_source: source,
            branding: top.branding,
            top,
            body: body.into(),
            rail: Vec::new(),
            facets: None,
            inspector: None,
            pane: None,
            statusbar: None,
            tabbar: None,
            shortcuts: BASE_SHORTCUTS.to_vec(),
            extra_css: Vec::new(),
            extra_js: Vec::new(),
            head: Html::default(),
            shell_attrs: super::HxAttrs::new(),
            notices: Html::default(),
        }
    }

    fn asset(&self, path: &str) -> String {
        AssetSet::kit().url(path)
    }

    fn profile_name(&self) -> &'static str {
        match self.profile {
            Profile::Workbench => "workbench",
            Profile::Everyday => "everyday",
        }
    }

    fn workbench(&self) -> bool {
        self.profile == Profile::Workbench
    }

    fn theme_attr(&self) -> Option<&'static str> {
        (self.prefs.theme != Theme::System).then(|| self.prefs.theme.as_str())
    }

    fn compact(&self) -> bool {
        self.prefs.density == Density::Compact
    }

    fn theme_light(&self) -> &'static str {
        &theme_colors().0
    }

    fn theme_dark(&self) -> &'static str {
        &theme_colors().1
    }

    fn modules(&self) -> Vec<String> {
        KIT_MODULES
            .iter()
            .map(|m| self.asset(&format!("js/{m}.js")))
            .chain(self.extra_js.iter().cloned())
            .collect()
    }
}

impl axum::response::IntoResponse for Page {
    fn into_response(self) -> axum::response::Response {
        Html::render(&self).into_response()
    }
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/sign-in.html")]
pub struct SignInForm {
    pub local: bool,
    pub passkeys: bool,
    pub oidc_label: Option<String>,
    pub notice: Option<Notice>,
    pub next: String,
}

html_safe!(TopBar, SignInForm);
