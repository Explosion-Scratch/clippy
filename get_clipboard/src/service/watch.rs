use crate::clipboard::mac;
use crate::config::{ensure_data_dir, load_config};
use crate::data::store::{store_binary, store_file_path, store_text};
use anyhow::Result;
use objc2::rc::{Retained, autoreleasepool};
use objc2_app_kit::{
    NSPasteboard, NSPasteboardItem, NSPasteboardType, NSPasteboardTypeFileURL,
    NSPasteboardTypeHTML, NSPasteboardTypePNG, NSPasteboardTypeRTF, NSPasteboardTypeString,
    NSPasteboardTypeTIFF,
};
use std::thread;
use std::time::Duration;

fn get_items(pasteboard: &NSPasteboard) -> Vec<Retained<NSPasteboardItem>> {
    pasteboard
        .pasteboardItems()
        .map(|items| items.to_vec())
        .unwrap_or_default()
}

fn read_types(item: &NSPasteboardItem) -> Vec<String> {
    item.types().iter().map(|t| t.to_string()).collect()
}

fn read_string(item: &NSPasteboardItem, ty: &NSPasteboardType) -> Option<String> {
    item.stringForType(ty).map(|s| s.to_string())
}

fn read_data(item: &NSPasteboardItem, ty: &NSPasteboardType) -> Option<Vec<u8>> {
    item.dataForType(ty).map(|d| d.to_vec())
}

pub fn run_watch(max_iterations: Option<u64>) -> Result<()> {
    let config = load_config()?;
    ensure_data_dir(&config)?;
    mac::assert_macos()?;
    let mut last_change: isize = 0;
    let mut iterations = 0;

    eprintln!("Starting clipboard watch...");

    loop {
        let (current_change, should_capture) = autoreleasepool(|_| {
            let pasteboard = NSPasteboard::generalPasteboard();
            let change = pasteboard.changeCount();
            let should_capture = change != last_change;
            (change, should_capture)
        });

        if should_capture {
            eprintln!(
                "Clipboard changed (count: {}), capturing...",
                current_change
            );
            last_change = current_change;
            autoreleasepool(|_| {
                let pasteboard = NSPasteboard::generalPasteboard();
                if let Err(err) = capture_items(&pasteboard) {
                    eprintln!("Failed to capture clipboard: {err:?}");
                } else {
                    eprintln!("Successfully captured clipboard item");
                }
            });
        }

        thread::sleep(Duration::from_millis(400));
        if let Some(max) = max_iterations {
            iterations += 1;
            if iterations >= max {
                break;
            }
        }
    }
    Ok(())
}

fn capture_items(pasteboard: &NSPasteboard) -> Result<()> {
    let items = get_items(pasteboard);
    eprintln!("Found {} pasteboard items", items.len());

    for item in items {
        let formats = read_types(&item);
        eprintln!("Item formats: {:?}", formats);

        if formats
            .iter()
            .any(|ty| ty.contains("public.utf8-plain-text") || ty.contains("public.text"))
        {
            if let Some(text) = read_string(&item, unsafe { NSPasteboardTypeString }) {
                eprintln!("Storing text: {} chars", text.len());
                store_text(&text, &formats)?;
                continue;
            }
        }
        if formats.iter().any(|ty| ty.contains("public.file-url")) {
            if let Some(raw) = read_string(&item, unsafe { NSPasteboardTypeFileURL }) {
                for path in mac::parse_file_urls(&raw) {
                    if let Err(err) = store_file_path(&path, &formats) {
                        eprintln!("Failed to store file path {}: {err:?}", path.display());
                    }
                }
                continue;
            }
        }
        if formats
            .iter()
            .any(|ty| ty.contains("public.tiff") || ty.contains("public.png"))
        {
            if let Some(bytes) = read_data(&item, unsafe { NSPasteboardTypeTIFF }) {
                store_binary(&bytes, "image/tiff", &formats)?;
                continue;
            }
            if let Some(bytes) = read_data(&item, unsafe { NSPasteboardTypePNG }) {
                store_binary(&bytes, "image/png", &formats)?;
                continue;
            }
        }
        if formats.iter().any(|ty| ty.contains("text/html")) {
            if let Some(html) = read_string(&item, unsafe { NSPasteboardTypeHTML }) {
                store_text(&html, &formats)?;
                continue;
            }
        }
        if formats.iter().any(|ty| ty.contains("public.rtf")) {
            if let Some(bytes) = read_data(&item, unsafe { NSPasteboardTypeRTF }) {
                store_binary(&bytes, "text/rtf", &formats)?;
                continue;
            }
        }
    }
    Ok(())
}
