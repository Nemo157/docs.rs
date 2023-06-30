use crate::error::Result;
use anyhow::{anyhow, bail, ensure};
use once_cell::sync::Lazy;
use std::cell::RefCell;
use wasmtime::{Config, Engine, Instance, Module, Store, Trap};

const TOTAL_CODE_BYTE_LENGTH_LIMIT: usize = 5 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
#[error("timed out while highlighting")]
struct Timeout;

#[derive(Debug, thiserror::Error)]
#[error("unknown error while highlighting")]
struct HighlighterFailed;

#[derive(Debug, thiserror::Error)]
#[error("the code exceeded a highlighting limit")]
struct LimitsExceeded;

pub fn try_with_lang(lang: Option<&str>, code: &str) -> Result<String> {
    ensure!(code.len() < TOTAL_CODE_BYTE_LENGTH_LIMIT, LimitsExceeded);

    static ENGINE: Lazy<Engine> = Lazy::new(|| {
        std::thread::spawn(|| loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            ENGINE.increment_epoch();
        });
        Engine::new(Config::new().epoch_interruption(true)).unwrap()
    });
    static MODULE: Lazy<Module> = Lazy::new(|| {
        static MODULE_BYTES: &[u8] = include_bytes!(env!("CARGO_CDYLIB_FILE_HIGHLIGHTER"));
        Module::new(&*ENGINE, MODULE_BYTES).unwrap()
    });

    thread_local! {
        static INSTANCE: RefCell<Option<(Store::<()>, Instance)>> = RefCell::new(None);
    }

    INSTANCE.with(|cell| {
        let mut guard = cell.borrow_mut();

        let (store, instance) = guard.get_or_insert_with(|| {
            let mut store = Store::new(&*ENGINE, ());
            let instance = Instance::new(&mut store, &*MODULE, &[]).unwrap();
            (store, instance)
        });

        // Allow between at least 1 second up to 2 seconds to highlight, dependent on where we are
        // in the current tick
        store.set_epoch_deadline(2);

        fn inner(
            store: &mut Store<()>,
            instance: &mut Instance,
            lang: Option<&str>,
            code: &str,
        ) -> Result<String> {
            let new_str = instance.get_typed_func::<u32, u32>(&mut *store, "new_str")?;
            let free_str = instance.get_typed_func::<(u32, u32), ()>(&mut *store, "free_str")?;
            let memory = instance
                .get_memory(&mut *store, "memory")
                .ok_or_else(|| anyhow!("missing memory"))?;

            let copy_str_in = |store: &mut Store<()>, s: Option<&str>| -> Result<u64> {
                if let Some(s) = s {
                    let len = u32::try_from(s.len())?;
                    let ptr = new_str.call(&mut *store, len)?;
                    ensure!(ptr != 0);
                    memory.write(&mut *store, ptr.try_into()?, s.as_bytes())?;
                    Ok(u64::try_from(ptr)? << 32 | u64::from(len))
                } else {
                    Ok(0)
                }
            };

            let copy_str_out = |store: &mut Store<()>, ptr_len: u64| -> Result<String> {
                let (ptr, len) = (
                    usize::try_from(ptr_len >> 32)?,
                    usize::try_from(ptr_len & u64::from(u32::MAX))?,
                );
                if ptr == 0 {
                    if len == 1 {
                        bail!(LimitsExceeded)
                    } else {
                        bail!(HighlighterFailed)
                    }
                } else {
                    let mut buffer = vec![0; len];
                    memory.read(&*store, ptr, &mut buffer)?;
                    free_str.call(&mut *store, (ptr.try_into()?, len.try_into()?))?;
                    Ok(String::from_utf8(buffer)?)
                }
            };

            let highlight = instance.get_typed_func::<(u64, u64), u64>(&mut *store, "highlight")?;
            let lang = copy_str_in(&mut *store, lang)?;
            let code = copy_str_in(&mut *store, Some(code))?;
            match highlight.call(&mut *store, (lang, code)) {
                Ok(result) => Ok(copy_str_out(&mut *store, result)?),
                Err(e) => {
                    if e.downcast_ref::<Trap>() == Some(&Trap::Interrupt) {
                        bail!(Timeout)
                    } else {
                        bail!(e)
                    }
                }
            }
        }

        match inner(&mut *store, &mut *instance, lang, code) {
            Ok(result) => Ok(result),
            Err(e) => {
                if e.is::<HighlighterFailed>() {
                    Err(e)
                } else {
                    // The above error is returned when the module completes but returns an error,
                    // for other errors we don't know what the state of the current instantiation
                    // is so we create a new one next time we need to highlight.
                    *guard = None;
                    Err(e)
                }
            }
        }
    })
}

pub fn with_lang(lang: Option<&str>, code: &str) -> String {
    match try_with_lang(lang, code) {
        Ok(highlighted) => highlighted,
        Err(e) => {
            if e.is::<Timeout>() {
                log::debug!("timeout while highlighting code");
            } else {
                log::error!("failed while highlighting code: {e:?}");
            }
            tera::escape_html(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{try_with_lang, with_lang, LimitsExceeded, Timeout, TOTAL_CODE_BYTE_LENGTH_LIMIT};

    const PER_LINE_BYTE_LENGTH_LIMIT: usize = 512;

    #[test]
    fn smoke() {
        assert_eq!(
            try_with_lang(Some("toml"), "[a]").unwrap(),
            r#"<span class="syntax-source syntax-toml"><span class="syntax-punctuation syntax-definition syntax-table syntax-begin syntax-toml">[</span><span class="syntax-meta syntax-tag syntax-table syntax-toml"><span class="syntax-entity syntax-name syntax-table syntax-toml">a</span></span><span class="syntax-punctuation syntax-definition syntax-table syntax-end syntax-toml">]</span></span>"#,
        );
    }

    #[test]
    fn timeout() {
        assert!(try_with_lang(
            Some("javascript"),
            r#"
                export async function go_to_bar(bar_no) {
                  foo = bar_no
                  /*
                    testing123
                  */
                }
            "#
        )
        .unwrap_err()
        .is::<Timeout>());
    }

    #[test]
    fn limits() {
        let is_limited = |s: String| {
            try_with_lang(Some("toml"), &s)
                .unwrap_err()
                .is::<LimitsExceeded>()
        };
        assert!(is_limited("a\n".repeat(TOTAL_CODE_BYTE_LENGTH_LIMIT)));
        assert!(is_limited("aa".repeat(PER_LINE_BYTE_LENGTH_LIMIT)));
    }

    #[test]
    fn limited_escaped() {
        let text = "<p>\n".to_string() + "aa".repeat(PER_LINE_BYTE_LENGTH_LIMIT).as_str();
        let highlighted = with_lang(Some("toml"), &text);
        assert!(highlighted.starts_with("&lt;p&gt;\n"));
    }
}
