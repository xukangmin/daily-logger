// logger.rs

use chrono::{Datelike, Local};
use log::{kv::Key, Level, Log, Metadata, Record};
use once_cell::sync::Lazy;

use std::{
    collections::{HashMap, VecDeque},
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::Mutex,
};

const MAX_CACHE_SIZE: usize = 32;
static LOGGER: Lazy<DailyLogger> = Lazy::new(DailyLogger::new);
static FILE_CACHE: Lazy<Mutex<FileCache>> =
    Lazy::new(|| Mutex::new(FileCache::new(MAX_CACHE_SIZE)));

pub fn init_logger(stdout_level: log::LevelFilter, file_level: log::LevelFilter, base_path: impl Into<PathBuf>) {
    LOGGER.set_base_path(base_path.into());
    LOGGER.set_levels(stdout_level, file_level);
    log::set_logger(&*LOGGER).unwrap();
    log::set_max_level(stdout_level.max(file_level));
}

pub struct DailyLogger {
    base_path: Mutex<Option<PathBuf>>,
    stdout_level: Mutex<log::LevelFilter>,
    file_level: Mutex<log::LevelFilter>,
}

impl DailyLogger {
    fn new() -> Self {
        Self {
            base_path: Mutex::new(None),
            stdout_level: Mutex::new(log::LevelFilter::Info),
            file_level: Mutex::new(log::LevelFilter::Info),
        }
    }

    fn set_base_path(&self, path: PathBuf) {
        let mut base = self.base_path.lock().unwrap();
        *base = Some(path);
    }

    fn get_base_path(&self) -> Option<PathBuf> {
        self.base_path.lock().unwrap().clone()
    }

    fn set_levels(&self, stdout_level: log::LevelFilter, file_level: log::LevelFilter) {
        *self.stdout_level.lock().unwrap() = stdout_level;
        *self.file_level.lock().unwrap() = file_level;
    }
}

impl Log for DailyLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let stdout_level = *self.stdout_level.lock().unwrap();
        let file_level = *self.file_level.lock().unwrap();

        let now = Local::now();
        let mut log_entry: String = format!(
            "{}-{}|[{}]: {}",
            now.to_rfc3339(),
            record.level(),
            record.target(),
            record.args()
        );


        
        let key_values = record.key_values();
        if let Some(uuid) = key_values.get(Key::from("uuid")) {
            let file_name = format!("order_{uuid}.log");

            log_entry = format!(
                "{}-{}|[{}]<{}>:{}",
                now.to_rfc3339(),
                record.level(),
                record.target(),
                uuid,
                record.args()
            );

            if record.level() <= file_level {
                write_to_file(&file_name, &log_entry, self.get_base_path());
            }
        }

        if record.level() <= stdout_level {
            let colored_entry = match record.level() {
                Level::Error => format!("\x1b[31m{log_entry}\x1b[0m"),
                Level::Warn => format!("\x1b[33m{log_entry}\x1b[0m"),
                Level::Info => format!("\x1b[32m{log_entry}\x1b[0m"),
                Level::Debug => format!("\x1b[37m{log_entry}\x1b[0m"),
                Level::Trace => format!("\x1b[90m{log_entry}\x1b[0m"),
            };
            println!("{colored_entry}");
        }

        if record.level() <= file_level {
            let date_log_name = format!("log_{}_{}_{}.log", now.year(), now.month(), now.day());
            write_to_file(&date_log_name, &log_entry, self.get_base_path());
        }
    }

    fn flush(&self) {}
}

fn write_to_file(file_name: &str, log_entry: &str, base_path: Option<PathBuf>) {
    let mut cache = FILE_CACHE.lock().unwrap();
    let full_path = base_path.map(|base| base.join(file_name)).unwrap_or_else(|| PathBuf::from(file_name));
    let writer = cache.get_or_open(full_path);
    let _ = writeln!(writer, "{log_entry}");
    let _ = writer.flush();
}

struct FileCache {
    max_size: usize,
    files: HashMap<PathBuf, BufWriter<File>>,
    order: VecDeque<PathBuf>,
}

impl FileCache {
    fn new(max_size: usize) -> Self {
        Self {
            max_size,
            files: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get_or_open(&mut self, path: PathBuf) -> &mut BufWriter<File> {
        if self.files.contains_key(&path) {
            self.order.retain(|f| f != &path);
            self.order.push_back(path.clone());
        } else {
            if self.files.len() >= self.max_size {
                if let Some(oldest) = self.order.pop_front() {
                    self.files.remove(&oldest);
                }
            }

            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .expect("Failed to open log file");

            let writer = BufWriter::with_capacity(1024, file);
            self.files.insert(path.clone(), writer);
            self.order.push_back(path.clone());
        }

        self.files.get_mut(&path).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, trace, warn};
    use std::fs;
    use std::path::Path;
    use std::sync::{Mutex, Once};
    use std::thread;
    use std::time::Duration;

    static INIT_LOGGER: Once = Once::new();
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn cleanup_test_dir(test_dir: &Path) {
        if test_dir.exists() {
            let _ = fs::remove_dir_all(test_dir);
        }
    }

    fn init_test_logger() {
        INIT_LOGGER.call_once(|| {
            init_logger(log::LevelFilter::Off, log::LevelFilter::Trace, "test_logs");
        });
    }

    fn setup_test_dir(test_name: &str) -> (PathBuf, std::sync::MutexGuard<'static, ()>) {
        let _guard = TEST_MUTEX.lock().unwrap();
        init_test_logger();
        let test_base = PathBuf::from(format!("test_logs_{}", test_name));
        cleanup_test_dir(&test_base);
        
        LOGGER.set_base_path(test_base.clone());
        thread::sleep(Duration::from_millis(50));
        (test_base, _guard)
    }

    fn wait_for_file_operations() {
        log::logger().flush();
        thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn test_daily_log_file_generation() {
        let (test_base, _guard) = setup_test_dir("daily");
        
        info!(target: "daily_test", "Daily log message 1");
        warn!(target: "daily_test", "Daily warning message");
        error!(target: "daily_test", "Daily error message");
        
        wait_for_file_operations();
        
        let now = Local::now();
        let daily_log = test_base.join(format!("log_{}_{}_{}.log", now.year(), now.month(), now.day()));
        assert!(daily_log.exists(), "Daily log file should exist: {:?}", daily_log);
        
        let daily_content = fs::read_to_string(&daily_log).expect("Should read daily log");
        assert!(daily_content.contains("Daily log message 1"));
        assert!(daily_content.contains("Daily warning message"));
        assert!(daily_content.contains("Daily error message"));
        
        cleanup_test_dir(&test_base);
    }

    #[test]
    fn test_uuid_specific_order_logs() {
        let (test_base, _guard) = setup_test_dir("uuid");
        
        let uuid1 = "test-order-123";
        let uuid2 = "test-order-456";
        
        info!(target: "vending", uuid = uuid1; "Order {} started", uuid1);
        debug!(target: "vending", uuid = uuid1; "Processing order {}", uuid1);
        error!(target: "vending", uuid = uuid1; "Order {} failed", uuid1);
        
        info!(target: "payment", uuid = uuid2; "Payment for order {} initiated", uuid2);
        warn!(target: "payment", uuid = uuid2; "Payment warning for {}", uuid2);
        
        wait_for_file_operations();
        
        let uuid1_file = test_base.join(format!("order_{}.log", uuid1));
        let uuid2_file = test_base.join(format!("order_{}.log", uuid2));
        
        assert!(uuid1_file.exists(), "UUID1 log file should exist");
        assert!(uuid2_file.exists(), "UUID2 log file should exist");
        
        let uuid1_content = fs::read_to_string(&uuid1_file).expect("Should read UUID1 log");
        assert!(uuid1_content.contains(&format!("<{}>", uuid1)));
        assert!(uuid1_content.contains("Order test-order-123 started"));
        assert!(uuid1_content.contains("Processing order"));
        assert!(uuid1_content.contains("Order test-order-123 failed"));
        
        let uuid2_content = fs::read_to_string(&uuid2_file).expect("Should read UUID2 log");
        assert!(uuid2_content.contains(&format!("<{}>", uuid2)));
        assert!(uuid2_content.contains("Payment for order test-order-456"));
        assert!(uuid2_content.contains("Payment warning"));
        
        assert!(!uuid1_content.contains("Payment"));
        assert!(!uuid2_content.contains("Order test-order-123"));
        
        cleanup_test_dir(&test_base);
    }

    #[test]
    fn test_file_cache_functionality() {
        let (test_base, _guard) = setup_test_dir("cache");
        
        for i in 0..35 {
            let uuid = format!("cache-test-{:03}", i);
            info!(target: "cache_test", uuid = uuid.as_str(); "Cache test message {}", i);
        }
        
        wait_for_file_operations();
        
        for i in 0..35 {
            let uuid = format!("cache-test-{:03}", i);
            let file_path = test_base.join(format!("order_{}.log", uuid));
            assert!(file_path.exists(), "Cache test file should exist for UUID {}", uuid);
            
            let content = fs::read_to_string(&file_path).expect("Should read cache test file");
            assert!(content.contains(&format!("Cache test message {}", i)));
        }
        
        cleanup_test_dir(&test_base);
    }

    #[test]
    fn test_concurrent_logging() {
        let (test_base, _guard) = setup_test_dir("concurrent");
        
        let handles: Vec<_> = (0..5).map(|thread_id| {
            thread::spawn(move || {
                for i in 0..5 {
                    let uuid = format!("concurrent-{}-{}", thread_id, i);
                    info!(target: "concurrent", uuid = uuid.as_str(); "Thread {} message {}", thread_id, i);
                    info!(target: "concurrent", "Non-UUID message from thread {}", thread_id);
                }
            })
        }).collect();
        
        for handle in handles {
            handle.join().expect("Thread should complete");
        }
        
        wait_for_file_operations();
        
        for thread_id in 0..5 {
            for i in 0..5 {
                let uuid = format!("concurrent-{}-{}", thread_id, i);
                let uuid_file = test_base.join(format!("order_{}.log", uuid));
                assert!(uuid_file.exists(), "Concurrent UUID file should exist for {}", uuid);
                
                let uuid_content = fs::read_to_string(&uuid_file).expect("Should read concurrent UUID file");
                assert!(uuid_content.contains(&format!("Thread {} message {}", thread_id, i)));
            }
        }
        
        cleanup_test_dir(&test_base);
    }

    #[test]
    fn test_log_format_validation() {
        let (test_base, _guard) = setup_test_dir("format");
        
        let format_uuid = "format-test-uuid";
        info!(target: "format_test", uuid = format_uuid; "Message with UUID");
        warn!(target: "format_test", "Message without UUID");
        
        wait_for_file_operations();
        
        let now = Local::now();
        let daily_log = test_base.join(format!("log_{}_{}_{}.log", now.year(), now.month(), now.day()));
        let format_file = test_base.join("order_format-test-uuid.log");
        
        assert!(daily_log.exists(), "Daily log should exist");
        assert!(format_file.exists(), "Format test UUID file should exist");
        
        let daily_content = fs::read_to_string(&daily_log).expect("Should read daily log");
        let format_content = fs::read_to_string(&format_file).expect("Should read format test file");
        
        assert!(format_content.contains("INFO|[format_test]<format-test-uuid>:Message with UUID"));
        assert!(!format_content.contains("Message without UUID"));
        
        assert!(daily_content.contains("WARN|[format_test]: Message without UUID"));
        assert!(daily_content.contains("INFO|[format_test]<format-test-uuid>:Message with UUID"));
        
        let lines: Vec<&str> = daily_content.lines().collect();
        for line in lines {
            if !line.is_empty() {
                assert!(line.contains("T"), "Log line should contain timestamp with 'T': {}", line);
                assert!(line.contains("|"), "Log line should contain level separator '|': {}", line);
            }
        }
        
        cleanup_test_dir(&test_base);
    }

    #[test]
    fn test_mixed_targets_and_levels() {
        let (test_base, _guard) = setup_test_dir("mixed");
        
        let mixed_uuid = "mixed-test-uuid";
        info!(target: "vending", uuid = mixed_uuid; "Vending machine info");
        debug!(target: "vending", uuid = mixed_uuid; "Vending machine debug");
        error!(target: "payment", uuid = mixed_uuid; "Payment error");
        warn!(target: "ui", "UI warning without UUID");
        trace!(target: "system", "System trace without UUID");
        
        wait_for_file_operations();
        
        let now = Local::now();
        let daily_log = test_base.join(format!("log_{}_{}_{}.log", now.year(), now.month(), now.day()));
        let mixed_file = test_base.join("order_mixed-test-uuid.log");
        
        assert!(daily_log.exists(), "Daily log should exist");
        assert!(mixed_file.exists(), "Mixed test UUID file should exist");
        
        let daily_content = fs::read_to_string(&daily_log).expect("Should read daily log");
        let mixed_content = fs::read_to_string(&mixed_file).expect("Should read mixed test file");
        
        assert!(mixed_content.contains("Vending machine info"));
        assert!(mixed_content.contains("Vending machine debug"));
        assert!(mixed_content.contains("Payment error"));
        assert!(!mixed_content.contains("UI warning"));
        assert!(!mixed_content.contains("System trace"));
        
        assert!(daily_content.contains("[vending]<mixed-test-uuid>"));
        assert!(daily_content.contains("[payment]<mixed-test-uuid>"));
        assert!(daily_content.contains("[ui]: UI warning"));
        assert!(daily_content.contains("[system]: System trace"));
        
        cleanup_test_dir(&test_base);
    }

    #[test]
    fn test_directory_creation() {
        let (test_base, _guard) = setup_test_dir("directory");
        let nested_uuid = "nested-dir-test";
        info!(target: "nested_test", uuid = nested_uuid; "Testing nested directory creation");
        wait_for_file_operations();

        let nested_file = test_base.join("order_nested-dir-test.log");
        assert!(
            nested_file.exists(),
            "Nested directory test file should exist"
        );
        let content = fs::read_to_string(&nested_file).expect("Should read nested test file");
        assert!(content.contains("Testing nested directory creation"));
        cleanup_test_dir(&test_base);
    }
}
