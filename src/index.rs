use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::{AllQuery, QueryParser};
use tantivy::schema::{Field, Schema, SchemaBuilder, Value, FAST, STORED, STRING, TEXT};
use tantivy::{DocAddress, Index, TantivyDocument};

use crate::models::NoteRecord;
use crate::repo::FukuraRepo;

#[derive(Debug, Clone, Copy, clap::ValueEnum, Serialize, Deserialize)]
pub enum SearchSort {
    #[clap(name = "relevance")]
    Relevance,
    #[clap(name = "updated")]
    Updated,
    #[clap(name = "likes")]
    Likes,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub object_id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub summary: String,
    pub updated_at: DateTime<Utc>,
    pub author: String,
    pub likes: u32,
    pub score: f32,
    pub privacy: String,
}

#[derive(Clone)]
pub struct SearchIndex {
    index: Index,
    fields: Fields,
}

#[derive(Clone)]
struct Fields {
    object_id: Field,
    title: Field,
    body: Field,
    tags: Field,
    summary: Field,
    updated_at: Field,
    likes: Field,
    author: Field,
    privacy: Field,
}

impl SearchIndex {
    pub fn open_or_create(repo: &FukuraRepo) -> Result<Self> {
        let path = repo.index_dir();
        Self::open_or_create_in(path)
    }

    fn open_or_create_in(path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&path)?;
        let schema = build_schema();
        let index = if path.read_dir()?.next().is_some() {
            Index::open_in_dir(&path).context("Failed to open search index")?
        } else {
            Index::create_in_dir(&path, schema.clone()).context("Failed to create search index")?
        };
        let actual_schema = index.schema();
        let fields = Fields::new(&actual_schema)?;
        Ok(Self { index, fields })
    }

    pub fn add_note(&self, record: &NoteRecord) -> Result<()> {
        let mut writer = self.index.writer(50_000_000)?;
        let mut document = TantivyDocument::new();
        document.add_text(self.fields.object_id, &record.object_id);
        document.add_text(self.fields.title, &record.note.title);
        document.add_text(self.fields.body, &record.note.body);
        for tag in &record.note.tags {
            document.add_text(self.fields.tags, tag);
        }
        document.add_text(self.fields.summary, make_summary(&record.note.body));
        document.add_text(self.fields.author, &record.note.author.name);
        document.add_text(self.fields.privacy, format_privacy(&record.note.privacy));
        document.add_i64(self.fields.updated_at, record.note.updated_at.timestamp());
        document.add_i64(self.fields.likes, total_likes(&record.note) as i64);
        writer.add_document(document)?;
        writer.commit()?;
        Ok(())
    }

    /// Add multiple notes efficiently in batch
    pub fn add_notes_batch(&self, records: &[NoteRecord]) -> Result<()> {
        let mut writer = self.index.writer(50_000_000)?;
        for record in records {
            let mut document = TantivyDocument::new();
            document.add_text(self.fields.object_id, &record.object_id);
            document.add_text(self.fields.title, &record.note.title);
            document.add_text(self.fields.body, &record.note.body);
            for tag in &record.note.tags {
                document.add_text(self.fields.tags, tag);
            }
            document.add_text(self.fields.summary, make_summary(&record.note.body));
            document.add_text(self.fields.author, &record.note.author.name);
            document.add_text(self.fields.privacy, format_privacy(&record.note.privacy));
            document.add_i64(self.fields.updated_at, record.note.updated_at.timestamp());
            document.add_i64(self.fields.likes, total_likes(&record.note) as i64);
            writer.add_document(document)?;
        }
        writer.commit()?;
        Ok(())
    }

    pub fn search(&self, query: &str, limit: usize, sort: SearchSort) -> Result<Vec<SearchHit>> {
        let limit = limit.max(1);
        let reader = self.index.reader()?;
        reader.reload()?;
        let searcher = reader.searcher();
        let query_text = query.trim();
        let query: Box<dyn tantivy::query::Query> = if query_text.is_empty() {
            Box::new(AllQuery)
        } else {
            let parser = QueryParser::for_index(
                &self.index,
                vec![self.fields.title, self.fields.body, self.fields.tags],
            );
            parser.parse_query(query_text)?
        };
        let top_docs = searcher.search(query.as_ref(), &TopDocs::with_limit(limit))?;
        let mut hits = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved: TantivyDocument = searcher.doc(doc_address)?;
            let object_id = retrieved
                .get_first(self.fields.object_id)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let title = retrieved
                .get_first(self.fields.title)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let tags = retrieved
                .get_all(self.fields.tags)
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            let summary = retrieved
                .get_first(self.fields.summary)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let author = retrieved
                .get_first(self.fields.author)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let likes = retrieved
                .get_first(self.fields.likes)
                .and_then(|v| v.as_i64())
                .unwrap_or_default() as u32;
            let updated_at_ts = retrieved
                .get_first(self.fields.updated_at)
                .and_then(|v| v.as_i64())
                .unwrap_or_default();
            let privacy = retrieved
                .get_first(self.fields.privacy)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let updated_at = Utc
                .timestamp_opt(updated_at_ts, 0)
                .single()
                .unwrap_or_else(Utc::now);
            hits.push(SearchHit {
                object_id,
                title,
                tags,
                summary,
                updated_at,
                author,
                likes,
                score,
                privacy,
            });
        }
        // Optimize sorting for large result sets
        match sort {
            SearchSort::Relevance => {
                // Already sorted by relevance score from tantivy
            }
            SearchSort::Updated => {
                // Use unstable sort for better performance with large datasets
                hits.sort_unstable_by(|a, b| b.updated_at.cmp(&a.updated_at));
            }
            SearchSort::Likes => {
                hits.sort_unstable_by(|a, b| b.likes.cmp(&a.likes));
            }
        }
        Ok(hits)
    }

    pub fn collect_tags(&self) -> Result<Vec<String>> {
        let reader = self.index.reader()?;
        reader.reload()?;
        let searcher = reader.searcher();
        let mut unique = BTreeSet::new();
        for (segment_ord, segment_reader) in searcher.segment_readers().iter().enumerate() {
            let max_doc = segment_reader.max_doc();
            for doc_id in 0..max_doc {
                let address = DocAddress::new(segment_ord as u32, doc_id);
                let retrieved: TantivyDocument = searcher.doc(address)?;
                for value in retrieved.get_all(self.fields.tags) {
                    if let Some(text) = value.as_str() {
                        unique.insert(text.to_string());
                    }
                }
            }
        }
        Ok(unique.into_iter().collect())
    }
}

impl Fields {
    fn new(schema: &Schema) -> Result<Self> {
        Ok(Self {
            object_id: schema
                .get_field("object_id")
                .context("object_id field missing")?,
            title: schema.get_field("title").context("title field missing")?,
            body: schema.get_field("body").context("body field missing")?,
            tags: schema.get_field("tags").context("tags field missing")?,
            summary: schema
                .get_field("summary")
                .context("summary field missing")?,
            updated_at: schema
                .get_field("updated_at")
                .context("updated_at field missing")?,
            likes: schema.get_field("likes").context("likes field missing")?,
            author: schema.get_field("author").context("author field missing")?,
            privacy: schema
                .get_field("privacy")
                .context("privacy field missing")?,
        })
    }
}

fn build_schema() -> Schema {
    let mut builder = SchemaBuilder::default();
    builder.add_text_field("object_id", STRING | STORED);
    builder.add_text_field("title", TEXT | STORED);
    builder.add_text_field("body", TEXT);
    builder.add_text_field("tags", TEXT | STORED);
    builder.add_text_field("summary", STORED);
    builder.add_text_field("author", STRING | STORED);
    builder.add_text_field("privacy", STRING | STORED);
    builder.add_i64_field("updated_at", FAST | STORED);
    builder.add_i64_field("likes", FAST | STORED);
    builder.build()
}

pub fn make_summary(body: &str) -> String {
    let mut lines = body.lines().filter(|line| !line.trim().is_empty());
    let preview: Vec<&str> = lines.by_ref().take(3).collect();
    let summary = preview.join(" ");
    if summary.len() > 160 {
        format!("{}…", &summary[..160])
    } else {
        summary
    }
}

fn total_likes(note: &crate::models::Note) -> u32 {
    note.solutions.iter().map(|s| s.likes).sum()
}

fn format_privacy(privacy: &crate::models::Privacy) -> String {
    format!("{:?}", privacy).to_lowercase()
}
