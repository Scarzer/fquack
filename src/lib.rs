extern crate duckdb;
extern crate duckdb_loadable_macros;
extern crate libduckdb_sys;

use duckdb::{
    core::{DataChunkHandle, Inserter, LogicalTypeHandle, LogicalTypeId},
    vtab::{BindInfo, InitInfo, TableFunctionInfo, VTab},
    Connection, Result,
};
use duckdb_loadable_macros::duckdb_entrypoint_c_api;
use libduckdb_sys as ffi;
use std::fs::File;
use std::{
    error::Error,
    sync::atomic::{AtomicBool, Ordering},
};

use std::sync::{Arc, Mutex};
use seq_io::fastq::{self, Record};

#[allow(dead_code)]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        if std::env::var("DEBUG").is_ok() {
            eprintln!("[FQuack Debug] {}", format!($($arg)*));
        }
    };
}

#[repr(C)]
struct FastQBindData {
    filename: String,
}

#[repr(C)]
struct FastQInitData {
    reader: Arc<Mutex<fastq::Reader<File>>>,
    done: AtomicBool,
}

struct FastQVTab;

impl VTab for FastQVTab {
    type InitData = FastQInitData;
    type BindData = FastQBindData;

    fn bind(bind: &BindInfo) -> Result<Self::BindData, Box<dyn std::error::Error>> {
        bind.add_result_column("metadata", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("sequence", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("quality", LogicalTypeHandle::from(LogicalTypeId::Varchar));

        let filename = bind.get_parameter(0).to_string();
        Ok(FastQBindData { filename })
    }

    fn init(info: &InitInfo) -> Result<Self::InitData, Box<dyn std::error::Error>> {
        let bind_data = info.get_bind_data::<FastQBindData>();
        let filename = unsafe { (*bind_data).filename.clone() };

        let reader = fastq::Reader::from_path(&filename)?;

        Ok(FastQInitData {
            reader: Arc::new(Mutex::new(reader)),
            done: AtomicBool::new(false),
        })
    }

    fn func(func: &TableFunctionInfo<Self>, output: &mut DataChunkHandle) -> Result<(), Box<dyn std::error::Error>> {
        let init_data = func.get_init_data();
        let _bind_data = func.get_bind_data();

        debug_print!("Starting func call");
        
        if init_data.done.load(Ordering::SeqCst) {
            return Ok(());
        }

        let mut reader_guard = match init_data.reader.lock() {
            Ok(guard) => guard,
            Err(_) => return Err("Failed to lock reader".into()),
        };

        let mut num_records = 0;
        while let Some(record) = reader_guard.next() {
            let record = record?;

            let id = String::from_utf8_lossy(record.id_bytes()).to_string();
            let seq = String::from_utf8_lossy(record.seq()).to_string();
            let qual = String::from_utf8_lossy(record.qual()).to_string();

            output.flat_vector(0).insert(num_records, id.as_str());
            output.flat_vector(1).insert(num_records, seq.as_str());
            output.flat_vector(2).insert(num_records, qual.as_str());

            num_records += 1;
            debug_print!("Inserted record {}", num_records);
        }

        if num_records == 0 {
            init_data.done.store(true, Ordering::SeqCst);
        }

        output.set_len(num_records);
        Ok(())
    }

    fn parameters() -> Option<Vec<LogicalTypeHandle>> {
        Some(vec![LogicalTypeHandle::from(LogicalTypeId::Varchar)])
    }
}

const EXTENSION_NAME: &str = env!("CARGO_PKG_NAME");

#[duckdb_entrypoint_c_api()]
pub unsafe fn extension_entrypoint(con: Connection) -> Result<(), Box<dyn Error>> {
    con.register_table_function::<FastQVTab>(EXTENSION_NAME)
        .expect("Failed to register fastq table function");
    Ok(())
}