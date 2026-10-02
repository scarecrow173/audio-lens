use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const BASE62: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const ID_LEN: usize = 22;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
static RNG_STATE: AtomicU64 = AtomicU64::new(0);

fn seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    nanos ^ u64::from(std::process::id()).rotate_left(17)
}

fn next_u64() -> u64 {
    // xorshift64* — 一意性確保が目的であり暗号強度は不要
    let mut state = RNG_STATE.load(Ordering::Relaxed);
    if state == 0 {
        state = seed();
    }
    let mut x = state ^ SEQUENCE.fetch_add(1, Ordering::Relaxed).rotate_left(7);
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    RNG_STATE.store(x, Ordering::Relaxed);
    x.wrapping_mul(0x2545_F491_4F6C_DD1D) ^ seed()
}

fn encode_id(prefix: &str) -> String {
    let mut buf = String::with_capacity(prefix.len() + 1 + ID_LEN);
    buf.push_str(prefix);
    buf.push('-');
    while buf.len() < prefix.len() + 1 + ID_LEN {
        let mut v = next_u64();
        for _ in 0..10 {
            if buf.len() == prefix.len() + 1 + ID_LEN {
                break;
            }
            buf.push(BASE62[(v % 62) as usize] as char);
            v /= 62;
        }
    }
    buf
}

/// Validate that `s` looks like `<PREFIX>-<22 base62 chars>`.
fn validate(prefix: &str, s: &str) -> bool {
    let Some(rest) = s.strip_prefix(&format!("{prefix}-")) else {
        return false;
    };
    rest.len() == ID_LEN && rest.bytes().all(|b| BASE62.contains(&b))
}

macro_rules! id_type {
    ($name:ident, $prefix:literal) => {
        /// Stable identifier (`<prefix>-<22 base62 chars>`).
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Issue a fresh identifier.
            pub fn new() -> Self {
                Self(encode_id($prefix))
            }

            /// Wrap an existing string after validating its shape.
            pub fn parse(s: impl Into<String>) -> Result<Self, IdError> {
                let s = s.into();
                if validate($prefix, &s) {
                    Ok(Self(s))
                } else {
                    Err(IdError {
                        expected: $prefix,
                        found: s,
                    })
                }
            }

            /// Borrow the underlying string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(JobId, "JOB");
id_type!(ArtifactId, "ART");
id_type!(InputId, "IN");

/// Error returned when an id does not match the expected shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdError {
    /// Expected prefix.
    pub expected: &'static str,
    /// Offending value.
    pub found: String,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid id: expected prefix {}-, found {:?}",
            self.expected, self.found
        )
    }
}

impl std::error::Error for IdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_match_shape() {
        let id = JobId::new();
        assert!(JobId::parse(id.as_str()).is_ok());
    }

    #[test]
    fn generated_ids_are_unique() {
        let ids: Vec<_> = (0..1000).map(|_| JobId::new()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn parse_rejects_wrong_prefix() {
        assert!(JobId::parse(ArtifactId::new().as_str()).is_err());
        assert!(JobId::parse("JOB-short").is_err());
        assert!(JobId::parse("nonsense").is_err());
    }
}
