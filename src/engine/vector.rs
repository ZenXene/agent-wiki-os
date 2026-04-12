use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::Table;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use std::path::{Path, PathBuf};
use arrow_schema::{Schema, Field, DataType};
use arrow_array::{RecordBatch, StringArray, Float32Array, FixedSizeListArray, RecordBatchIterator};
use std::sync::Arc;
use futures::StreamExt;

pub struct VectorStore {
    db: lancedb::Connection,
    table_name: String,
    model: TextEmbedding,
}

impl VectorStore {
    pub async fn new(wiki_root: &Path) -> anyhow::Result<Self> {
        let db_path = wiki_root.join(".lancedb");
        let uri = db_path.to_string_lossy().to_string();
        let db = lancedb::connect(&uri).execute().await?;
        
        let model = TextEmbedding::try_new(InitOptions {
            model_name: EmbeddingModel::AllMiniLML6V2,
            show_download_progress: true,
            ..Default::default()
        })?;

        Ok(Self {
            db,
            table_name: "wiki_chunks".to_string(),
            model,
        })
    }

    async fn get_or_create_table(&self) -> anyhow::Result<Table> {
        let table_names = self.db.table_names().execute().await?;
        if table_names.contains(&self.table_name) {
            Ok(self.db.open_table(&self.table_name).execute().await?)
        } else {
            let schema = Arc::new(Schema::new(vec![
                Field::new("path", DataType::Utf8, false),
                Field::new("type", DataType::Utf8, false),
                Field::new("title", DataType::Utf8, false),
                Field::new("content", DataType::Utf8, false),
                Field::new("vector", DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 384), false),
            ]));
            let batch = RecordBatch::new_empty(schema.clone());
            let iterator = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());
            Ok(self.db.create_table(&self.table_name, iterator).execute().await?)
        }
    }

    pub async fn upsert_document(&self, path: &Path, content: &str, doc_type: &str, title: &str) -> anyhow::Result<()> {
        let snippet = content.chars().take(1500).collect::<String>();
        let embeddings = self.model.embed(vec![snippet], None)?;
        let vector = &embeddings[0]; 
        
        let path_str = path.to_string_lossy().to_string();
        let table = self.get_or_create_table().await?;
        
        let _ = table.delete(&format!("path = '{}'", path_str)).await;
        
        let path_arr = Arc::new(StringArray::from(vec![path_str]));
        let type_arr = Arc::new(StringArray::from(vec![doc_type.to_string()]));
        let title_arr = Arc::new(StringArray::from(vec![title.to_string()]));
        let content_arr = Arc::new(StringArray::from(vec![content.to_string()]));
        
        let flat_vec: Vec<f32> = vector.clone();
        let float_arr = Float32Array::from(flat_vec);
        let list_arr = Arc::new(FixedSizeListArray::try_new_from_values(float_arr, 384)?);
        
        let schema = table.schema().await?;
        let batch = RecordBatch::try_new(schema.clone(), vec![
            path_arr, type_arr, title_arr, content_arr, list_arr
        ])?;
        
        let iterator = RecordBatchIterator::new(vec![Ok(batch)], schema);
        table.add(iterator).execute().await?;
        Ok(())
    }

    pub async fn search(&self, query: &str, type_filter: Option<&str>, top_k: usize) -> anyhow::Result<Vec<String>> {
        let embeddings = self.model.embed(vec![query], None)?;
        let vector = &embeddings[0];
        
        let table_names = self.db.table_names().execute().await?;
        if !table_names.contains(&self.table_name) {
            return Ok(vec!["Index is empty.".to_string()]);
        }
        
        let table = self.db.open_table(&self.table_name).execute().await?;
        let mut builder = table.vector_search(vector)?;
        if let Some(t) = type_filter {
            builder = builder.filter(format!("type = '{}'", t));
        }
        let mut stream = builder.limit(top_k).execute().await?;
        
        let mut results = Vec::new();
        while let Some(batch_res) = stream.next().await {
            let batch = batch_res?;
            let path_col = batch.column_by_name("path").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
            let title_col = batch.column_by_name("title").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
            let type_col = batch.column_by_name("type").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
            let content_col = batch.column_by_name("content").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
            
            for i in 0..batch.num_rows() {
                let p = path_col.value(i);
                let t = title_col.value(i);
                let typ = type_col.value(i);
                let c = content_col.value(i);
                
                let summary = c.chars().take(150).collect::<String>();
                results.push(format!("File: {}\nTitle: [{}] (Type: {})\nSummary: {}...", p, t, typ, summary));
            }
        }
        
        Ok(results)
    }

    pub async fn get_all_documents(&self, type_filter: Option<&str>) -> anyhow::Result<Vec<(String, String, Vec<f32>)>> {
        let table_names = self.db.table_names().execute().await?;
        if !table_names.contains(&self.table_name) {
            return Ok(Vec::new());
        }
        let table = self.db.open_table(&self.table_name).execute().await?;
        let mut builder = table.query();
        if let Some(t) = type_filter {
            builder = builder.filter(format!("type = '{}'", t));
        }
        let mut stream = builder.execute().await?;
        let mut docs = Vec::new();
        while let Some(batch_res) = stream.next().await {
            let batch = batch_res?;
            let path_col = batch.column_by_name("path").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
            let content_col = batch.column_by_name("content").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
            let vector_col = batch.column_by_name("vector").unwrap().as_any().downcast_ref::<FixedSizeListArray>().unwrap();
            let values = vector_col.values().as_any().downcast_ref::<Float32Array>().unwrap();

            for i in 0..batch.num_rows() {
                let p = path_col.value(i).to_string();
                let c = content_col.value(i).to_string();
                let start = i * 384;
                let mut v = Vec::with_capacity(384);
                for j in 0..384 {
                    v.push(values.value(start + j));
                }
                docs.push((p, c, v));
            }
        }
        Ok(docs)
    }
}
