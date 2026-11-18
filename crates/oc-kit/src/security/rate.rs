use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{FromFnLayer, Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};

use super::ClientIp;

#[derive(Clone)]
pub struct Buckets {
    per_second: f64,
    burst: f64,
    state: Arc<Mutex<HashMap<IpAddr, (f64, Instant)>>>,
}

/// One IPv6 client usually controls a whole /64, so it shares one bucket.
fn bucket_key(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6((u128::from(v6) & !(u128::MAX >> 64)).into()),
        },
        v4 => v4,
    }
}

impl Buckets {
    pub fn new(per_minute: u32, burst: u32) -> Self {
        Self { per_second: f64::from(per_minute) / 60.0, burst: f64::from(burst), state: Arc::default() }
    }

    /// Spends one token for `ip`. To count only failures, call it from a handler
    /// after the failure and answer 429 on `false`.
    pub fn take(&self, ip: IpAddr) -> bool {
        let Ok(mut map) = self.state.lock() else {
            return true;
        };
        let now = Instant::now();
        if map.len() > 10_000 {
            // A bucket that has refilled carries no state, so it can go.
            let (rate, burst) = (self.per_second, self.burst);
            map.retain(|_, (tokens, t)| *tokens + now.duration_since(*t).as_secs_f64() * rate < burst);
            if map.len() > 100_000 {
                map.clear();
            }
        }
        let (tokens, last) = map.entry(bucket_key(ip)).or_insert((self.burst, now));
        *tokens = (*tokens + now.duration_since(*last).as_secs_f64() * self.per_second).min(self.burst);
        *last = now;
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

async fn limit(axum::extract::State(b): axum::extract::State<Buckets>, req: Request, next: Next) -> Response {
    let ip = req.extensions().get::<ClientIp>().map(|c| c.0);
    match ip {
        Some(ip) if !b.take(ip) => {
            (StatusCode::TOO_MANY_REQUESTS, "Too many attempts. Wait a minute and try again.").into_response()
        }
        _ => next.run(req).await,
    }
}

type RateLayer = FromFnLayer<
    fn(
        axum::extract::State<Buckets>,
        Request,
        Next,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>,
    Buckets,
    (axum::extract::State<Buckets>, Request),
>;

/// A per-client token bucket: `per_minute` sustained, `burst` at once.
pub fn rate_limit(per_minute: u32, burst: u32) -> RateLayer {
    from_fn_with_state(Buckets::new(per_minute, burst), |s, r, n| Box::pin(limit(s, r, n)))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{Buckets, bucket_key};

    #[test]
    fn buckets_refuse_after_burst() {
        let b = Buckets::new(60, 2);
        let ip = "10.0.0.1".parse().unwrap();
        assert!(b.take(ip));
        assert!(b.take(ip));
        assert!(!b.take(ip));
    }

    #[test]
    fn same_v6_64_shares() {
        assert_eq!(bucket_key("2001:db8::1".parse().unwrap()), bucket_key("2001:db8::ffff:2".parse().unwrap()));
    }

    #[test]
    fn other_v6_64_differs() {
        assert_ne!(bucket_key("2001:db8:0:1::1".parse().unwrap()), bucket_key("2001:db8:0:2::1".parse().unwrap()));
    }

    #[test]
    fn v4_whole() {
        assert_ne!(bucket_key("10.0.0.1".parse().unwrap()), bucket_key("10.0.0.2".parse().unwrap()));
    }

    #[test]
    fn v4_mapped_is_v4() {
        assert_eq!(bucket_key("::ffff:1.2.3.4".parse().unwrap()), bucket_key("1.2.3.4".parse().unwrap()));
        assert_ne!(bucket_key("::ffff:1.2.3.4".parse().unwrap()), bucket_key("::ffff:1.2.3.5".parse().unwrap()));
    }
}
