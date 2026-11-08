pub mod account;
pub mod cases;
pub mod errors;
pub mod inspector;
pub mod live;
pub mod notes;
pub mod timeline;

use axum::Router;
use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use oc_kit::auth::CurrentUser;
use oc_kit::identity::User;
use oc_kit::prefs::{PagePrefs, Prefs, PrefsSource};
use oc_kit::ui::{Html, Icon, Page, Profile, Tab, TabBar, TopBar};

use crate::state::{AppState, BRANDING};

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(notes::routes())
        .merge(timeline::routes())
        .merge(cases::routes())
        .merge(inspector::routes())
        .merge(live::routes())
        .merge(account::routes())
}

/// Everything a signed-in page needs to render its shell.
pub struct Ctx {
    pub user: User,
    pub prefs: Prefs,
    pub source: PrefsSource,
}

impl FromRequestParts<AppState> for Ctx {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, st: &AppState) -> Result<Self, Self::Rejection> {
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, st)
            .await
            .map_err(IntoResponse::into_response)?;
        let PagePrefs(prefs, source) = PagePrefs::from_request_parts(parts, st)
            .await
            .map_err(IntoResponse::into_response)?;
        Ok(Ctx {
            user,
            prefs,
            source,
        })
    }
}

impl OptionalFromRequestParts<AppState> for Ctx {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        st: &AppState,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(
            <Ctx as FromRequestParts<AppState>>::from_request_parts(parts, st)
                .await
                .ok(),
        )
    }
}

const NAV: [(&str, Icon, &str); 4] = [
    ("/notes", Icon::Notes, "Notes"),
    ("/search", Icon::Search, "Search"),
    ("/cases", Icon::Cases, "Cases"),
    ("/account", Icon::Account, "Account"),
];

fn tabs(current: &str) -> Vec<Tab> {
    NAV.iter()
        .map(|(href, icon, label)| Tab::new(*href, *icon, *label, *href == current))
        .collect()
}

pub fn everyday(ctx: &Ctx, current: &str, title: &str, body: Html) -> Page {
    let mut top = TopBar::new(BRANDING, "/notes", ctx.prefs, Some(ctx.user.clone()));
    top.nav = tabs(current);
    let mut page = Page::new(title, Profile::Everyday, top, body, ctx.source);
    page.tabbar = Some(TabBar {
        items: tabs(current),
    });
    page
}

pub fn render<T: askama::Template>(t: &T) -> Html {
    Html::render(t)
}

pub fn paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|p| p.trim().to_owned())
        .filter(|p| !p.is_empty())
        .collect()
}

pub fn when(t: &time::OffsetDateTime) -> String {
    oc_kit::ui::date(*t)
}

pub fn tag_icon() -> Html {
    Icon::Tag.sized(oc_kit::ui::IconSize::Row)
}

pub fn demo_css() -> String {
    crate::app::assets().url("demo.css")
}
