//! Some of the game's fields name things from its own tables (`&'static str`: an animation, a person's
//! name, a sprite). Over the network they arrive as text; `intern` turns that back into a lasting string,
//! keeping one copy of each. A host could send endless new names to eat a joiner's memory, so the total
//! is capped: past it, unknown text becomes "?".
#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::sync::Mutex;

/// The most text ever interned (bytes).
const CAP: usize = 256 * 1024;
/// The longest single string accepted.
const MAX_LEN: usize = 64;

struct Pool {
    set: HashSet<&'static str>,
    used: usize,
}

/// A `&'static str` field that crosses the network (an alias, so serde doesn't take it for borrowed text).
pub type Text = &'static str;

static POOL: Mutex<Option<Pool>> = Mutex::new(None);

pub fn intern(s: &str) -> &'static str {
    if s.is_empty() {
        return "";
    }
    let mut g = POOL.lock().unwrap_or_else(|e| e.into_inner());
    let pool = g.get_or_insert_with(|| Pool { set: HashSet::new(), used: 0 });
    if let Some(&k) = pool.set.get(s) {
        return k;
    }
    if s.len() > MAX_LEN || pool.used + s.len() > CAP {
        return "?";
    }
    let k: &'static str = Box::leak(s.to_string().into_boxed_str());
    pool.used += k.len();
    pool.set.insert(k);
    k
}

/// `#[serde(with = "crate::net::intern::static_str")]` for `&'static str` fields.
pub mod static_str {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &&'static str, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(v)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<&'static str, D::Error> {
        let s = String::deserialize(d)?;
        Ok(super::intern(&s))
    }
}

/// The same for `Option<&'static str>`.
pub mod opt_static_str {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Option<&'static str>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(t) => s.serialize_some(t),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<&'static str>, D::Error> {
        Ok(Option::<String>::deserialize(d)?.map(|s| super::intern(&s)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning_keeps_one_copy_and_has_a_ceiling() {
        let a = intern("walk");
        let b = intern(&String::from("walk"));
        assert!(std::ptr::eq(a, b));
        assert_eq!(intern(&"x".repeat(MAX_LEN + 1)), "?");
        // A flood of new names stops costing memory at the cap.
        let mut q = 0;
        for i in 0..20_000 {
            if intern(&format!("flood-name-{i:08}-padding-padding")) == "?" {
                q += 1;
            }
        }
        assert!(q > 0, "the cap was reached");
    }
}

/// `#[serde(with = "crate::net::intern::big_array")]` for arrays longer than serde's 32: sent as a list,
/// and refused unless it's exactly the right length.
pub mod big_array {
    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer, T: Serialize, const N: usize>(a: &[T; N], s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(a.iter())
    }

    pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de> + Default + Copy, const N: usize>(d: D) -> Result<[T; N], D::Error> {
        let v = Vec::<T>::deserialize(d)?;
        if v.len() != N {
            return Err(D::Error::custom("wrong length"));
        }
        let mut a = [T::default(); N];
        a.copy_from_slice(&v);
        Ok(a)
    }
}
