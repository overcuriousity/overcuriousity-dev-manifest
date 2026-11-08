use askama::Template;
use axum::Router;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use oc_kit::OcError;
use oc_kit::auth::passkeys;
use oc_kit::identity::tokens;
use oc_kit::ui::{Fact, Facts, Html, HxAttrs, PasskeyList, Select, TokenList};

use super::{Ctx, everyday, render};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/account", get(account))
}

#[derive(Template)]
#[template(path = "account.html")]
struct Account {
    who: Facts,
    theme: Select,
    density: Select,
    passkeys: Html,
    tokens: TokenList,
}

fn options(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(v, t)| ((*v).to_owned(), (*t).to_owned()))
        .collect()
}

async fn account(State(st): State<AppState>, ctx: Ctx) -> Result<Response, OcError> {
    let theme = Select {
        name: "theme".into(),
        label: "Theme".into(),
        options: options(&[
            ("system", "Follow the system"),
            ("light", "Light"),
            ("dark", "Dark"),
        ]),
        selected: ctx.prefs.theme.as_str().into(),
        attrs: HxAttrs::new().with("data-oc-pref-select", "theme"),
    };
    let density = Select {
        name: "density".into(),
        label: "Text size".into(),
        options: options(&[("default", "Default"), ("compact", "Compact")]),
        selected: ctx.prefs.density.as_str().into(),
        attrs: HxAttrs::new().with("data-oc-pref-select", "density"),
    };
    let passkeys = Html::from(PasskeyList {
        passkeys: passkeys::list_items(&st.pool, ctx.user.id).await?,
        enabled: st.auth.webauthn.is_some(),
    });
    let who = Facts {
        rows: vec![
            Fact::new("Name", ctx.user.display_name.clone()),
            Fact::new("Username", ctx.user.username.clone())
                .mono()
                .copy(),
            Fact::new(
                "Role",
                if ctx.user.is_admin {
                    "Admin"
                } else {
                    "Analyst"
                },
            ),
        ],
    };
    let body = render(&Account {
        who,
        theme,
        density,
        passkeys,
        tokens: TokenList {
            tokens: tokens::list(&st.pool, ctx.user.id).await?,
        },
    });
    Ok(everyday(&ctx, "/account", "Account", body).into_response())
}
