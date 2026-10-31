//! The spell checker of Windows (#1405): Windows 8 and later check the
//! languages the user installed through the Spell Checking API
//! (`ISpellCheckerFactory`, `ISpellChecker` in `spellcheck.h`), with
//! dictionaries TraceDraw cannot read as Hunspell files. `spell.rs` asks
//! it about a word when no Hunspell dictionary or word list knows it, and
//! for suggestions. See `docs/behavior/writing-tools.md`.
//!
//! The interfaces are COM objects, reached here through their published
//! method tables (only the leading methods used are declared). Each
//! thread that checks gets its own checkers, as COM objects belong to the
//! thread that made them; every failure leaves the word unknown, never a
//! panic.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use windows_sys::core::{GUID, HRESULT, PCWSTR, PWSTR};
use windows_sys::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};

/// `CLSID_SpellCheckerFactory`.
const CLSID_SPELL_CHECKER_FACTORY: GUID = GUID::from_u128(0x7ab36653_1796_484b_bdfa_e74f1db7c1dc);
/// `IID_ISpellCheckerFactory`.
const IID_ISPELL_CHECKER_FACTORY: GUID = GUID::from_u128(0x8e018a9d_2415_4677_bf08_794ea61f94bb);

const S_OK: HRESULT = 0;

type This = *mut c_void;

#[allow(dead_code)]
#[repr(C)]
struct UnknownVtbl {
    query_interface: unsafe extern "system" fn(This, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(This) -> u32,
    release: unsafe extern "system" fn(This) -> u32,
}

/// `ISpellCheckerFactory`.
#[allow(dead_code)]
#[repr(C)]
struct FactoryVtbl {
    base: UnknownVtbl,
    get_supported_languages: unsafe extern "system" fn(This, *mut This) -> HRESULT,
    is_supported: unsafe extern "system" fn(This, PCWSTR, *mut i32) -> HRESULT,
    create_spell_checker: unsafe extern "system" fn(This, PCWSTR, *mut This) -> HRESULT,
}

/// `ISpellChecker` (its first methods).
#[allow(dead_code)]
#[repr(C)]
struct CheckerVtbl {
    base: UnknownVtbl,
    get_language_tag: unsafe extern "system" fn(This, *mut PWSTR) -> HRESULT,
    check: unsafe extern "system" fn(This, PCWSTR, *mut This) -> HRESULT,
    suggest: unsafe extern "system" fn(This, PCWSTR, *mut This) -> HRESULT,
}

/// `IEnumSpellingError`.
#[allow(dead_code)]
#[repr(C)]
struct EnumErrorVtbl {
    base: UnknownVtbl,
    next: unsafe extern "system" fn(This, *mut This) -> HRESULT,
}

/// `IEnumString`.
#[allow(dead_code)]
#[repr(C)]
struct EnumStringVtbl {
    base: UnknownVtbl,
    next: unsafe extern "system" fn(This, u32, *mut PWSTR, *mut u32) -> HRESULT,
}

/// A COM object pointer, released when dropped.
struct Com(This);

impl Com {
    /// The object's method table, as `T`.
    ///
    /// SAFETY: the caller names the table of the interface the pointer
    /// was obtained as.
    unsafe fn vtbl<T>(&self) -> &T {
        unsafe { &**(self.0 as *const *const T) }
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: every `Com` holds a reference it owns, from a call
            // that returned an interface pointer.
            unsafe { (self.vtbl::<UnknownVtbl>().release)(self.0) };
        }
    }
}

/// Text as the API takes it: UTF-16 ending in a zero.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// A string the API allocated, read and freed.
///
/// SAFETY: `p` is null or a zero-ended UTF-16 string from `CoTaskMemAlloc`.
unsafe fn take_string(p: PWSTR) -> Option<String> {
    if p.is_null() {
        return None;
    }
    let mut len = 0;
    // SAFETY: the string ends in a zero.
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `len` characters were read above.
    let s = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) });
    // SAFETY: the API allocated it with the task allocator.
    unsafe { CoTaskMemFree(p as *const c_void) };
    Some(s)
}

/// Every string of an `IEnumString`.
fn strings(e: &Com) -> Vec<String> {
    let mut out = Vec::new();
    loop {
        let mut p: PWSTR = std::ptr::null_mut();
        let mut fetched = 0u32;
        // SAFETY: `e` is an `IEnumString`; one string is asked for.
        let hr = unsafe { (e.vtbl::<EnumStringVtbl>().next)(e.0, 1, &mut p, &mut fetched) };
        if hr != S_OK || fetched == 0 {
            // SAFETY: as returned by `Next` (null when none came).
            let _ = unsafe { take_string(p) };
            break;
        }
        // SAFETY: as returned by `Next`.
        if let Some(s) = unsafe { take_string(p) } {
            out.push(s);
        }
        if out.len() > 500 {
            break;
        }
    }
    out
}

/// The checkers of this thread, one per language, and the words looked up.
struct Checkers {
    languages: Vec<(String, Com)>,
    known: HashMap<String, bool>,
}

thread_local! {
    /// `None` until first used; `Some(None)` where there is no checker.
    static CHECKERS: RefCell<Option<Option<Checkers>>> = const { RefCell::new(None) };
}

/// Make the factory and a checker for each language it supports.
fn open() -> Option<Checkers> {
    // SAFETY: plain calls into COM; an apartment already set up on this
    // thread (the window's) answers with an error that leaves COM usable.
    unsafe {
        let _ = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        let mut factory: This = std::ptr::null_mut();
        let hr = CoCreateInstance(
            &CLSID_SPELL_CHECKER_FACTORY,
            std::ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            &IID_ISPELL_CHECKER_FACTORY,
            &mut factory,
        );
        if hr != S_OK || factory.is_null() {
            log::info!("no spell checker in this Windows ({hr:#x})");
            return None;
        }
        let factory = Com(factory);
        let mut list: This = std::ptr::null_mut();
        let hr = (factory.vtbl::<FactoryVtbl>().get_supported_languages)(factory.0, &mut list);
        if hr != S_OK || list.is_null() {
            return None;
        }
        let tags = strings(&Com(list));
        let mut languages = Vec::new();
        for tag in tags {
            let w = wide(&tag);
            let mut supported = 0i32;
            let hr =
                (factory.vtbl::<FactoryVtbl>().is_supported)(factory.0, w.as_ptr(), &mut supported);
            if hr != S_OK || supported == 0 {
                continue;
            }
            let mut checker: This = std::ptr::null_mut();
            let hr = (factory.vtbl::<FactoryVtbl>().create_spell_checker)(
                factory.0,
                w.as_ptr(),
                &mut checker,
            );
            if hr == S_OK && !checker.is_null() {
                let checker = Com(checker);
                // The checker's own name for its language.
                let tag = language_tag(&checker).unwrap_or(tag);
                languages.push((tag, checker));
            }
        }
        log::info!(
            "the spell checker of Windows checks {:?}",
            languages.iter().map(|l| &l.0).collect::<Vec<_>>()
        );
        Some(Checkers {
            languages,
            known: HashMap::new(),
        })
    }
}

fn with<T>(f: impl FnOnce(&mut Checkers) -> T) -> Option<T> {
    CHECKERS.with(|c| {
        let mut c = c.borrow_mut();
        c.get_or_insert_with(open).as_mut().map(f)
    })
}

/// Whether one checker finds nothing wrong in `word`.
fn checks(checker: &Com, word: &[u16]) -> bool {
    let mut errors: This = std::ptr::null_mut();
    // SAFETY: `checker` is an `ISpellChecker`, `word` ends in a zero.
    let hr =
        unsafe { (checker.vtbl::<CheckerVtbl>().check)(checker.0, word.as_ptr(), &mut errors) };
    if hr != S_OK || errors.is_null() {
        return false;
    }
    let errors = Com(errors);
    let mut first: This = std::ptr::null_mut();
    // SAFETY: `errors` is an `IEnumSpellingError`; S_FALSE means none.
    let hr = unsafe { (errors.vtbl::<EnumErrorVtbl>().next)(errors.0, &mut first) };
    if !first.is_null() {
        drop(Com(first));
    }
    hr != S_OK
}

/// The language a checker checks (`get_LanguageTag`).
fn language_tag(checker: &Com) -> Option<String> {
    let mut p: PWSTR = std::ptr::null_mut();
    // SAFETY: `checker` is an `ISpellChecker`; the tag is allocated for us.
    let hr = unsafe { (checker.vtbl::<CheckerVtbl>().get_language_tag)(checker.0, &mut p) };
    if hr != S_OK {
        return None;
    }
    // SAFETY: as returned by `get_LanguageTag`.
    unsafe { take_string(p) }
}

/// The languages the spell checker of Windows checks here.
pub fn languages() -> Vec<String> {
    with(|c| c.languages.iter().map(|l| l.0.clone()).collect()).unwrap_or_default()
}

/// Whether any language's checker knows `word` (remembered per word).
pub fn is_known(word: &str) -> bool {
    with(|c| {
        if let Some(k) = c.known.get(word) {
            return *k;
        }
        let w = wide(word);
        let k = c.languages.iter().any(|(_, checker)| checks(checker, &w));
        if c.known.len() > 50_000 {
            c.known.clear();
        }
        c.known.insert(word.to_string(), k);
        k
    })
    .unwrap_or(false)
}

/// The checkers' suggestions for `word`.
pub fn suggest(word: &str) -> Vec<String> {
    with(|c| {
        let w = wide(word);
        let mut out: Vec<String> = Vec::new();
        for (_, checker) in &c.languages {
            let mut list: This = std::ptr::null_mut();
            // SAFETY: `checker` is an `ISpellChecker`, `w` ends in a zero.
            let hr = unsafe {
                (checker.vtbl::<CheckerVtbl>().suggest)(checker.0, w.as_ptr(), &mut list)
            };
            if hr < 0 || list.is_null() {
                continue;
            }
            for s in strings(&Com(list)) {
                if !out.contains(&s) {
                    out.push(s);
                }
            }
        }
        out
    })
    .unwrap_or_default()
}

/// Run on Windows by the build workflow (`scripts/windows/spell-check`):
/// the method tables above match the system's, as a wrong one would
/// crash the app rather than fail a check (#1405).
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spell_checker_of_windows_answers() {
        let langs = languages();
        println!("::notice::Windows spell checker languages: {langs:?}");
        assert!(
            langs.iter().any(|l| l.eq_ignore_ascii_case("en-US")),
            "no en-US checker: {langs:?}"
        );
        // The en-US checker itself: its tag, a right and a wrong word.
        let direct = with(|c| {
            let (tag, checker) = c
                .languages
                .iter()
                .find(|(t, _)| t.eq_ignore_ascii_case("en-US"))
                .expect("en-US");
            (
                language_tag(checker),
                tag.clone(),
                checks(checker, &wide("beautiful")),
                checks(checker, &wide("recieve")),
                checks(checker, &wide("qzxvbnmq")),
            )
        })
        .expect("a factory");
        println!("::notice::en-US checker (tag, listed, beautiful, recieve, qzxvbnmq): {direct:?}");
        assert_eq!(
            direct.0.as_deref().map(str::to_ascii_lowercase),
            Some("en-us".into())
        );
        assert!(direct.2, "beautiful is a word");
        assert!(!direct.3 && !direct.4, "misspellings pass");
        // Through the functions spell.rs calls, cached and not.
        assert!(is_known("beautiful") && is_known("beautiful"));
        assert!(!is_known("recieve") && !is_known("recieve"));
        let s = suggest("recieve");
        println!("::notice::suggestions for recieve: {s:?}");
        assert!(s.iter().any(|w| w == "receive"), "{s:?}");
        assert!(suggest("").len() < 500);
        // Another thread makes checkers of its own.
        let other = std::thread::spawn(|| (is_known("beautiful"), is_known("qzxvbnmq")))
            .join()
            .expect("thread");
        assert_eq!(other, (true, false));
    }
}
