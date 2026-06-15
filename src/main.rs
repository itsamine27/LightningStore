use std::sync::Arc;
use dashmap::DashMap;
use tokio::task;

type Key = String;
type Store = Arc<DashMap<Key, Value>>;


pub enum Value {
    Map(Arc<DashMap<String, Value>>),
    Str(String),
    Int(i64),
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(i) => write!(f, "Int({})", i),
            Value::Str(s) => write!(f, "Str({})", s),
            Value::Map(m) => write!(f, "Map(len={})", m.len()),
        }
    }
}


impl From<i64> for Value {
    fn from(v: i64) -> Self { Value::Int(v) }
}
impl From<&str> for Value {
    fn from(s: &str) -> Self { Value::Str(s.to_string()) }
}
impl From<String> for Value {
    fn from(s: String) -> Self { Value::Str(s) }
}
impl From<Arc<DashMap<String, Value>>> for Value {
    fn from(m: Arc<DashMap<String, Value>>) -> Self { Value::Map(m) }
}

impl Value {
    pub fn as_i64(&self) -> Option<i64> {
        if let Value::Int(n) = self { Some(*n) } else { None }
    }
    pub fn as_str(&self) -> Option<&str> {
        if let Value::Str(s) = self { Some(s.as_str()) } else { None }
    }
    pub fn as_map(&self) -> Option<&Arc<DashMap<String, Value>>> {
        if let Value::Map(m) = self { Some(m) } else { None }
    }
}

#[tokio::main]
async fn main() {
    let store: Store = Arc::new(DashMap::new());

    // Create an inner concurrent map for a hash-like value
    let inner = Arc::new(DashMap::new());
    inner.insert("key1".to_string(), Value::from(100i64));
    inner.insert("key2".to_string(), Value::from("value"));

    // Insert into top-level store
    store.insert("my_table".to_string(), Value::from(inner.clone()));
    store.insert("greeting".to_string(), Value::from("hello"));
    store.insert("count".to_string(), Value::from(42i64));

    // Read typed values
    if let Some(v) = store.get("count") {
        if let Some(n) = v.as_i64() {
            println!("count: {}", n);
        }
    }

    if let Some(v) = store.get("greeting") {
        if let Some(s) = v.as_str() {
            println!("Grt: {}", s);
        }
    }

    if let Some(v) = store.get("my_table") {
        if let Some(map_arc) = v.as_map() {
            // iterate safely over the inner dashmap
            for r in map_arc.iter() {
                println!("{} => {:?}", r.key(), r.value());
            }
        }
    }

    let s1 = Arc::clone(&store);
    let writer = task::spawn(async move {
        for i in 0..3 {
            let inner = Arc::new(DashMap::new());
            inner.insert("key1".to_string(), Value::from(i * 100));
            inner.insert("key2".to_string(), Value::from(format!("value{}", i)));
            s1.insert("my_table".to_string(), Value::from(inner));
            println!("table {}", i);
            // simulate work
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    });

    let s2 = Arc::clone(&store);
    let reader = task::spawn(async move {
        for _ in 0..6 {
            if let Some(v) = s2.get("my_table") {
                if let Some(map_arc) = v.as_map() {
                    for r in map_arc.iter() {
                        println!("read {} => {:?}", r.key(), r.value());
                    }
                }
            } else {
                println!("my_table not present");
            }
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }
    });

    let _ = tokio::join!(writer, reader);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use rand::Rng;

    #[tokio::test]
    async fn test_thousand_queries_performance() {
        // 1. Initialize the store and pre-populate it with some keys
        let store: Store = Arc::new(DashMap::new());
        store.insert("count".to_string(), Value::from(42i64));
        store.insert("greeting".to_string(), Value::from("hello"));
        
        let inner_map = Arc::new(DashMap::new());
        inner_map.insert("nested_key".to_string(), Value::from(100i64));
        store.insert("my_table".to_string(), Value::from(inner_map));

        // 2. Start the timer
        let start_time = Instant::now();

        // 3. Spawn concurrent workers to perform queries
        let mut workers = vec![];
        let num_queries_per_worker = 100;
        let num_workers = 10; // 10 workers * 100 queries = 1,000 total queries

        for worker_id in 0..num_workers {
            let store_clone = Arc::clone(&store);
            
            let handle = tokio::spawn(async move {
                let mut rng = rand::thread_rng();
                
                for i in 0..num_queries_per_worker {
                    // 90% Reads, 10% Writes
                    if rng.gen_bool(0.9) {
                        // --- SIMULATE READ ---
                        let target_key = if i % 2 == 0 { "count" } else { "greeting" };
                        if let Some(ref_multi) = store_clone.get(target_key) {
                            // Access the value to make sure the read isn't optimized away
                            let _ = ref_multi.value(); 
                        }
                    } else {
                        // --- SIMULATE WRITE ---
                        let dynamic_key = format!("worker_{}_key_{}", worker_id, i);
                        store_clone.insert(dynamic_key, Value::from(i as i64));
                    }
                }
            });
            
            workers.push(handle);
        }

        // 4. Wait for all 1,000 queries to complete
        for worker in workers {
            worker.await.expect("Worker task panicked");
        }

        // 5. Calculate and print duration
        let duration = start_time.elapsed();
        println!("\n=========== PERF RESULT ===========");
        println!("Time taken for 1,000 queries: {:?}", duration);
        println!("Average time per query: {:?}", duration / 1000);
        println!("===================================\n");

        // Basic assertion just to ensure the test passes
        assert!(duration.as_millis() < 500, "Performance is unusually slow!");
    }
}