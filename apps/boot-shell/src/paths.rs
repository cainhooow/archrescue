//! Display-only formatting for Windows paths.

#[cfg(any(windows, test))]
pub fn friendly(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        if let Some((prefix, share)) = rest.split_once('\\') {
            if prefix.eq_ignore_ascii_case("UNC") {
                let mut parts = share.split('\\');
                if parts.next().is_some_and(|part| !part.is_empty())
                    && parts.next().is_some_and(|part| !part.is_empty())
                {
                    return format!(r"\\{share}");
                }
            }
            if let Some(guid) = prefix
                .strip_prefix("Volume{")
                .and_then(|id| id.strip_suffix('}'))
            {
                let segments = guid.split('-').collect::<Vec<_>>();
                if segments.len() == 5
                    && segments.iter().zip([8, 4, 4, 4, 12]).all(|(part, size)| {
                        part.len() == size && part.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
                {
                    return if share.is_empty() {
                        "Volume sem letra".into()
                    } else {
                        format!(r"Volume sem letra\{share}")
                    };
                }
            }
        }
        if let [drive, b':', b'\\', ..] = rest.as_bytes() {
            if drive.is_ascii_alphabetic() {
                return rest.into();
            }
        }
    }
    if text.starts_with(r"\\?\") || text.starts_with(r"\\.\") || text.starts_with(r"\Device\") {
        return "Caminho de dispositivo".into();
    }

    text.into_owned()
}

#[cfg(windows)]
pub fn display(path: &std::path::Path, technician: bool) -> String {
    if technician {
        path.display().to_string()
    } else {
        friendly(path)
    }
}

#[cfg(test)]
mod tests {
    use super::friendly;
    use std::path::Path;

    #[test]
    fn windows_namespaces_are_presented_without_changing_the_original() {
        let cases = [
            (r"\\?\E:\Usuários\Lúcia", r"E:\Usuários\Lúcia"),
            (r"\\?\d:\montagens\disco", r"d:\montagens\disco"),
            (
                r"\\?\UNC\servidor\compartilhamento\資料",
                r"\\servidor\compartilhamento\資料",
            ),
            (
                r"\\servidor\compartilhamento",
                r"\\servidor\compartilhamento",
            ),
            (
                r"\\?\Volume{11111111-2222-3333-4444-555555555555}\EFI",
                r"Volume sem letra\EFI",
            ),
            (r"\\?\UNC\servidor", "Caminho de dispositivo"),
            (r"\\?\D:relativo", "Caminho de dispositivo"),
            (r"\\?\GLOBALROOT\Device\Harddisk0", "Caminho de dispositivo"),
        ];

        for (original, expected) in cases {
            assert_eq!(friendly(Path::new(original)), expected);
        }
        let long = format!(r"X:\{}\fim", "pasta-com-acentuação\\".repeat(100));
        assert_eq!(friendly(Path::new(&format!(r"\\?\{long}"))), long);
    }
}
