//! Stable accessor functions for `fff-c` FFI struct fields.
//!
//! # Why this exists
//!
//! `fff-c` exposes `FffFileItem`, `FffGrepMatch`, `FffSearchResult`, and
//! `FffGrepResult` as plain C structs. External consumers in other languages
//! (Emacs Lisp via `emacs-ffi`, Python via `ctypes`, etc.) historically
//! accessed fields by computing byte offsets manually — e.g.
//! `(ffi-pointer+ match-ptr 104)` to reach `line_number`. That approach is
//! silently fragile: adding a field, changing a type size, or reordering
//! members shifts every subsequent offset without any compile-time warning.
//!
//! These accessor functions turn field access into a **stable named API**:
//! the struct layout remains an implementation detail of `fff-c`, and callers
//! are insulated from any future layout changes.
//!
//! # Usage from Emacs Lisp (example)
//!
//! ```elisp
//! (define-ffi-function fff--grep-match-get-line-content
//!   "fff_grep_match_get_line_content" :pointer [:pointer] fff--library)
//!
//! ;; instead of: (fff--string-at match-ptr 32)  ; ← was wrong anyway
//! (ffi-get-c-string (fff--grep-match-get-line-content match-ptr))
//! ```
//!
//! # Array iteration helpers
//!
//! `fff_search_result_get_item` and `fff_grep_result_get_item` return a
//! pointer to the Nth element of the result array with bounds checking,
//! eliminating the need for callers to compute `base + n * sizeof(item)`.

use std::ffi::c_char;

use crate::ffi_types::{FffFileItem, FffGrepMatch, FffGrepResult, FffSearchResult};

// ── FffFileItem ──────────────────────────────────────────────────────────────

/// Returns the relative path of a file item (e.g. `"src/main.rs"`).
///
/// The returned pointer is valid for the lifetime of the owning
/// `FffSearchResult`. Do **not** free it directly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_file_item_get_relative_path(
    item: *const FffFileItem,
) -> *const c_char {
    unsafe { (*item).relative_path }
}

/// Returns the file name component of a file item (e.g. `"main.rs"`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_file_item_get_file_name(
    item: *const FffFileItem,
) -> *const c_char {
    unsafe { (*item).file_name }
}

/// Returns the git status string for a file item (e.g. `"M"`, `"?"`).
/// May be null if git is not available or the file is untracked.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_file_item_get_git_status(
    item: *const FffFileItem,
) -> *const c_char {
    unsafe { (*item).git_status }
}

/// Returns the file size in bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_file_item_get_size(item: *const FffFileItem) -> u64 {
    unsafe { (*item).size }
}

/// Returns `true` if the file was detected as binary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_file_item_get_is_binary(item: *const FffFileItem) -> bool {
    unsafe { (*item).is_binary }
}

// ── FffGrepMatch ─────────────────────────────────────────────────────────────

/// Returns the relative path of the file containing this grep match.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_grep_match_get_relative_path(
    m: *const FffGrepMatch,
) -> *const c_char {
    unsafe { (*m).relative_path }
}

/// Returns the file name component of the file containing this grep match.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_grep_match_get_file_name(
    m: *const FffGrepMatch,
) -> *const c_char {
    unsafe { (*m).file_name }
}

/// Returns the full text content of the matched line.
///
/// # Historical note
///
/// Early consumers of `fff-c` hardcoded offset 32 to reach this field.
/// That offset pointed to `match_ranges` (a pointer to a highlight-range
/// array) after upstream added `file_name` and `git_status` fields between
/// `relative_path` and `line_content`. The correct offset is now 24.
/// Use this function to avoid any dependency on the physical layout.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_grep_match_get_line_content(
    m: *const FffGrepMatch,
) -> *const c_char {
    unsafe { (*m).line_content }
}

/// Returns the 1-based line number of the match within its file.
///
/// # Historical note
///
/// Early consumers hardcoded offset 104 expecting `line_number`.
/// That offset now points to `byte_offset`. The correct offset is 96.
/// Use this function instead.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_grep_match_get_line_number(m: *const FffGrepMatch) -> u64 {
    unsafe { (*m).line_number }
}

/// Returns the 0-based column of the match start within its line.
///
/// # Historical note
///
/// Early consumers hardcoded offset 120 expecting `col`.
/// That offset now points to `context_before_count`. The correct offset is 112.
/// Use this function instead.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_grep_match_get_col(m: *const FffGrepMatch) -> u32 {
    unsafe { (*m).col }
}

/// Returns the byte offset of the match from the start of the file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_grep_match_get_byte_offset(m: *const FffGrepMatch) -> u64 {
    unsafe { (*m).byte_offset }
}

/// Returns `true` if the matched file was detected as binary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_grep_match_get_is_binary(m: *const FffGrepMatch) -> bool {
    unsafe { (*m).is_binary }
}

// ── FffSearchResult ──────────────────────────────────────────────────────────

/// Returns the number of file items in a search result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_search_result_get_count(r: *const FffSearchResult) -> u32 {
    unsafe { (*r).count }
}

// ── FffGrepResult ────────────────────────────────────────────────────────────

/// Returns the number of grep matches in a grep result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fff_grep_result_get_count(r: *const FffGrepResult) -> u32 {
    unsafe { (*r).count }
}
