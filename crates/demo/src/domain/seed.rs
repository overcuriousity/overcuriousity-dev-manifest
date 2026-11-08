//! Synthetic, deterministic incident data. Every value is invented.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sqlx::SqlitePool;
use time::OffsetDateTime;

use super::events::{self, NewEvent};
use super::notes::{self, NoteInput};
use super::{OcError, cases, ms};
use oc_kit::identity::{password, users};

pub const CASES: [(&str, &str); 3] = [
    (
        "ACME breach",
        "Interactive logons on ws-01 outside business hours, followed by a new service on the file server.",
    ),
    ("Payroll phish", "A macro document from a spoofed payroll address; two users opened it."),
    ("Lab ws-07", "Unexpected PowerShell on a lab workstation that should be idle."),
];

const HOSTS: [&str; 12] = [
    "ws-01",
    "ws-02",
    "ws-03",
    "ws-04",
    "ws-05",
    "ws-06",
    "lab-ws07",
    "ws-08",
    "srv-dc01",
    "srv-file01",
    "srv-sql01",
    "srv-web01",
];
const USERS: [&str; 20] = [
    "iain.olsen",
    "lilli.moll",
    "svc_backup01",
    "mara.kowal",
    "j.ito",
    "admin.reyes",
    "t.nakamura",
    "SYSTEM",
    "a.osei",
    "b.lindqvist",
    "c.duarte",
    "d.huber",
    "e.varga",
    "f.mensah",
    "g.ruiz",
    "h.kim",
    "svc_sql",
    "k.ahmed",
    "l.novak",
    "m.weber",
];
const PROCESSES: [(&str, &str); 10] = [
    ("powershell.exe", "powershell.exe -NoProfile -EncodedCommand SQBFAFgA"),
    ("cmd.exe", "cmd.exe /c whoami /all"),
    ("rundll32.exe", "rundll32.exe C:\\Users\\Public\\x.dll,Start"),
    ("svchost.exe", "svchost.exe -k netsvcs -p"),
    ("explorer.exe", "C:\\Windows\\explorer.exe"),
    ("winword.exe", "\"C:\\Program Files\\Microsoft Office\\WINWORD.EXE\" /n payroll.docm"),
    ("lsass.exe", "C:\\Windows\\system32\\lsass.exe"),
    ("schtasks.exe", "schtasks.exe /create /sc minute /mo 5 /tn Updater /tr C:\\ProgramData\\u.exe"),
    ("net.exe", "net.exe group \"Domain Admins\" /domain"),
    ("sqlservr.exe", "C:\\Program Files\\Microsoft SQL Server\\sqlservr.exe -sMSSQLSERVER"),
];
const EVENT_IDS: [(i64, &str, &str, &str); 15] = [
    (4624, "info", "Security", "An account was successfully logged on."),
    (4625, "warning", "Security", "An account failed to log on."),
    (4634, "info", "Security", "An account was logged off."),
    (4648, "warning", "Security", "A logon was attempted using explicit credentials."),
    (4672, "info", "Security", "Special privileges assigned to new logon."),
    (4688, "info", "Security", "A new process has been created."),
    (4697, "warning", "Security", "A service was installed in the system."),
    (4698, "warning", "Security", "A scheduled task was created."),
    (4720, "warning", "Security", "A user account was created."),
    (4732, "warning", "Security", "A member was added to a security-enabled local group."),
    (7045, "warning", "System", "A new service was installed in the system."),
    (1102, "error", "Security", "The audit log was cleared."),
    (4104, "warning", "PowerShell", "Creating Scriptblock text."),
    (5140, "info", "Security", "A network share object was accessed."),
    (4776, "info", "Security", "The computer attempted to validate the credentials for an account."),
];
const INTEGRITY: [&str; 4] = ["Low", "Medium", "High", "System"];

/// 2026-10-28 00:00 UTC: seeded timelines end here so the data is identical on every machine.
const SEED_END_MS: i64 = 1_793_145_600_000;
const DAY_MS: i64 = 86_400_000;

fn pick<'a, T>(rng: &mut impl Rng, items: &'a [T]) -> &'a T {
    &items[rng.random_range(0..items.len())]
}

fn hex(rng: &mut impl Rng, len: usize) -> String {
    (0..len).map(|_| char::from(b"0123456789abcdef"[rng.random_range(0..16)])).collect()
}

pub fn event(rng: &mut impl Rng, ts: i64, record: i64) -> NewEvent {
    let (event_id, level, source, message) = *pick(rng, &EVENT_IDS);
    let (process, cmd) = *pick(rng, &PROCESSES);
    let host = *pick(rng, &HOSTS);
    let with_process = matches!(event_id, 4688 | 4104 | 4697 | 4698 | 7045);
    NewEvent {
        ts,
        host: host.to_owned(),
        user: (*pick(rng, &USERS)).to_owned(),
        event_id,
        level: level.to_owned(),
        source: source.to_owned(),
        process: process.to_owned(),
        pid: Some(rng.random_range(400..12_000)),
        parent: with_process
            .then(|| (*pick(rng, &["explorer.exe", "services.exe", "winword.exe", "cmd.exe"])).to_owned()),
        command_line: with_process.then(|| cmd.to_owned()),
        record,
        integrity: Some((*pick(rng, &INTEGRITY)).to_owned()),
        logon_id: Some(format!("0x{}", hex(rng, 6))),
        source_file: format!("{host}/Windows/System32/winevt/Logs/{source}.evtx"),
        sha256: with_process.then(|| hex(rng, 64)),
        message: message.to_owned(),
    }
}

/// Creates the demo admin when there are no users; returns the one-time password.
pub async fn ensure_demo_user(pool: &SqlitePool, now: OffsetDateTime) -> Result<Option<String>, OcError> {
    if users::count(pool).await? > 0 {
        return Ok(None);
    }
    let pw = password::generate(20);
    users::create(pool, "demo", "Demo analyst", true, Some(&pw), now).await?;
    Ok(Some(pw))
}

const NOTES: [(usize, &str, &str, &str); 6] = [
    (
        0,
        "Logon outside hours",
        "iain.olsen logged on interactively at 03:14 although badge records place him off-site.\n\nCompare with VPN logs before contacting HR.",
        "logon,timeline",
    ),
    (
        0,
        "New service on srv-file01",
        "A service named Updater was installed by svc_backup01. The binary hash is not on the allow-list.",
        "persistence,service",
    ),
    (
        1,
        "Macro document",
        "payroll.docm spawned PowerShell with an encoded command on ws-03 and ws-05.\n\nBoth users clicked through the macro warning.",
        "phishing,macro",
    ),
    (
        1,
        "Mail headers",
        "The sender domain was registered four days before the campaign. SPF passed for the look-alike domain.",
        "phishing,mail",
    ),
    (
        2,
        "Idle lab machine",
        "lab-ws07 runs no scheduled jobs. Script block logging shows a download cradle at 22:40.",
        "powershell",
    ),
    (
        2,
        "Next steps",
        "Image the disk, collect memory, and check the lab VLAN firewall for outbound connections.",
        "todo",
    ),
];

pub async fn seed(pool: &SqlitePool, now: OffsetDateTime) -> Result<bool, OcError> {
    let existing: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM cases").fetch_one(pool).await?;
    if existing > 0 {
        return Ok(false);
    }
    let author = sqlx::query_scalar!(r#"SELECT id AS "id!" FROM oc_users ORDER BY id LIMIT 1"#)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| OcError::Internal("Seeding needs a user.".into()))?;
    let mut rng = ChaCha8Rng::seed_from_u64(0x0c0c_2026);
    let mut ids = Vec::new();
    for (i, (name, summary)) in CASES.iter().enumerate() {
        let created = now - time::Duration::days(i64::try_from(i).unwrap_or(0) + 1);
        ids.push(cases::create(pool, name, summary, created).await?);
    }
    let mut tx = pool.begin().await?;
    for (i, id) in ids.iter().copied().enumerate() {
        let count = [2400, 1600, 1000][i];
        let span = 3 * DAY_MS;
        for record in 0..count {
            let ts = SEED_END_MS - span + rng.random_range(0..span);
            events::insert(&mut *tx, id, &event(&mut rng, ts, 100_000 + record)).await?;
        }
    }
    tx.commit().await?;
    for (case, title, body, tags) in NOTES {
        let input = NoteInput {
            case_id: ids.get(case).copied(),
            title: title.into(),
            body: body.into(),
            tags: notes::parse_tags(tags),
        };
        notes::create(pool, author, &input, now).await?;
    }
    Ok(true)
}

pub fn now_ms() -> i64 {
    ms(OffsetDateTime::now_utc())
}
