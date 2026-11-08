use askama::Template;

use super::{Html, HxAttrs, Icon, IconSize, html_safe};
use crate::identity::tokens::TokenInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Default,
    Danger,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/button.html")]
pub struct Button {
    pub label: String,
    pub variant: ButtonVariant,
    pub icon: Option<Icon>,
    pub attrs: HxAttrs,
    pub disabled: bool,
    pub submit: bool,
    pub class: &'static str,
}

impl Button {
    pub fn new(label: impl Into<String>, variant: ButtonVariant) -> Self {
        Self {
            label: label.into(),
            variant,
            icon: None,
            attrs: HxAttrs::new(),
            disabled: false,
            submit: false,
            class: "",
        }
    }
    pub fn primary(label: impl Into<String>) -> Self {
        Self::new(label, ButtonVariant::Primary)
    }
    pub fn default(label: impl Into<String>) -> Self {
        Self::new(label, ButtonVariant::Default)
    }
    pub fn danger(label: impl Into<String>) -> Self {
        Self::new(label, ButtonVariant::Danger)
    }
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn attrs(mut self, attrs: HxAttrs) -> Self {
        self.attrs = attrs;
        self
    }
    pub fn submit(mut self) -> Self {
        self.submit = true;
        self
    }
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
    pub fn class(mut self, class: &'static str) -> Self {
        self.class = class;
        self
    }
    fn classes(&self) -> String {
        let v = match self.variant {
            ButtonVariant::Primary => " oc-btn--primary",
            ButtonVariant::Default => "",
            ButtonVariant::Danger => " oc-btn--danger",
        };
        let extra = if self.class.is_empty() {
            String::new()
        } else {
            format!(" {}", self.class)
        };
        format!("oc-btn{v}{extra}")
    }
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/icon-button.html")]
pub struct IconButton {
    pub icon: Icon,
    pub label: String,
    pub shortcut: Option<&'static str>,
    pub attrs: HxAttrs,
    pub row: bool,
    pub reveal: bool,
    pub pressed: Option<bool>,
    pub submit: bool,
}

impl IconButton {
    pub fn new(icon: Icon, label: impl Into<String>) -> Self {
        Self {
            icon,
            label: label.into(),
            shortcut: None,
            attrs: HxAttrs::new(),
            row: false,
            reveal: false,
            pressed: None,
            submit: false,
        }
    }
    pub fn shortcut(mut self, key: &'static str) -> Self {
        self.shortcut = Some(key);
        self
    }
    pub fn attrs(mut self, attrs: HxAttrs) -> Self {
        self.attrs = attrs;
        self
    }
    /// Table-row size; `reveal` hides it until the row is hovered, focused or selected.
    pub fn row(mut self, reveal: bool) -> Self {
        self.row = true;
        self.reveal = reveal;
        self
    }
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }
    pub fn submit(mut self) -> Self {
        self.submit = true;
        self
    }
    pub fn tip(&self) -> String {
        match self.shortcut {
            Some(k) => format!("{} ({k})", self.label),
            None => self.label.clone(),
        }
    }
    fn svg(&self) -> Html {
        self.icon.sized(if self.row {
            IconSize::Row
        } else {
            IconSize::Tool
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkKind {
    #[default]
    Prose,
    Data,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/link.html")]
pub struct Link {
    pub href: String,
    pub text: String,
    pub kind: LinkKind,
}

impl Link {
    pub fn prose(href: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            text: text.into(),
            kind: LinkKind::Prose,
        }
    }
    pub fn data(href: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            text: text.into(),
            kind: LinkKind::Data,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoticeKind {
    #[default]
    Info,
    Success,
    Warning,
    Danger,
}

impl NoticeKind {
    pub fn icon(self) -> Icon {
        match self {
            Self::Info => Icon::Info,
            Self::Success => Icon::Success,
            Self::Warning => Icon::Warning,
            Self::Danger => Icon::Danger,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
        }
    }
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/notice.html")]
pub struct Notice {
    pub kind: NoticeKind,
    pub text: String,
    pub action: Option<Button>,
    pub autohide_ms: Option<u32>,
}

impl Notice {
    pub fn new(kind: NoticeKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            action: None,
            autohide_ms: None,
        }
    }
    pub fn info(text: impl Into<String>) -> Self {
        Self::new(NoticeKind::Info, text)
    }
    pub fn success(text: impl Into<String>) -> Self {
        Self::new(NoticeKind::Success, text)
    }
    pub fn warning(text: impl Into<String>) -> Self {
        Self::new(NoticeKind::Warning, text)
    }
    pub fn danger(text: impl Into<String>) -> Self {
        Self::new(NoticeKind::Danger, text)
    }
    pub fn action(mut self, b: Button) -> Self {
        self.action = Some(b);
        self
    }
    pub fn autohide(mut self, ms: u32) -> Self {
        self.autohide_ms = Some(ms);
        self
    }
    /// For a fragment that lands in the floating `#oc-notices` stack via an out-of-band swap.
    pub fn oob(self) -> Html {
        Html(format!(
            "<div hx-swap-oob=\"afterbegin:#oc-notices\">{}</div>",
            Html::from(self)
        ))
    }
}

#[derive(Debug, Clone)]
pub struct Fact {
    pub label: String,
    pub value: String,
    pub mono: bool,
    pub copy: bool,
    pub href: Option<String>,
}

impl Fact {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            mono: false,
            copy: false,
            href: None,
        }
    }
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }
    pub fn copy(mut self) -> Self {
        self.copy = true;
        self
    }
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }
    fn copy_button(&self) -> CopyButton {
        CopyButton::new(&self.value, &self.label)
    }
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/facts.html")]
pub struct Facts {
    pub rows: Vec<Fact>,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/empty.html")]
pub struct Empty {
    pub text: String,
    pub action: Option<Button>,
}

impl Empty {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            action: None,
        }
    }
    pub fn action(mut self, b: Button) -> Self {
        self.action = Some(b);
        self
    }
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/copy-button.html")]
pub struct CopyButton {
    pub value: String,
    pub label: String,
}

impl CopyButton {
    pub fn new(value: impl Into<String>, label: impl AsRef<str>) -> Self {
        Self {
            value: value.into(),
            label: label.as_ref().to_lowercase(),
        }
    }
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/pill.html")]
pub struct Pill {
    pub text: String,
    pub exclude: bool,
    pub remove: HxAttrs,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/row-actions.html")]
pub struct RowActions {
    pub actions: Vec<IconButton>,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/dialog.html")]
pub struct Dialog {
    pub id: String,
    pub title: String,
    pub body: Html,
    pub actions: Vec<Button>,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/typed-confirm.html")]
pub struct TypedConfirm {
    pub id: String,
    pub title: String,
    pub text: String,
    pub name_to_type: String,
    pub action_label: String,
    pub post: HxAttrs,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/select.html")]
pub struct Select {
    pub name: String,
    pub label: String,
    pub options: Vec<(String, String)>,
    pub selected: String,
    pub attrs: HxAttrs,
}

#[derive(Debug, Clone)]
pub struct Tab {
    pub href: String,
    pub icon: Icon,
    pub label: String,
    pub current: bool,
}

impl Tab {
    pub fn new(
        href: impl Into<String>,
        icon: Icon,
        label: impl Into<String>,
        current: bool,
    ) -> Self {
        Self {
            href: href.into(),
            icon,
            label: label.into(),
            current,
        }
    }
    fn tab_icon(&self) -> Html {
        self.icon.sized(IconSize::Tab)
    }
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/tabbar.html")]
pub struct TabBar {
    pub items: Vec<Tab>,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/search.html")]
pub struct SearchBox {
    pub name: String,
    pub value: String,
    pub label: String,
    pub attrs: HxAttrs,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/statusbar.html")]
pub struct StatusBar {
    pub items: Vec<Html>,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/token-minted.html")]
pub struct TokenMinted {
    pub name: String,
    pub token: String,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/token-list.html")]
pub struct TokenList {
    pub tokens: Vec<TokenInfo>,
}

#[derive(Debug, Clone)]
pub struct PasskeyItem {
    pub id: i64,
    pub name: String,
    pub last_used: Option<String>,
}

#[derive(Debug, Clone, Template)]
#[template(path = "oc/passkey-list.html")]
pub struct PasskeyList {
    pub passkeys: Vec<PasskeyItem>,
    pub enabled: bool,
}

impl TokenList {
    fn meta(&self, t: &TokenInfo) -> String {
        match t.last_used_at {
            Some(u) => format!("created {} · last used {}", date(t.created_at), date(u)),
            None => format!("created {} · never used", date(t.created_at)),
        }
    }
}

pub fn date(t: time::OffsetDateTime) -> String {
    let f = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]");
    t.format(&f).unwrap_or_default()
}

html_safe!(
    Button,
    IconButton,
    Link,
    Notice,
    Facts,
    Empty,
    CopyButton,
    Pill,
    RowActions,
    Dialog,
    TypedConfirm,
    Select,
    TabBar,
    SearchBox,
    StatusBar,
    TokenMinted,
    TokenList,
    PasskeyList
);
