use sealed_boxes::PublicKey;
use serde::{Serialize, Deserialize};
use serde::{Deserializer, Serializer, de::Error};
use std::borrow::{Borrow, Cow};
use std::fmt;
use std::sync::Arc;
use util::base64;

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId {
    #[serde(with = "self")]
    val: Arc<[u8]>
}

impl AgentId {
    pub fn from_base64(s: &str) -> Option<Self> {
        let b = base64::decode(s)?;
        Some(AgentId::from(&*b))
    }

    pub fn to_base64(&self) -> String {
        base64::encode(&self.val)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &*self.val
    }
}

impl From<PublicKey> for AgentId {
    fn from(k: PublicKey) -> Self {
        AgentId::from(&k.as_bytes()[..])
    }
}

impl From<&[u8]> for AgentId {
    fn from(id: &[u8]) -> Self {
        AgentId { val: Arc::from(Vec::from(id)) }
    }
}

impl fmt::Debug for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&base64::encode(&self.val))
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

fn serialize<S: Serializer>(val: &Arc<[u8]>, s: S) -> Result<S::Ok, S::Error> {
    base64::encode(val).serialize(s)
}

fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Arc<[u8]>, D::Error> {
    let s = <Cow<'de, str>>::deserialize(d)?;
    let v = base64::decode(s.borrow()).ok_or_else(|| Error::custom("invalid base64"))?;
    Ok(Arc::from(v))
}

