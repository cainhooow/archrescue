# ArchRescue source style

Use four spaces per level and the stable Rust 2021 formatter settings in `rustfmt.toml`. Treat `cargo fmt --all --check` as the formatting baseline. Read the formatted code as a maintainer would and check that the function's stages and safety boundaries are visible.

## Owner references

- Group standard library, dependency, and crate imports. Separate declarations and methods with blank lines. Keep builder and call chains readable, with long argument lists on separate lines. [Company use cases](https://github.com/cainhooow/kav-poss/blob/f93a25fa2ceedaea875ea96caafac6d43c012d7a/src/application/usecases/company_usecases.rs), [Company handlers](https://github.com/cainhooow/kav-poss/blob/f93a25fa2ceedaea875ea96caafac6d43c012d7a/src/infrastructure/interfaces/http/handlers/company_handler.rs)
- Separate lookup, validation, mutation, and result construction. Put each meaningful initializer field on its own line. Use guard clauses for early exits and separate test setup, action, and assertions. [Login use case](https://github.com/cainhooow/archti-backend/blob/d7ef3c3a896de3bbeb3693b2c92972cf4ef52deb/src/application/usecases/user/login_user_usecase.rs), [Access control service](https://github.com/cainhooow/archti-backend/blob/d7ef3c3a896de3bbeb3693b2c92972cf4ef52deb/src/application/services/access_control_service.rs)
- Keep related lookups together. Give validation guards, transaction work, and notifications distinct space. Apply this grouping in Rust using Rust indentation. [Commitment service](https://github.com/cainhooow/schedules/blob/0ab9eddc55eb46ed6729438500d2bbabb3361e18/app/Services/CommitmentServices.php)

## Layout and control flow

Put a blank line between import groups and between unrelated declarations, including constants, types, impl blocks, and functions. Keep attributes attached to their items. Within a function, group related assignments. Separate preparation, validation, processing, verification, and the result where those stages occur.

Keep trivial expressions compact. For a meaningful struct literal, place one field per line. Give long calls and expressions room to read and use trailing commas in multiline lists where Rust permits them. Name intermediate values when they explain a condition or result. Extract cohesive operations when that makes a complex function easier to follow. Preserve check order, error propagation, resource lifetime, and short-circuiting when reorganizing safety-sensitive code.

```rust
fn inspect_destination(source: &VolumeIdentity, destination: &Path) -> Result<Space> {
    let destination = safe_existing(destination)?;
    let destination_identity = identity(&destination)?;

    require_separate(source, &destination_identity)?;

    space(&destination)
}
```

```rust
let file_result = FileResult {
    relative: planned_file.relative.clone(),
    bytes: 0,
    sha256: None,
    verified: false,
};
```

## Names and comments

Name paths by their role, such as `source_path` and `destination_root`. Prefer `backup_plan`, `bytes_read`, `device_handle`, and `verification_error` where those names convey meaning. Use short names for familiar local conventions such as coordinates or a closure comparing two values.

Write comments in English. Explain intent, constraints, and invariants that are hard to infer from code. Keep existing safety explanations with their Win32, filesystem, and other unsafe operations. Explain pointer validity, buffer bounds, handle ownership, or reentrancy at the point that depends on it.

```rust
// SAFETY: encoded_path is NUL-terminated and stats points to writable storage.
if unsafe { libc::statvfs(encoded_path.as_ptr(), stats.as_mut_ptr()) } != 0 {
    return Err(Error::io("statvfs", std::io::Error::last_os_error()));
}
```

## Review

Check that each function can be scanned for its inputs, guards, effects, and result. In tests, leave visible boundaries between fixture setup, the operation under test, and assertions. For backup and storage edits, check that identity validation, write boundaries, partial results, and completion after verification are still explicit.
