//! Display-only Windows path formatting. Callers retain the original evidence.

use std::{borrow::Cow, path::Path};

pub fn path(path: &Path) -> String {
    assisted(&path.to_string_lossy(), |_| None).into_owned()
}

pub fn text(value: &str, technician: bool) -> Cow<'_, str> {
    if technician {
        Cow::Borrowed(value)
    } else {
        assisted(value, |_| None)
    }
}

/// Formats native and escaped namespaces in mixed prose. Only recognized
/// namespace spans are decoded, so ordinary UNC paths and prose stay intact.
pub fn assisted<'a>(
    value: &'a str,
    mut volume_label: impl FnMut(&str) -> Option<String>,
) -> Cow<'a, str> {
    let mut result = String::new();
    let mut copied = 0;
    let mut previous = None;
    for (offset, character) in value.char_indices() {
        let boundary = previous.is_none_or(|previous: char| {
            previous.is_whitespace() || "=\"'([{<,;".contains(previous)
        });
        previous = Some(character);
        if offset < copied || character != '\\' || !boundary {
            continue;
        }

        let tail = &value[offset..];
        let Some((prefix, escaped, kind)) = namespace(tail) else {
            continue;
        };
        let rest = &tail[prefix..];
        result.push_str(&value[copied..offset]);
        let (replacement, consumed) = display_namespace(rest, escaped, kind, &mut volume_label);
        result.push_str(&replacement);
        copied = offset + prefix + consumed;
    }

    if copied == 0 {
        return Cow::Borrowed(value);
    }
    result.push_str(&value[copied..]);
    Cow::Owned(result)
}

#[derive(Clone, Copy)]
enum Namespace {
    Path,
    Device,
}

fn namespace(value: &str) -> Option<(usize, bool, Namespace)> {
    use Namespace::*;

    [
        (r"\\\\?\\", true, Path),
        (r"\\\\.\\", true, Device),
        (r"\\??\\", true, Path),
        (r"\\Device\\", true, Device),
        (r"\\?\", false, Path),
        (r"\\.\", false, Device),
        (r"\??\", false, Path),
        (r"\Device\", false, Device),
    ]
    .into_iter()
    .find_map(|(prefix, escaped, kind)| {
        starts_with(value, prefix).then_some((prefix.len(), escaped, kind))
    })
}

fn starts_with(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn display_namespace(
    rest: &str,
    escaped: bool,
    kind: Namespace,
    volume_label: &mut impl FnMut(&str) -> Option<String>,
) -> (String, usize) {
    let separator = if escaped { r"\\" } else { r"\" };
    let end = if escaped {
        // Debug and JSON values delimit paths with a quote. Avoid consuming
        // newlines escaped by a producer or punctuation after an unquoted path.
        rest.find(['"', '\r', '\n']).unwrap_or(rest.len())
    } else {
        0
    };
    let suffix = |prefix: String, offset: usize| {
        if escaped {
            let end = end.max(offset);
            (prefix + &rest[offset..end].replace(r"\\", r"\"), end)
        } else {
            (prefix, offset)
        }
    };

    if matches!(kind, Namespace::Path) {
        let drive = rest.as_bytes();
        if drive.first().is_some_and(u8::is_ascii_alphabetic)
            && drive.get(1) == Some(&b':')
            && rest
                .get(2..)
                .is_some_and(|tail| tail.starts_with(separator))
        {
            return suffix(String::new(), 0);
        }
        if starts_with(rest, "UNC") && rest[3..].starts_with(separator) {
            let offset = 3 + separator.len();
            let mut components = rest[offset..].split(separator);
            let valid = |part: &str| !part.is_empty() && !part.starts_with(['"', '\r', '\n', ' ']);
            if components.next().is_some_and(valid) && components.next().is_some_and(valid) {
                return suffix(r"\\".into(), offset);
            }
        }
        if starts_with(rest, "Volume{") {
            if let Some(end_guid) = rest.find('}') {
                let guid = &rest[7..end_guid];
                if guid.len() == 36
                    && guid.split('-').zip([8, 4, 4, 4, 12]).all(|(part, size)| {
                        part.len() == size && part.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
                {
                    let offset = end_guid + 1;
                    let label = volume_label(guid).unwrap_or_else(|| "Volume sem letra".into());
                    let after = &rest[offset..];
                    if after == separator
                        || after.strip_prefix(separator).is_some_and(|tail| {
                            tail.is_empty() || tail.starts_with(['"', '\r', '\n', ')', ']'])
                        })
                    {
                        return (label, offset + separator.len());
                    }
                    return suffix(label, offset);
                }
            }
        }
    }

    // Preserve the device and suffix for diagnosis without exposing a native
    // namespace. GLOBALROOT can contain an additional native prefix.
    let mut offset = 0;
    for component in ["GLOBALROOT", "Device", "??", "?"] {
        if starts_with(&rest[offset..], component)
            && rest[offset + component.len()..].starts_with(separator)
        {
            offset += component.len() + separator.len();
        }
    }
    let remainder = &rest[offset..];
    if starts_with(remainder, "PhysicalDrive") {
        let number = remainder[13..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        if number > 0 {
            return suffix(
                format!("Disco {}", &remainder[13..13 + number]),
                offset + 13 + number,
            );
        }
    }

    suffix("Dispositivo: ".into(), offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_native_and_escaped_paths_keep_context_and_unicode() {
        let native = r#"Falha em \\?\E:\Dados de Lúcia\資料 e \\?\uNc\host\pasta\file"#;
        assert_eq!(
            text(native, false),
            r#"Falha em E:\Dados de Lúcia\資料 e \\host\pasta\file"#,
        );

        let debug = r#"Open ["\\\\?\\C:\\Users\\Lúcia", "\\Device\\HarddiskVolume9", "\\\\.\\PhysicalDrive3"]"#;
        assert_eq!(
            text(debug, false),
            r#"Open ["C:\Users\Lúcia", "Dispositivo: HarddiskVolume9", "Disco 3"]"#,
        );
        assert_eq!(text(debug, true), debug);
        assert_eq!(text(&text(debug, false), false), text(debug, false));

        let unc = r"\\host\Device\arquivo e C:\Device\legítimo";
        assert_eq!(text(unc, false), unc);
        assert_eq!(
            text(r"partition=\??\D:\Windows", false),
            r"partition=D:\Windows"
        );
    }

    #[test]
    fn letterless_paths_keep_the_relative_location_and_use_known_labels() {
        let raw = r"\\?\vOLuMe{11111111-2222-3333-4444-555555555555}\EFI\Boot";
        assert_eq!(path(Path::new(raw)), r"Volume sem letra\EFI\Boot");
        assert_eq!(
            assisted(raw, |_| Some("Sistema · Disco 2 · Partição 1".into())),
            r"Sistema · Disco 2 · Partição 1\EFI\Boot",
        );
        let long = format!(r"D:\{}fim", r"pasta-com-acentuação\".repeat(100));
        assert_eq!(path(Path::new(&format!(r"\\?\{long}"))), long);
    }
}
