use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use serde::Serialize;

// 剪贴板历史数据结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClipboardItem {
    pub id: String,
    pub content: String,
    pub timestamp: u64,
    pub item_type: String, // "text" 或 "files"
    pub size: Option<u64>,
    pub file_paths: Option<Vec<String>>,
    pub file_types: Option<Vec<FileTypeInfo>>, // 文件类型信息
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileTypeInfo {
    pub path: String,
    pub file_type: String, // 检测的文件类型
    pub mime_type: String, // MIME 类型
    pub category: String, // 文件类别，如 "image", "document", "code" 等
}

// 操作类型
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum Operation {
    Insert = 1,
    Delete = 2,
}

impl From<u8> for Operation {
    fn from(value: u8) -> Self {
        match value {
            1 => Operation::Insert,
            2 => Operation::Delete,
            _ => panic!("Invalid operation type: {}", value),
        }
    }
}

// 存储记录格式
#[derive(Debug, Clone)]
struct StorageRecord {
    operation: Operation,
    timestamp: u64,
    item_id: String,
    data: Option<ClipboardItem>, // INSERT时有数据，DELETE时为None
}

// 自定义存储引擎
pub struct StorageEngine {
    file_path: PathBuf,
    file: BufWriter<File>,
    index: HashMap<String, ClipboardItem>, // 内存索引，key为item_id
    deleted_items: HashMap<String, u64>,   // 已删除项目，key为item_id，value为删除时间戳
}

impl StorageEngine {
    pub fn new(storage_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        // 确保存储目录存在
        std::fs::create_dir_all(&storage_dir)?;
        
        let file_path = storage_dir.join("clipboard.log");
        
        // 打开或创建文件
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;
        
        let mut storage = StorageEngine {
            file_path: file_path.clone(),
            file: BufWriter::new(file),
            index: HashMap::new(),
            deleted_items: HashMap::new(),
        };
        
        // 恢复数据
        storage.recover()?;
        
        Ok(storage)
    }
    
    // 从存储文件恢复数据到内存
    fn recover(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.file_path.exists() {
            return Ok(());
        }
        
        let file = File::open(&self.file_path)?;
        let mut reader = BufReader::new(file);
        
        loop {
            // 读取记录
            match self.read_record(&mut reader) {
                Ok(record) => {
                    match record.operation {
                        Operation::Insert => {
                            if let Some(data) = record.data {
                                // 检查是否后续被删除了
                                if !self.deleted_items.contains_key(&record.item_id) {
                                    self.index.insert(record.item_id.clone(), data);
                                }
                            }
                        }
                        Operation::Delete => {
                            // 标记为已删除
                            self.deleted_items.insert(record.item_id.clone(), record.timestamp);
                            // 从索引中移除
                            self.index.remove(&record.item_id);
                        }
                    }
                }
                Err(_) => break, // 文件读取完毕或出错
            }
        }
        
        println!("恢复了 {} 个剪切板项目", self.index.len());
        Ok(())
    }
    
    // 从文件中读取一条记录
    fn read_record(&self, reader: &mut BufReader<File>) -> Result<StorageRecord, Box<dyn std::error::Error>> {
        // 读取操作类型 (1 byte)
        let mut op_buf = [0u8; 1];
        reader.read_exact(&mut op_buf)?;
        let operation = Operation::from(op_buf[0]);
        
        // 读取时间戳 (8 bytes)
        let mut timestamp_buf = [0u8; 8];
        reader.read_exact(&mut timestamp_buf)?;
        let timestamp = u64::from_le_bytes(timestamp_buf);
        
        // 读取item_id长度 (4 bytes)
        let mut id_len_buf = [0u8; 4];
        reader.read_exact(&mut id_len_buf)?;
        let id_len = u32::from_le_bytes(id_len_buf) as usize;
        
        // 读取item_id
        let mut id_buf = vec![0u8; id_len];
        reader.read_exact(&mut id_buf)?;
        let item_id = String::from_utf8(id_buf)?;
        
        // 读取数据长度 (4 bytes)
        let mut data_len_buf = [0u8; 4];
        reader.read_exact(&mut data_len_buf)?;
        let data_len = u32::from_le_bytes(data_len_buf) as usize;
        
        // 读取数据
        let data = if data_len > 0 {
            let mut data_buf = vec![0u8; data_len];
            reader.read_exact(&mut data_buf)?;
            let json_str = String::from_utf8(data_buf)?;
            Some(serde_json::from_str::<ClipboardItem>(&json_str)?)
        } else {
            None
        };
        
        Ok(StorageRecord {
            operation,
            timestamp,
            item_id,
            data,
        })
    }
    
    // 插入新记录
    pub fn insert(&mut self, item: &ClipboardItem) -> Result<(), Box<dyn std::error::Error>> {
        let record = StorageRecord {
            operation: Operation::Insert,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            item_id: item.id.clone(),
            data: Some(item.clone()),
        };
        
        // 写入文件
        self.write_record(&record)?;
        
        // 更新内存索引
        self.index.insert(item.id.clone(), item.clone());
        // 从删除列表中移除（如果存在）
        self.deleted_items.remove(&item.id);
        
        Ok(())
    }
    
    // 标记删除记录
    pub fn delete(&mut self, item_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
            
        let record = StorageRecord {
            operation: Operation::Delete,
            timestamp,
            item_id: item_id.to_string(),
            data: None,
        };
        
        // 写入删除标记
        self.write_record(&record)?;
        
        // 更新内存索引
        self.index.remove(item_id);
        self.deleted_items.insert(item_id.to_string(), timestamp);
        
        Ok(())
    }
    
    // 写入记录到文件
    fn write_record(&mut self, record: &StorageRecord) -> Result<(), Box<dyn std::error::Error>> {
        // 写入操作类型 (1 byte)
        self.file.write_all(&[record.operation as u8])?;
        
        // 写入时间戳 (8 bytes)
        self.file.write_all(&record.timestamp.to_le_bytes())?;
        
        // 写入item_id长度和内容
        let id_bytes = record.item_id.as_bytes();
        self.file.write_all(&(id_bytes.len() as u32).to_le_bytes())?;
        self.file.write_all(id_bytes)?;
        
        // 写入数据长度和内容
        if let Some(ref data) = record.data {
            let json_str = serde_json::to_string(data)?;
            let data_bytes = json_str.as_bytes();
            self.file.write_all(&(data_bytes.len() as u32).to_le_bytes())?;
            self.file.write_all(data_bytes)?;
        } else {
            // 没有数据，写入长度0
            self.file.write_all(&0u32.to_le_bytes())?;
        }
        
        // 立即刷新到磁盘
        self.file.flush()?;
        
        Ok(())
    }
    
    // 获取所有有效的剪切板项目
    pub fn get_all(&self) -> Vec<ClipboardItem> {
        let mut items: Vec<ClipboardItem> = self.index.values().cloned().collect();
        // 按时间戳倒序排列（最新的在前面）
        items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        items
    }
    
    // 清空所有数据（标记所有项目为删除）
    pub fn clear_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let item_ids: Vec<String> = self.index.keys().cloned().collect();
        
        for item_id in item_ids {
            self.delete(&item_id)?;
        }
        
        Ok(())
    }
    
    // 获取存储统计信息
    pub fn stats(&self) -> StorageStats {
        StorageStats {
            total_items: self.index.len(),
            deleted_items: self.deleted_items.len(),
            file_size: std::fs::metadata(&self.file_path)
                .map(|m| m.len())
                .unwrap_or(0),
        }
    }
    
    // 压缩存储文件（可选实现，移除已删除的记录）
    pub fn compact(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 刷新并关闭当前文件
        self.file.flush()?;
        
        // 重要：创建一个临时的虚拟writer来替换当前文件句柄
        // 这样确保原文件句柄被完全释放
        let temp_dummy_path = self.file_path.with_extension("dummy");
        let dummy_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_dummy_path)?;
        drop(std::mem::replace(&mut self.file, BufWriter::new(dummy_file)));
        
        let temp_path = self.file_path.with_extension("tmp");
        
        // 创建临时文件
        let temp_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)?;
        
        let mut temp_writer = BufWriter::new(temp_file);
        
        // 重写所有有效记录
        for item in self.index.values() {
            let record = StorageRecord {
                operation: Operation::Insert,
                timestamp: item.timestamp,
                item_id: item.id.clone(),
                data: Some(item.clone()),
            };
            
            // 直接写入到临时文件的writer
            Self::write_record_to_writer_static(&record, &mut temp_writer)?;
        }
        
        temp_writer.flush()?;
        drop(temp_writer);
        
        // 现在可以安全地替换原文件
        std::fs::rename(&temp_path, &self.file_path)?;
        
        // 清理临时的dummy文件
        let _ = std::fs::remove_file(&temp_dummy_path);
        
        // 重新打开文件
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;
        
        self.file = BufWriter::new(file);
        
        // 清空删除列表
        self.deleted_items.clear();
        
        println!("存储文件压缩完成");
        Ok(())
    }
    
    // 辅助方法：写入记录到指定writer（静态版本）
    fn write_record_to_writer_static(record: &StorageRecord, writer: &mut BufWriter<File>) -> Result<(), Box<dyn std::error::Error>> {
        // 写入操作类型 (1 byte)
        writer.write_all(&[record.operation as u8])?;
        
        // 写入时间戳 (8 bytes)
        writer.write_all(&record.timestamp.to_le_bytes())?;
        
        // 写入item_id长度和内容
        let id_bytes = record.item_id.as_bytes();
        writer.write_all(&(id_bytes.len() as u32).to_le_bytes())?;
        writer.write_all(id_bytes)?;
        
        // 写入数据长度和内容
        if let Some(ref data) = record.data {
            let json_str = serde_json::to_string(data)?;
            let data_bytes = json_str.as_bytes();
            writer.write_all(&(data_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(data_bytes)?;
        } else {
            // 没有数据，写入长度0
            writer.write_all(&0u32.to_le_bytes())?;
        }
        
        Ok(())
    }

    // 辅助方法：写入记录到指定writer
    fn write_record_to_writer(&self, record: &StorageRecord, writer: &mut BufWriter<File>) -> Result<(), Box<dyn std::error::Error>> {
        Self::write_record_to_writer_static(record, writer)
    }
}

// 存储统计信息
#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub total_items: usize,
    pub deleted_items: usize,
    pub file_size: u64,
} 