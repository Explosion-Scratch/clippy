use anyhow::{Result, anyhow};
use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString, NSPasteboardTypeTIFF};
use objc2_foundation::{NSData, NSString};
use std::path::PathBuf;

pub fn assert_macos() -> Result<()> {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        Err(anyhow!("get_clipboard supports macOS only"))
    }
}

pub fn parse_file_urls(raw: &str) -> Vec<PathBuf> {
    raw.split('\n')
        .filter_map(|line| line.strip_prefix("file://"))
        .map(|path| {
            percent_encoding::percent_decode_str(path)
                .decode_utf8_lossy()
                .to_string()
        })
        .map(PathBuf::from)
        .collect()
}

pub fn set_clipboard_from_bytes(bytes: &[u8], formats: &[String]) -> Result<()> {
    unsafe {
        autoreleasepool(|_| {
            let pasteboard = NSPasteboard::generalPasteboard();
            pasteboard.clearContents();
            if formats.iter().any(|f| f.contains("text")) {
                if let Ok(text) = std::str::from_utf8(bytes) {
                    let string = NSString::from_str(text);
                    pasteboard.setString_forType(&string, NSPasteboardTypeString);
                }
            } else {
                let data = NSData::with_bytes(bytes);
                pasteboard.setData_forType(Some(&data), NSPasteboardTypeTIFF);
            }
        });
    }
    Ok(())
}

pub fn get_current_text() -> Result<Option<String>> {
    unsafe {
        Ok(autoreleasepool(|_| {
            NSPasteboard::generalPasteboard()
                .stringForType(NSPasteboardTypeString)
                .map(|ns| ns.to_string())
        }))
    }
}
