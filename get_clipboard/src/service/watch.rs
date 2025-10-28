use crate::clipboard::ClipboardSnapshot;
use crate::config::{ensure_data_dir, load_config};
use crate::data::store::store_snapshot;
use anyhow::Result;
use objc2::rc::autoreleasepool;
use objc2_app_kit::NSPasteboard;
use std::thread;
use std::time::Duration;

pub fn run_watch(max_iterations: Option<u64>) -> Result<()> {
    let config = load_config()?;
    ensure_data_dir(&config)?;
    crate::clipboard::mac::assert_macos()?;
    let mut last_change: isize = 0;
    let mut iterations = 0;

    eprintln!("Starting clipboard watch...");

    loop {
        let (current_change, should_capture) = autoreleasepool(|_| {
            let pasteboard = NSPasteboard::generalPasteboard();
            let change = pasteboard.changeCount();
            (change, change != last_change)
        });

        if should_capture {
            last_change = current_change;
            autoreleasepool(|_| {
                let pasteboard = NSPasteboard::generalPasteboard();
                match ClipboardSnapshot::from_pasteboard(&pasteboard) {
                    Ok(Some(snapshot)) => match store_snapshot(snapshot) {
                        Ok(metadata) => {
                            let summary = metadata
                                .summary
                                .clone()
                                .unwrap_or_else(|| "(no summary)".into());
                            eprintln!(
                                "Stored clipboard item: {} [{} copies]",
                                summary, metadata.copy_count
                            );
                        }
                        Err(err) => {
                            eprintln!("Failed to persist clipboard item: {err:?}");
                        }
                    },
                    Ok(None) => {
                        eprintln!("Clipboard change had no supported content");
                    }
                    Err(err) => {
                        eprintln!("Failed to read clipboard snapshot: {err:?}");
                    }
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
