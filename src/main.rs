use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::middleware::{Compress, Logger};
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use env_logger::Env;
use regex::Regex;
use serde::{Deserialize, Serialize};

const MAX_KEY_SIZE: usize = 256;
const MAX_VALUE_SIZE: usize = 10_485_760;
const BACKUP_THRESHOLD: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyMetadata {
    value: String,
    created_at: u64,
    updated_at: u64,
    access_count: u64,
}

impl KeyMetadata {
    fn new(value: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            value,
            created_at: now,
            updated_at: now,
            access_count: 0,
        }
    }
}

#[derive(Serialize)]
struct KeyInfo {
    key: String,
    size: usize,
    created_at: u64,
    updated_at: u64,
    access_count: u64,
}

#[derive(Serialize)]
struct StoreStats {
    total_keys: usize,
    total_size_bytes: usize,
    operations_count: u64,
    uptime_seconds: u64,
}

struct KvStore {
    data: Mutex<HashMap<String, KeyMetadata>>,
    file: Mutex<File>,
    operations_count: Mutex<u64>,
    start_time: u64,
}

impl KvStore {
    fn new() -> Self {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("kvstore.db")
            .unwrap();

        let mut data = HashMap::new();
        let mut reader = BufReader::new(&file);
        let mut buffer = Vec::new();

        if reader.read_to_end(&mut buffer).is_ok() {
            let mut pos = 0;
            while pos < buffer.len() {
                if buffer.len() - pos < 16 {
                    break;
                }

                let key_size =
                    u64::from_le_bytes(buffer[pos..pos + 8].try_into().unwrap()) as usize;
                pos += 8;
                let value_size =
                    u64::from_le_bytes(buffer[pos..pos + 8].try_into().unwrap()) as usize;
                pos += 8;

                if pos + key_size + value_size > buffer.len() {
                    break;
                }

                let key = String::from_utf8_lossy(&buffer[pos..pos + key_size]).to_string();
                pos += key_size;
                let value = String::from_utf8_lossy(&buffer[pos..pos + value_size]).to_string();
                pos += value_size;

                if !value.is_empty() {
                    data.insert(key, KeyMetadata::new(value));
                } else {
                    data.remove(&key);
                }
            }
        }

        file.seek(SeekFrom::End(0)).unwrap();
        
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            data: Mutex::new(data),
            file: Mutex::new(file),
            operations_count: Mutex::new(0),
            start_time,
        }
    }

    fn validate_key(&self, key: &str) -> Result<(), String> {
        if key.is_empty() {
            return Err("Key cannot be empty".to_string());
        }
        if key.len() > MAX_KEY_SIZE {
            return Err(format!("Key exceeds maximum size of {} bytes", MAX_KEY_SIZE));
        }
        Ok(())
    }

    fn validate_value(&self, value: &str) -> Result<(), String> {
        if value.len() > MAX_VALUE_SIZE {
            return Err(format!("Value exceeds maximum size of {} bytes", MAX_VALUE_SIZE));
        }
        Ok(())
    }

    fn increment_operations(&self) {
        let mut count = self.operations_count.lock().unwrap();
        *count += 1;
    }

    fn set(&self, key: String, value: String) -> Result<(), String> {
        self.validate_key(&key)?;
        self.validate_value(&value)?;

        let mut data = self.data.lock().unwrap();
        let mut file = self.file.lock().unwrap();

        let metadata = KeyMetadata::new(value.clone());
        data.insert(key.clone(), metadata);

        let key_bytes = key.as_bytes();
        let value_bytes = value.as_bytes();
        file.write_all(&(key_bytes.len() as u64).to_le_bytes())
            .map_err(|e| e.to_string())?;
        file.write_all(&(value_bytes.len() as u64).to_le_bytes())
            .map_err(|e| e.to_string())?;
        file.write_all(key_bytes).map_err(|e| e.to_string())?;
        file.write_all(value_bytes).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;

        self.increment_operations();
        Ok(())
    }

    fn update(&self, key: &str, value: String) -> Result<(), String> {
        self.validate_key(key)?;
        self.validate_value(&value)?;

        let mut data = self.data.lock().unwrap();
        
        if let Some(metadata) = data.get_mut(key) {
            metadata.value = value.clone();
            metadata.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            drop(data);
            self.compact();
            self.increment_operations();
            Ok(())
        } else {
            Err("Key does not exist".to_string())
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        let mut data = self.data.lock().unwrap();
        if let Some(metadata) = data.get_mut(key) {
            metadata.access_count += 1;
            self.increment_operations();
            Some(metadata.value.clone())
        } else {
            None
        }
    }

    fn get_info(&self, key: &str) -> Option<KeyInfo> {
        let data = self.data.lock().unwrap();
        data.get(key).map(|metadata| KeyInfo {
            key: key.to_string(),
            size: metadata.value.len(),
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
            access_count: metadata.access_count,
        })
    }

    fn list_keys(&self, prefix: Option<&str>, limit: Option<usize>) -> Vec<String> {
        let data = self.data.lock().unwrap();
        let mut keys: Vec<String> = data
            .keys()
            .filter(|k| {
                if let Some(p) = prefix {
                    k.starts_with(p)
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        
        keys.sort();
        
        if let Some(l) = limit {
            keys.truncate(l);
        }
        
        keys
    }

    fn get_stats(&self) -> StoreStats {
        let data = self.data.lock().unwrap();
        let operations = *self.operations_count.lock().unwrap();
        let total_size: usize = data.values().map(|m| m.value.len()).sum();
        let uptime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - self.start_time;

        StoreStats {
            total_keys: data.len(),
            total_size_bytes: total_size,
            operations_count: operations,
            uptime_seconds: uptime,
        }
    }

    fn compact(&self) {
        let data = self.data.lock().unwrap();
        let mut file = self.file.lock().unwrap();

        file.set_len(0).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();

        for (key, metadata) in data.iter() {
            let key_bytes = key.as_bytes();
            let value_bytes = metadata.value.as_bytes();
            file.write_all(&(key_bytes.len() as u64).to_le_bytes())
                .unwrap();
            file.write_all(&(value_bytes.len() as u64).to_le_bytes())
                .unwrap();
            file.write_all(key_bytes).unwrap();
            file.write_all(value_bytes).unwrap();
        }
        file.flush().unwrap();
    }

    fn delete(&self, key: &str) -> bool {
        let mut data = self.data.lock().unwrap();
        if data.remove(key).is_some() {
            drop(data);
            self.compact();
            self.increment_operations();
            true
        } else {
            false
        }
    }

    fn delete_by_prefix(&self, prefix: &str) -> usize {
        let mut data = self.data.lock().unwrap();
        let keys_to_remove: Vec<String> = data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        
        let count = keys_to_remove.len();
        for key in keys_to_remove {
            data.remove(&key);
        }
        
        drop(data);
        if count > 0 {
            self.compact();
            self.increment_operations();
        }
        count
    }

    fn find_values_by_regex(&self, pattern: &str) -> Result<Vec<String>, regex::Error> {
        let re = Regex::new(pattern)?;
        let data = self.data.lock().unwrap();
        let values: Vec<String> = data
            .iter()
            .filter(|(key, _)| re.is_match(key))
            .map(|(_, metadata)| metadata.value.clone())
            .collect();
        Ok(values)
    }

    fn exists(&self, key: &str) -> bool {
        let data = self.data.lock().unwrap();
        data.contains_key(key)
    }

    fn batch_set(&self, items: Vec<(String, String)>) -> Result<usize, String> {
        let mut success_count = 0;
        for (key, value) in items {
            if self.set(key, value).is_ok() {
                success_count += 1;
            }
        }
        Ok(success_count)
    }

    fn backup(&self) -> Result<(), String> {
        let data = self.data.lock().unwrap();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let backup_name = format!("kvstore_backup_{}.db", timestamp);
        let mut backup_file = File::create(&backup_name)
            .map_err(|e| e.to_string())?;

        for (key, metadata) in data.iter() {
            let key_bytes = key.as_bytes();
            let value_bytes = metadata.value.as_bytes();
            backup_file.write_all(&(key_bytes.len() as u64).to_le_bytes())
                .map_err(|e| e.to_string())?;
            backup_file.write_all(&(value_bytes.len() as u64).to_le_bytes())
                .map_err(|e| e.to_string())?;
            backup_file.write_all(key_bytes).map_err(|e| e.to_string())?;
            backup_file.write_all(value_bytes).map_err(|e| e.to_string())?;
        }
        backup_file.flush().map_err(|e| e.to_string())?;
        
        Ok(())
    }
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

async fn get_stats(store: web::Data<KvStore>) -> impl Responder {
    let stats = store.get_stats();
    HttpResponse::Ok().json(stats)
}

async fn get_all_keys(
    store: web::Data<KvStore>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let prefix = query.get("prefix").map(|s| s.as_str());
    let limit = query.get("limit").and_then(|s| s.parse::<usize>().ok());
    
    let keys = store.list_keys(prefix, limit);
    if keys.is_empty() {
        HttpResponse::NotFound().json(vec![] as Vec<String>)
    } else {
        HttpResponse::Ok().json(keys)
    }
}

async fn get_key(store: web::Data<KvStore>, path: web::Path<String>) -> impl Responder {
    let key = path.into_inner();
    match store.get(&key) {
        Some(value) => HttpResponse::Ok().body(value),
        None => HttpResponse::NotFound().body("Key not found"),
    }
}

async fn get_key_info(store: web::Data<KvStore>, path: web::Path<String>) -> impl Responder {
    let key = path.into_inner();
    match store.get_info(&key) {
        Some(info) => HttpResponse::Ok().json(info),
        None => HttpResponse::NotFound().body("Key not found"),
    }
}

async fn check_key_exists(store: web::Data<KvStore>, path: web::Path<String>) -> impl Responder {
    let key = path.into_inner();
    if store.exists(&key) {
        HttpResponse::Ok().json(serde_json::json!({"exists": true}))
    } else {
        HttpResponse::Ok().json(serde_json::json!({"exists": false}))
    }
}

async fn put_key(
    store: web::Data<KvStore>,
    path: web::Path<String>,
    body: String,
) -> impl Responder {
    let key = path.into_inner();
    if store.exists(&key) {
        return HttpResponse::Conflict().body("Key already exists");
    }
    
    match store.set(key, body) {
        Ok(_) => HttpResponse::Created().body("OK"),
        Err(e) => HttpResponse::BadRequest().body(e),
    }
}

async fn update_key(
    store: web::Data<KvStore>,
    path: web::Path<String>,
    body: String,
) -> impl Responder {
    let key = path.into_inner();
    match store.update(&key, body) {
        Ok(_) => HttpResponse::Ok().body("OK"),
        Err(e) => HttpResponse::BadRequest().body(e),
    }
}

async fn delete_key(store: web::Data<KvStore>, path: web::Path<String>) -> impl Responder {
    let key = path.into_inner();
    if store.delete(&key) {
        HttpResponse::Ok().body("OK")
    } else {
        HttpResponse::NotFound().body("Key not found")
    }
}

async fn delete_by_prefix(store: web::Data<KvStore>, path: web::Path<String>) -> impl Responder {
    let prefix = path.into_inner();
    let count = store.delete_by_prefix(&prefix);
    HttpResponse::Ok().json(serde_json::json!({
        "deleted_count": count
    }))
}

async fn get_values_by_regex(store: web::Data<KvStore>, path: web::Path<String>) -> impl Responder {
    let pattern = path.into_inner();
    match store.find_values_by_regex(&pattern) {
        Ok(values) => {
            if values.is_empty() {
                HttpResponse::NotFound().body("No values matched the pattern")
            } else {
                HttpResponse::Ok().json(values)
            }
        }
        Err(e) => HttpResponse::BadRequest().body(format!("Invalid regex pattern: {}", e)),
    }
}

#[derive(Deserialize)]
struct BatchItem {
    key: String,
    value: String,
}

async fn batch_set(
    store: web::Data<KvStore>,
    items: web::Json<Vec<BatchItem>>,
) -> impl Responder {
    let items: Vec<(String, String)> = items
        .into_inner()
        .into_iter()
        .map(|item| (item.key, item.value))
        .collect();
    
    match store.batch_set(items) {
        Ok(count) => HttpResponse::Ok().json(serde_json::json!({
            "success_count": count
        })),
        Err(e) => HttpResponse::BadRequest().body(e),
    }
}

async fn create_backup(store: web::Data<KvStore>) -> impl Responder {
    match store.backup() {
        Ok(_) => HttpResponse::Ok().body("Backup created successfully"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Backup failed: {}", e)),
    }
}

async fn manual_compact(store: web::Data<KvStore>) -> impl Responder {
    store.compact();
    HttpResponse::Ok().body("Database compacted successfully")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let store = web::Data::new(KvStore::new());
    println!("Server running at http://127.0.0.1:8080");
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    
    HttpServer::new(move || {
        App::new()
            .app_data(store.clone())
            .wrap(Compress::default())
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .route("/health", web::get().to(health_check))
            .route("/stats", web::get().to(get_stats))
            .route("/kv/", web::get().to(get_all_keys))
            .route("/kv/{key}", web::get().to(get_key))
            .route("/kv/{key}/info", web::get().to(get_key_info))
            .route("/kv/{key}/exists", web::get().to(check_key_exists))
            .route("/kv/{key}", web::post().to(put_key))
            .route("/kv/{key}", web::put().to(update_key))
            .route("/kv/{key}", web::delete().to(delete_key))
            .route("/kv/prefix/{prefix}", web::delete().to(delete_by_prefix))
            .route("/kv/r/{regex}", web::get().to(get_values_by_regex))
            .route("/batch", web::post().to(batch_set))
            .route("/backup", web::post().to(create_backup))
            .route("/compact", web::post().to(manual_compact))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
