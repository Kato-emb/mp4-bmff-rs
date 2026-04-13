//! OPFS sink implementation backing the WASM bindings.
//!
//! Wraps a JS-side `FileSystemSyncAccessHandle` with an internal write
//! buffer that batches small writes and bypasses the buffer for large ones.

#![allow(missing_docs)] // Wrapping a JS API; doc comments live on the JS side.

use std::fmt;

use wasm_bindgen::prelude::*;
use web_sys::{FileSystemReadWriteOptions, FileSystemSyncAccessHandle};

const BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug)]
pub struct WasmFileSystemSyncAccessHandle {
    handle: FileSystemSyncAccessHandle,
    /// 再利用する write オプション（毎回 new するのを避けて FFI コストを削る）
    opts: FileSystemReadWriteOptions,
    pos: u64,
    len: u64,
    buffer: Vec<u8>,
    buffer_offset: u64,
}

impl Drop for WasmFileSystemSyncAccessHandle {
    fn drop(&mut self) {
        // close() 自体が永続化を伴うので、明示 flush は呼ばない（drain だけ）
        let _ = self.drain();
        self.handle.close();
    }
}

impl WasmFileSystemSyncAccessHandle {
    pub fn new(handle: FileSystemSyncAccessHandle) -> Self {
        let len = handle.get_size().unwrap_or(0.0) as u64;

        Self {
            handle,
            opts: FileSystemReadWriteOptions::new(),
            pos: 0,
            len,
            buffer: Vec::with_capacity(BUFFER_SIZE),
            buffer_offset: 0,
        }
    }

    pub const fn position(&self) -> u64 {
        self.pos
    }

    pub const fn len(&self) -> u64 {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<(), WasmFileSystemSyncAccessHandleError> {
        if buf.is_empty() {
            return Ok(());
        }

        // 直前の write_at 等で論理位置がバッファと不連続なら先に吐き出す
        if !self.buffer.is_empty() && self.pos != self.buffer_offset + self.buffer.len() as u64 {
            self.drain()?;
        }

        // バッファ容量以上の書き込みはバッファをバイパスして直接 OPFS に書く
        if buf.len() >= BUFFER_SIZE {
            if !self.buffer.is_empty() {
                self.drain()?;
            }
            self.write_raw(self.pos, buf)?;
            self.advance(buf.len() as u64);
            return Ok(());
        }

        // 小さい書き込みはバッファに溜める（最大 1 回の drain で完了）
        if self.buffer.is_empty() {
            self.buffer_offset = self.pos;
        }

        let available = BUFFER_SIZE - self.buffer.len();
        if buf.len() <= available {
            self.buffer.extend_from_slice(buf);
        } else {
            let (head, tail) = buf.split_at(available);
            self.buffer.extend_from_slice(head);
            self.advance(head.len() as u64);
            self.drain()?;
            self.buffer_offset = self.pos;
            self.buffer.extend_from_slice(tail);
            self.advance(tail.len() as u64);
            return Ok(());
        }

        self.advance(buf.len() as u64);
        Ok(())
    }

    /// 任意オフセットへの書き込み。バッファをバイパスし、論理 `pos` は変更しない。
    /// finalize 時の mdat ヘッダ書き換え等に使う。
    pub fn write_at(
        &mut self,
        offset: u64,
        buf: &[u8],
    ) -> Result<(), WasmFileSystemSyncAccessHandleError> {
        if buf.is_empty() {
            return Ok(());
        }
        // 直前の追記をまず確定させる（同領域を二重書きする可能性を排除）
        self.drain()?;
        self.write_raw(offset, buf)?;
        let end = offset.saturating_add(buf.len() as u64);
        if end > self.len {
            self.len = end;
        }
        Ok(())
    }

    /// 内部バッファを OPFS に吐き出すだけ（OS sync は呼ばない）。高速。
    pub fn drain(&mut self) -> Result<(), WasmFileSystemSyncAccessHandleError> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        self.opts.set_at(self.buffer_offset as f64);
        self.handle
            .write_with_u8_array_and_options(&self.buffer, &self.opts)?;
        self.buffer.clear();
        Ok(())
    }

    /// 内部バッファを吐き出した上で OS レベルの永続化を要求する。コスト高。
    /// finalize 完了時など「ここまでを確実にディスクに」と保証したい時だけ呼ぶ。
    pub fn sync(&mut self) -> Result<(), WasmFileSystemSyncAccessHandleError> {
        self.drain()?;
        self.handle.flush()?;
        Ok(())
    }

    #[inline]
    fn write_raw(
        &self,
        offset: u64,
        buf: &[u8],
    ) -> Result<(), WasmFileSystemSyncAccessHandleError> {
        self.opts.set_at(offset as f64);
        self.handle
            .write_with_u8_array_and_options(buf, &self.opts)?;
        Ok(())
    }

    #[inline]
    fn advance(&mut self, n: u64) {
        self.pos += n;
        if self.pos > self.len {
            self.len = self.pos;
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WasmFileSystemSyncAccessHandleError {
    DomException(web_sys::DomException),
    Other(JsValue),
}

impl WasmFileSystemSyncAccessHandleError {
    pub fn from_js_value(value: JsValue) -> Self {
        if let Some(dom_exception) = value.dyn_ref::<web_sys::DomException>() {
            WasmFileSystemSyncAccessHandleError::DomException(dom_exception.clone())
        } else {
            WasmFileSystemSyncAccessHandleError::Other(value)
        }
    }

    pub fn into_js_value(self) -> JsValue {
        match self {
            WasmFileSystemSyncAccessHandleError::DomException(ex) => JsValue::from(ex),
            WasmFileSystemSyncAccessHandleError::Other(value) => value,
        }
    }
}

impl fmt::Display for WasmFileSystemSyncAccessHandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WasmFileSystemSyncAccessHandleError::DomException(ex) => {
                write!(f, "DOM Exception {}: {}", ex.name(), ex.message())
            }
            WasmFileSystemSyncAccessHandleError::Other(value) => {
                write!(f, "Other error: {:?}", value)
            }
        }
    }
}

impl std::error::Error for WasmFileSystemSyncAccessHandleError {}

impl From<JsValue> for WasmFileSystemSyncAccessHandleError {
    fn from(value: JsValue) -> Self {
        WasmFileSystemSyncAccessHandleError::from_js_value(value)
    }
}

impl From<WasmFileSystemSyncAccessHandleError> for JsValue {
    fn from(error: WasmFileSystemSyncAccessHandleError) -> Self {
        error.into_js_value()
    }
}
