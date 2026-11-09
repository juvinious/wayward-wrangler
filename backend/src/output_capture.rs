//! Bounded, append-only stream index over a retained combined transcript.
//! Index entries publish only after the corresponding bytes are synced.
use crate::execution::read;
use serde_json::{Value,json};
use std::{fs::{File,OpenOptions},io::{Read,Seek,SeekFrom,Write},path::Path};

pub const TRANSCRIPT_LIMIT:usize=32*1024*1024;
pub const INDEX_LIMIT:u64=65_536;
pub const PAGE_LIMIT:usize=64*1024;
const CHUNK_LIMIT:usize=64*1024;
const RECORD_SIZE:u64=32;

fn stream_code(name:&str)->Result<u8,String>{match name{
    "stdout"=>Ok(1),"stderr"=>Ok(2),"runner"=>Ok(3),_=>Err("Unknown output stream".into())}}
fn stream_name(code:u8)->Result<&'static str,String>{match code{
    1=>Ok("stdout"),2=>Ok("stderr"),3=>Ok("runner"),_=>Err("Corrupt output stream index".into())}}
pub fn hex_bytes(bytes:&[u8])->String{let mut out=String::with_capacity(bytes.len()*2);
    for byte in bytes{use std::fmt::Write;let _=write!(out,"{byte:02x}");}out}

pub struct Recorder {transcript:File,index:File,pending:Vec<u8>,stream:u8,
    offset:u64,records:u64,pending_time_ms:u64}
impl Recorder {
    pub fn create(directory:&Path)->Result<Self,String>{
        let transcript=OpenOptions::new().write(true).create_new(true)
            .open(directory.join("transcript.log")).map_err(|e|e.to_string())?;
        let index=OpenOptions::new().write(true).create_new(true)
            .open(directory.join("output.index")).map_err(|e|e.to_string())?;
        Ok(Self{transcript,index,pending:Vec::with_capacity(CHUNK_LIMIT),stream:0,
            offset:0,records:0,pending_time_ms:0})
    }
    pub fn push(&mut self,name:&str,mut bytes:&[u8])->Result<(),String>{
        let code=stream_code(name)?;
        while !bytes.is_empty(){
            if self.stream!=0 && self.stream!=code{self.flush()?;}
            if self.stream==0{self.stream=code;self.pending_time_ms=
                (crate::execution::now()*1000.0) as u64;}
            let amount=(CHUNK_LIMIT-self.pending.len()).min(bytes.len());
            self.pending.extend_from_slice(&bytes[..amount]);bytes=&bytes[amount..];
            if self.pending.len()==CHUNK_LIMIT{self.flush()?;}
        }
        Ok(())
    }
    pub fn flush(&mut self)->Result<(),String>{
        if self.pending.is_empty(){return Ok(());}
        if self.records>=INDEX_LIMIT{return Err("output_index_limit: 65,536 records (2 MiB)".into());}
        let len=self.pending.len() as u32;
        self.transcript.write_all(&self.pending).map_err(|e|e.to_string())?;
        self.transcript.sync_data().map_err(|e|e.to_string())?;
        let mut record=[0u8;32];
        record[0..8].copy_from_slice(&(self.records+1).to_le_bytes());
        record[8..16].copy_from_slice(&self.offset.to_le_bytes());
        record[16..20].copy_from_slice(&len.to_le_bytes());
        record[20]=self.stream;
        record[24..32].copy_from_slice(&self.pending_time_ms.to_le_bytes());
        self.index.write_all(&record).map_err(|e|e.to_string())?;
        self.index.sync_data().map_err(|e|e.to_string())?;
        self.offset+=len as u64;self.records+=1;self.pending.clear();self.stream=0;
        Ok(())
    }
    pub fn committed(&self)->usize{self.offset as usize}
}

fn indexed(directory:&Path,attempt:&str,cursor:Option<&Value>,limit:usize)->Result<Value,String>{
    let mut index=File::open(directory.join("output.index")).map_err(|e|e.to_string())?;
    let bytes=index.metadata().map_err(|e|e.to_string())?.len();
    let high_water=(bytes/RECORD_SIZE).min(INDEX_LIMIT);
    let incomplete_tail=bytes%RECORD_SIZE!=0;
    let (mut sequence,mut within)=if let Some(cursor)=cursor{
        if cursor["format"]!="indexed-v1" || cursor["attempt"]!=attempt{
            return Err("Output cursor belongs to another attempt or format".into());
        }
        (cursor["sequence"].as_u64().ok_or("Invalid output cursor sequence")?,
            cursor["offset"].as_u64().ok_or("Invalid output cursor offset")?)
    }else{(1,0)};
    if sequence==0 || sequence>high_water+1 || (sequence==high_water+1 && within!=0){
        return Err("Output cursor is outside retained index".into());
    }
    let mut remaining=limit.min(PAGE_LIMIT);let mut segments=Vec::new();
    let mut transcript=File::open(directory.join("transcript.log")).map_err(|e|e.to_string())?;
    let transcript_size=transcript.metadata().map_err(|e|e.to_string())?.len();
    while sequence<=high_water && remaining>0{
        let mut record=[0u8;32];index.seek(SeekFrom::Start((sequence-1)*RECORD_SIZE)).map_err(|e|e.to_string())?;
        index.read_exact(&mut record).map_err(|e|e.to_string())?;
        let seq=u64::from_le_bytes(record[0..8].try_into().unwrap());
        let start=u64::from_le_bytes(record[8..16].try_into().unwrap());
        let len=u32::from_le_bytes(record[16..20].try_into().unwrap()) as u64;
        let stream=stream_name(record[20])?;
        let time=u64::from_le_bytes(record[24..32].try_into().unwrap());
        if seq!=sequence || len==0 || len>CHUNK_LIMIT as u64 || within>=len ||
            start.checked_add(len).is_none_or(|end|end>transcript_size){
            return Err("Indexed output is incomplete or corrupt; preserve evidence".into());
        }
        let take=(len-within).min(remaining as u64) as usize;
        let mut content=vec![0;take];
        transcript.seek(SeekFrom::Start(start+within)).map_err(|e|e.to_string())?;
        transcript.read_exact(&mut content).map_err(|e|e.to_string())?;
        segments.push(json!({"sequence":sequence,"stream":stream,"raw_offset":start+within,
            "time_unix_ms":time,"encoding":"hex","data":hex_bytes(&content)}));
        remaining-=take;within+=take as u64;
        if within==len{sequence+=1;within=0;}
    }
    let limit_record=directory.join("output-limit.json");
    Ok(json!({"schema_version":1,"format":"indexed-v1","attempt":attempt,
        "segments":segments,"cursor":{"format":"indexed-v1","attempt":attempt,
            "sequence":sequence,"offset":within},"high_water":high_water,
        "incomplete_index_tail":incomplete_tail,"terminal":directory.join("receipt.json").exists(),
        "output_limit":if limit_record.exists(){Some(read(&limit_record)?)}else{None}}))
}
pub fn page(directory:&Path,attempt:&str,cursor:Option<&Value>,limit:usize)->Result<Value,String>{
    if limit==0 || limit>PAGE_LIMIT{return Err("Output page must be 1–65536 bytes".into());}
    if directory.join("output.index").exists(){return indexed(directory,attempt,cursor,limit);}
    // Historical records predate stream labels. Their bytes remain untouched.
    let path=directory.join("transcript.log");
    if !path.exists(){
        if cursor.is_some() && (cursor.unwrap()["format"]!="indexed-v1" ||
            cursor.unwrap()["attempt"]!=attempt || cursor.unwrap()["sequence"]!=1 ||
            cursor.unwrap()["offset"]!=0){
            return Err("Output cursor is outside unpublished output".into());
        }
        return Ok(json!({"schema_version":1,"format":"indexed-v1","attempt":attempt,
            "segments":[],"cursor":{"format":"indexed-v1","attempt":attempt,
                "sequence":1,"offset":0},"high_water":0,
            "terminal":directory.join("receipt.json").exists(),"output_limit":null}));
    }
    let mut file=File::open(&path).map_err(|e|e.to_string())?;
    let size=file.metadata().map_err(|e|e.to_string())?.len();
    let offset=if let Some(cursor)=cursor{
        if cursor["format"]!="combined-legacy-v1" || cursor["attempt"]!=attempt{
            return Err("Legacy output cursor belongs to another attempt".into());
        }
        cursor["offset"].as_u64().ok_or("Invalid legacy output offset")?
    }else{0};
    if offset>size{return Err("Legacy output cursor exceeds retained bytes".into());}
    let take=(size-offset).min(limit as u64) as usize;let mut content=vec![0;take];
    file.seek(SeekFrom::Start(offset)).map_err(|e|e.to_string())?;
    file.read_exact(&mut content).map_err(|e|e.to_string())?;
    Ok(json!({"schema_version":1,"format":"combined-legacy-v1","attempt":attempt,
        "segments":[{"sequence":0,"stream":"combined legacy output","raw_offset":offset,
            "encoding":"hex","data":hex_bytes(&content)}],
        "cursor":{"format":"combined-legacy-v1","attempt":attempt,"offset":offset+take as u64},
        "high_water":size,"terminal":directory.join("receipt.json").exists(),
        "output_limit":if directory.join("output-limit.json").exists(){Some(read(&directory.join("output-limit.json"))?)}else{None}}))
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn index_pages_keep_streams_and_cursor_without_duplicate_bytes(){
        let root=tempfile::tempdir().unwrap();let mut writer=Recorder::create(root.path()).unwrap();
        writer.push("stdout",b"one ").unwrap();writer.push("stdout",b"two").unwrap();
        writer.push("stderr",b"!\n").unwrap();writer.flush().unwrap();
        let first=page(root.path(),"a",None,5).unwrap();
        assert_eq!(first["segments"][0]["data"],"6f6e652074");
        let second=page(root.path(),"a",Some(&first["cursor"]),64).unwrap();
        assert_eq!(second["segments"][0]["stream"],"stdout");
        assert_eq!(second["segments"][1]["stream"],"stderr");
        assert!(page(root.path(),"b",Some(&first["cursor"]),64).is_err());
        assert_eq!(page(root.path(),"a",Some(&second["cursor"]),64).unwrap()["segments"].as_array().unwrap().len(),0);
    }
    #[test]fn unpublished_output_is_empty_and_cursor_reconnects(){
        let root=tempfile::tempdir().unwrap();
        let first=page(root.path(),"attempt",None,32).unwrap();
        assert_eq!(first["segments"],json!([]));
        let mut writer=Recorder::create(root.path()).unwrap();
        writer.push("stderr",b"late\n").unwrap();writer.flush().unwrap();
        let second=page(root.path(),"attempt",Some(&first["cursor"]),32).unwrap();
        assert_eq!(second["segments"][0]["stream"],"stderr");
        assert_eq!(second["segments"][0]["data"],"6c6174650a");
        let bad=json!({"format":"indexed-v1","attempt":"attempt","sequence":2,"offset":1});
        assert!(page(root.path(),"attempt",Some(&bad),32).is_err());
    }
    #[test]fn index_cap_refuses_before_unbounded_growth(){
        let root=tempfile::tempdir().unwrap();
        let mut writer=Recorder::create(root.path()).unwrap();
        writer.records=INDEX_LIMIT;
        writer.push("stdout",b"next").unwrap();
        assert!(writer.flush().unwrap_err().contains("output_index_limit"));
        assert_eq!(std::fs::metadata(root.path().join("output.index")).unwrap().len(),0);
        assert_eq!(std::fs::metadata(root.path().join("transcript.log")).unwrap().len(),0);
    }
    #[test]fn sleeping_reader_does_not_backpressure_publishing(){
        let root=tempfile::tempdir().unwrap();let path=root.path().to_path_buf();
        let mut writer=Recorder::create(&path).unwrap();
        let viewer=std::thread::spawn({let path=path.clone();move||{
            std::thread::sleep(std::time::Duration::from_millis(700));
            let page=page(&path,"slow",None,1024).unwrap();
            assert!(!page["segments"].as_array().unwrap().is_empty());
            std::thread::sleep(std::time::Duration::from_millis(700));
        }});
        let started=std::time::Instant::now();
        for _ in 0..16{writer.push("stdout",&[b'x';64*1024]).unwrap();}
        writer.flush().unwrap();
        assert!(started.elapsed()<std::time::Duration::from_secs(2),
            "one MiB publication blocked behind a sleeping viewer");
        assert_eq!(writer.committed(),1024*1024);
        viewer.join().unwrap();
    }
}
