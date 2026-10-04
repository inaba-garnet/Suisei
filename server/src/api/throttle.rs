//! 認証の総当たりの制限（docs/server.md の「認証の総当たりの制限」）。

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::HeaderMap;
use axum::http::request::Parts;

use super::AppState;

/// この回数を超えて失敗したら拒む。
const MAX_FAILURES: u32 = 10;
/// 失敗を数える期間。最初の失敗からこの長さが過ぎたら数え直す。
const WINDOW: Duration = Duration::from_secs(15 * 60);
/// 拒む長さ。
const LOCK: Duration = Duration::from_secs(15 * 60);
/// 記録する送り主の数の上限。IP を変えながら送られても、メモリが際限なく増えないようにする。
const CAPACITY: usize = 10_000;
/// 期限の切れた記録をまとめて消す間隔。
const PRUNE_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// 送り主ごとの認証の失敗の記録。再起動で消えてよいので DB に持たない。
/// 時刻は壁の時計で測る。単調時計（`Instant`）は休止やスリープの間に進まず、
/// 休止しがちな検証用サーバーや NAS で拒否が明けなくなるため。
#[derive(Debug, Default)]
pub struct Throttle(Mutex<State>);

#[derive(Debug, Default)]
struct State {
    entries: HashMap<IpAddr, Entry>,
    pruned_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    failures: u32,
    first_failed_at: SystemTime,
    last_failed_at: SystemTime,
    locked_until: Option<SystemTime>,
}

impl Entry {
    fn new(now: SystemTime) -> Self {
        Self {
            failures: 0,
            first_failed_at: now,
            last_failed_at: now,
            locked_until: None,
        }
    }

    /// 拒否中か、失敗を数える期間の中にあるか。どちらでもなければ消してよい。
    fn is_live(&self, now: SystemTime) -> bool {
        self.locked_until.is_some_and(|until| now < until)
            || elapsed(self.first_failed_at, now) < WINDOW
    }
}

impl Throttle {
    /// 送り主からの認証を拒否中か。
    pub fn is_locked(&self, ip: IpAddr) -> bool {
        self.is_locked_at(ip, SystemTime::now())
    }

    /// 認証の失敗を数える。拒み始めたら true を返す。
    pub fn fail(&self, ip: IpAddr) -> bool {
        self.fail_at(ip, SystemTime::now())
    }

    /// 認証に成功したら、その送り主の失敗を消す。
    pub fn succeed(&self, ip: IpAddr) {
        self.lock().entries.remove(&key(ip));
    }

    fn is_locked_at(&self, ip: IpAddr, now: SystemTime) -> bool {
        self.lock()
            .entries
            .get(&key(ip))
            .and_then(|e| e.locked_until)
            .is_some_and(|until| now < until)
    }

    fn fail_at(&self, ip: IpAddr, now: SystemTime) -> bool {
        let mut state = self.lock();
        state.prune(now);
        let key = key(ip);
        if !state.entries.contains_key(&key) && state.entries.len() >= CAPACITY {
            state.evict_oldest();
        }
        let entry = state
            .entries
            .entry(key)
            .and_modify(|e| {
                if !e.is_live(now) {
                    *e = Entry::new(now);
                }
            })
            .or_insert_with(|| Entry::new(now));
        entry.failures += 1;
        entry.last_failed_at = now;
        if entry.failures > MAX_FAILURES && entry.locked_until.is_none() {
            entry.locked_until = Some(now + LOCK);
            return true;
        }
        false
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        // 記録が壊れても失うのは数え途中の回数だけなので、毒された Mutex もそのまま使う
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl State {
    fn prune(&mut self, now: SystemTime) {
        if self
            .pruned_at
            .is_some_and(|at| elapsed(at, now) < PRUNE_INTERVAL)
        {
            return;
        }
        self.entries.retain(|_, e| e.is_live(now));
        self.pruned_at = Some(now);
    }

    /// 最後の失敗が最も古い記録を消す。
    fn evict_oldest(&mut self) {
        if let Some(oldest) = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_failed_at)
            .map(|(ip, _)| *ip)
        {
            self.entries.remove(&oldest);
        }
    }
}

/// `earlier` から `now` までの長さ。時計が戻ったときは 0 とみなす。
fn elapsed(earlier: SystemTime, now: SystemTime) -> Duration {
    now.duration_since(earlier).unwrap_or_default()
}

/// 送り主の IP。`X-Forwarded-For` は、信じる設定のときだけ読む。
/// 信じると、プロキシのない構成では偽装で制限を外せてしまうため。
pub(super) struct ClientIp(pub IpAddr);

impl FromRequestParts<Arc<AppState>> for ClientIp {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let forwarded = if state.trust_forwarded_for {
            forwarded_for(&parts.headers)
        } else {
            None
        };
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip());
        // 接続元が分からないのはテストで router を直接呼ぶときだけ
        Ok(Self(
            forwarded
                .or(peer)
                .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        ))
    }
}

/// 最後の要素を使う。手前のプロキシが付け足したもので、それより前は送り主が偽装できるため。
fn forwarded_for(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get_all("x-forwarded-for")
        .iter()
        .next_back()?
        .to_str()
        .ok()?
        .rsplit(',')
        .next()?
        .trim()
        .parse()
        .ok()
}

/// 数える単位。IPv6 は一つの回線に割り当てられる /64 ごとにまとめる。
fn key(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => {
                let mask = u128::MAX << 64;
                IpAddr::V6(Ipv6Addr::from(u128::from(v6) & mask))
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    fn fail_times(t: &Throttle, addr: IpAddr, times: u32, now: SystemTime) {
        for _ in 0..times {
            t.fail_at(addr, now);
        }
    }

    #[test]
    fn locks_after_too_many_failures() {
        let t = Throttle::default();
        let now = SystemTime::now();
        let a = ip("192.0.2.1");
        fail_times(&t, a, MAX_FAILURES, now);
        assert!(!t.is_locked_at(a, now));
        assert!(t.fail_at(a, now));
        assert!(t.is_locked_at(a, now));
        assert!(!t.is_locked_at(ip("192.0.2.2"), now));
    }

    #[test]
    fn lock_expires() {
        let t = Throttle::default();
        let now = SystemTime::now();
        let a = ip("192.0.2.1");
        fail_times(&t, a, MAX_FAILURES + 1, now);
        assert!(t.is_locked_at(a, now + LOCK - Duration::from_secs(1)));
        assert!(!t.is_locked_at(a, now + LOCK));
        // 拒否が明けたら数え直す
        assert!(!t.fail_at(a, now + LOCK));
    }

    #[test]
    fn failures_outside_window_are_forgotten() {
        let t = Throttle::default();
        let now = SystemTime::now();
        let a = ip("192.0.2.1");
        fail_times(&t, a, MAX_FAILURES, now);
        assert!(!t.fail_at(a, now + WINDOW));
    }

    #[test]
    fn success_clears_failures() {
        let t = Throttle::default();
        let now = SystemTime::now();
        let a = ip("192.0.2.1");
        fail_times(&t, a, MAX_FAILURES, now);
        t.succeed(a);
        assert!(!t.fail_at(a, now));
    }

    #[test]
    fn ipv6_is_counted_per_64() {
        let t = Throttle::default();
        let now = SystemTime::now();
        fail_times(&t, ip("2001:db8:0:1::1"), MAX_FAILURES, now);
        assert!(t.fail_at(ip("2001:db8:0:1::2"), now));
        assert!(t.is_locked_at(ip("2001:db8:0:1:ffff::"), now));
        assert!(!t.is_locked_at(ip("2001:db8:0:2::1"), now));
    }

    #[test]
    fn forwarded_for_uses_last_entry() {
        let mut headers = HeaderMap::new();
        headers.append("x-forwarded-for", "198.51.100.1".parse().unwrap());
        headers.append("x-forwarded-for", "203.0.113.9, 192.0.2.1".parse().unwrap());
        assert_eq!(forwarded_for(&headers), Some(ip("192.0.2.1")));
        assert_eq!(forwarded_for(&HeaderMap::new()), None);
    }

    #[test]
    fn clock_going_back_counts_as_no_time_passed() {
        let now = SystemTime::now();
        assert_eq!(elapsed(now, now - Duration::from_secs(60)), Duration::ZERO);
        assert_eq!(elapsed(now, now + LOCK), LOCK);
    }

    #[test]
    fn ipv4_mapped_is_counted_as_ipv4() {
        assert_eq!(key(ip("::ffff:192.0.2.1")), ip("192.0.2.1"));
    }

    #[test]
    fn entries_are_bounded() {
        let t = Throttle::default();
        let now = SystemTime::now();
        for i in 0..(CAPACITY as u32 + 100) {
            let addr = IpAddr::V4(i.into());
            t.fail_at(addr, now + Duration::from_millis(u64::from(i)));
        }
        let state = t.lock();
        assert_eq!(state.entries.len(), CAPACITY);
        // 最後の失敗が古いものから消える
        assert!(!state.entries.contains_key(&IpAddr::V4(0.into())));
    }

    #[test]
    fn expired_entries_are_pruned() {
        let t = Throttle::default();
        let now = SystemTime::now();
        t.fail_at(ip("192.0.2.1"), now);
        t.fail_at(ip("192.0.2.2"), now + WINDOW + PRUNE_INTERVAL);
        let state = t.lock();
        assert_eq!(state.entries.len(), 1);
    }
}
