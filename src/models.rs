use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Privacy {
    Private,
    Org,
    Public,
}

impl Default for Privacy {
    fn default() -> Self {
        Privacy::Private
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Author {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub meta: BTreeMap<String, String>,
    #[serde(default)]
    pub solutions: Vec<Solution>,
    #[serde(default)]
    pub privacy: Privacy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: Author,
}

impl Note {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        let envelope = NoteEnvelope {
            schema: "fuku.note".to_owned(),
            version: 1,
            note: self.clone(),
        };
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&envelope, &mut buf)?;
        Ok(buf)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Solution {
    pub steps: Vec<String>,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub likes: u32,
    #[serde(default)]
    pub adopted: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEnvelope {
    pub schema: String,
    pub version: u32,
    pub note: Note,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteRecord {
    pub object_id: String,
    pub note: Note,
}
