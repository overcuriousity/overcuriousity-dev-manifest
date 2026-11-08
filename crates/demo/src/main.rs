use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use demo::state::{APP, Settings};
use demo::{app, db, domain};
use oc_kit::config::{OcConfig, load};
use oc_kit::identity::{password, users};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "demo",
    version,
    about = "Incident journal: the overcuriousity reference app"
)]
struct Cli {
    /// TOML config file; DEMO__SECTION__KEY environment variables override it.
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the web server (the default).
    Serve,
    /// Manage accounts.
    #[command(subcommand)]
    User(UserCommand),
}

#[derive(Subcommand)]
enum UserCommand {
    /// Create an account and print its one-time password.
    Add {
        username: String,
        #[arg(long)]
        admin: bool,
        #[arg(long)]
        display_name: Option<String>,
    },
    /// Replace an account's password and print the new one.
    SetPassword { username: String },
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), oc_kit::OcError> {
    let cfg: OcConfig = load(APP, cli.config.as_deref())?;
    let settings: Settings = cfg.app()?;
    let pool = db::open(&cfg.data_dir(APP)).await?;
    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(cfg, settings, pool).await,
        Command::User(UserCommand::Add {
            username,
            admin,
            display_name,
        }) => {
            let pw = password::generate(20);
            let name = display_name.unwrap_or_else(|| username.clone());
            users::create(&pool, &username, &name, admin, Some(&pw), domain::now()).await?;
            println!("Created {username}. Password (shown once): {pw}");
            Ok(())
        }
        Command::User(UserCommand::SetPassword { username }) => {
            let user = users::by_username(&pool, &username)
                .await?
                .ok_or(oc_kit::OcError::NotFound)?;
            let pw = password::generate(20);
            users::set_password(&pool, user.id, &pw).await?;
            println!("New password for {username} (shown once): {pw}");
            Ok(())
        }
    }
}

async fn serve(
    cfg: OcConfig,
    settings: Settings,
    pool: sqlx::SqlitePool,
) -> Result<(), oc_kit::OcError> {
    if let Some(pw) = domain::seed::ensure_demo_user(&pool, domain::now()).await? {
        tracing::warn!("Seeded user demo with password {pw} (shown once)");
    }
    if domain::seed::seed(&pool, domain::now()).await? {
        tracing::info!("Generated synthetic cases, events and notes");
    }
    let (router, st) = app::build(&cfg, pool.clone()).await?;
    demo::ingest::spawn(&st, &settings.ingest);
    oc_kit::trash::spawn_purger(
        domain::notes::NotesStore(pool),
        std::time::Duration::from_secs(3600),
    );
    let listener = tokio::net::TcpListener::bind(cfg.bind)
        .await
        .map_err(|e| oc_kit::OcError::Config(format!("Cannot listen on {}: {e}.", cfg.bind)))?;
    tracing::info!("Listening on {}", cfg.deployment()?.origin);
    let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Ready]);
    spawn_watchdog();
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await
    .map_err(oc_kit::OcError::internal)
}

fn spawn_watchdog() {
    let mut usec = 0;
    if !sd_notify::watchdog_enabled(false, &mut usec) {
        return;
    }
    let every = std::time::Duration::from_micros(usec / 2);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(every).await;
            let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]);
        }
    });
}

async fn shutdown() {
    let ctrl_c = tokio::signal::ctrl_c();
    let mut term = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(s) => s,
        Err(_) => return std::future::pending().await,
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = term.recv() => {},
    }
    let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Stopping]);
}
