use std::ffi::{c_char, c_longlong, c_ulonglong, c_void, CStr, CString};

use crate::{compute_distribution, parse, roll, roll_seeded, Expression};

/// Helper: convert a Rust serde_json::Value to a C string pointer.
/// Returns null on serialization failure.
fn json_to_ptr(value: &serde_json::Value) -> *mut c_char {
    match serde_json::to_string(value) {
        Ok(s) => CString::new(s).unwrap_or_default().into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Helper: convert a C string pointer to a Rust &str.
/// Returns None if the pointer is null or the string is not valid UTF-8.
///
/// # Safety
/// `ptr` must be a valid, null-terminated C string pointer.
unsafe fn ptr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().ok()
}

/// Parse a dice expression and return the result as a JSON string.
///
/// Returns a JSON object:
/// - On success: `{"success": true, "expression": <Expression as JSON>}`
/// - On failure: `{"success": false, "errors": [{"message": "...", "position": N, "suggestion": "..."}]}`
///
/// The returned pointer must be freed with `diceng_free`.
///
/// # Safety
/// `input` must be a valid, null-terminated C string pointer.
#[no_mangle]
pub unsafe extern "C" fn diceng_parse(input: *const c_char) -> *mut c_char {
    let Some(input_str) = (unsafe { ptr_to_str(input) }) else {
        let err = serde_json::json!({
            "success": false,
            "errors": [{"message": "null or invalid UTF-8 input", "position": 0, "suggestion": null}]
        });
        return json_to_ptr(&err);
    };

    let result = parse(input_str);

    let output = if result.success() {
        let expr = result.expression().unwrap();
        serde_json::json!({
            "success": true,
            "expression": expr
        })
    } else {
        let errors: Vec<serde_json::Value> = result
            .errors()
            .iter()
            .map(|e| {
                serde_json::json!({
                    "message": e.message,
                    "position": e.position,
                    "suggestion": e.suggestion
                })
            })
            .collect();
        serde_json::json!({
            "success": false,
            "errors": errors
        })
    };

    json_to_ptr(&output)
}

/// Roll a dice expression (given as JSON) and return the result as a JSON string.
///
/// `expr_json` must be a JSON string produced by `diceng_parse` (the `expression` field).
/// `seed` is the deterministic seed. Pass -1 for random rolls.
///
/// Returns a JSON object:
/// - `{"value": N, "dice": [...]}` on success
/// - `{"error": "..."}` on failure
///
/// The returned pointer must be freed with `diceng_free`.
///
/// # Safety
/// `expr_json` must be a valid, null-terminated C string pointer.
#[no_mangle]
pub unsafe extern "C" fn diceng_roll(expr_json: *const c_char, seed: c_longlong) -> *mut c_char {
    let Some(json_str) = (unsafe { ptr_to_str(expr_json) }) else {
        let err = serde_json::json!({"error": "null or invalid UTF-8 input"});
        return json_to_ptr(&err);
    };

    let expr: Expression = match serde_json::from_str(json_str) {
        Ok(e) => e,
        Err(e) => {
            let err = serde_json::json!({"error": format!("invalid expression JSON: {}", e)});
            return json_to_ptr(&err);
        }
    };

    let result = if seed < 0 {
        roll(&expr)
    } else {
        roll_seeded(&expr, seed as u32)
    };

    let dice: Vec<serde_json::Value> = result
        .to_verbose_entries()
        .iter()
        .map(|entry| {
            serde_json::json!({
                "value": entry.value,
                "kept": entry.kept,
                "chain": entry.chain,
                "kind": entry.kind,
                "operator": entry.operator,
            })
        })
        .collect();

    let output = serde_json::json!({
        "value": result.value(),
        "dice": dice,
    });
    json_to_ptr(&output)
}

/// Convenience: parse and roll a dice expression in one call.
///
/// `input` is a dice expression string like "3d6+4" or "4d6kh3".
/// `seed` is the deterministic seed. Pass -1 for random rolls.
///
/// Returns a JSON object:
/// - `{"value": N, "dice": [{"value": N, "kept": true, "chain": [...], "kind": "...", "operator": "..."}, ...]}`
/// - `{"error": "..."}` on failure
///
/// The returned pointer must be freed with `diceng_free`.
///
/// # Safety
/// `input` must be a valid, null-terminated C string pointer.
#[no_mangle]
pub unsafe extern "C" fn diceng_roll_dice(input: *const c_char, seed: c_longlong) -> *mut c_char {
    let Some(input_str) = (unsafe { ptr_to_str(input) }) else {
        let err = serde_json::json!({"error": "null or invalid UTF-8 input"});
        return json_to_ptr(&err);
    };

    let parse_result = parse(input_str);
    if !parse_result.success() {
        let err = serde_json::json!({"error": format!("parse error: {}", parse_result.errors()[0].message)});
        return json_to_ptr(&err);
    }

    let expr = parse_result.expression().unwrap();
    let result = if seed < 0 {
        roll(expr)
    } else {
        roll_seeded(expr, seed as u32)
    };

    let dice: Vec<serde_json::Value> = result
        .to_verbose_entries()
        .iter()
        .map(|entry| {
            serde_json::json!({
                "value": entry.value,
                "kept": entry.kept,
                "chain": entry.chain,
                "kind": entry.kind,
                "operator": entry.operator,
            })
        })
        .collect();

    let output = serde_json::json!({
        "value": result.value(),
        "dice": dice,
    });

    json_to_ptr(&output)
}

/// Compute probability distribution for a dice expression.
///
/// `expr_json` must be a JSON string produced by `diceng_parse` (the `expression` field).
/// `trials` is the number of Monte Carlo trials (used as fallback if exact computation fails).
///
/// Returns a JSON object:
/// - `{"distribution": {value: count, ...}, "total": N, "stats": {...}}` on success
/// - `{"error": "..."}` on failure
///
/// The returned pointer must be freed with `diceng_free`.
///
/// # Safety
/// `expr_json` must be a valid, null-terminated C string pointer.
#[no_mangle]
pub unsafe extern "C" fn diceng_compute_distribution(
    expr_json: *const c_char,
    trials: c_ulonglong,
) -> *mut c_char {
    let Some(json_str) = (unsafe { ptr_to_str(expr_json) }) else {
        let err = serde_json::json!({"error": "null or invalid UTF-8 input"});
        return json_to_ptr(&err);
    };

    let expr: Expression = match serde_json::from_str(json_str) {
        Ok(e) => e,
        Err(e) => {
            let err = serde_json::json!({"error": format!("invalid expression JSON: {}", e)});
            return json_to_ptr(&err);
        }
    };

    let trials = if trials == 0 { 10_000 } else { trials as usize };
    let dist = compute_distribution(&expr, trials);

    let stats = dist.stats();
    let distribution: std::collections::HashMap<String, u64> = dist
        .distribution
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();

    let output = serde_json::json!({
        "distribution": distribution,
        "total": dist.total,
        "stats": {
            "min": stats.min,
            "max": stats.max,
            "mean": stats.mean,
            "stddev": stats.stddev,
            "variance": stats.variance,
        }
    });

    json_to_ptr(&output)
}

/// Free a string previously returned by `diceng_parse`, `diceng_roll`, or `diceng_compute_distribution`.
///
/// # Safety
/// `ptr` must be a pointer previously returned by one of the diceng FFI functions.
/// Calling this with any other pointer (or calling twice on the same pointer) is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn diceng_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr as *mut c_char);
        }
    }
}
