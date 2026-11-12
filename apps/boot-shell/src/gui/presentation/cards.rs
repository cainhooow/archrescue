//! Presentation of typed session data in the reference card layout.

use archrescue_core::State;
use archrescue_storage::{Disk, Partition, Volume};

use super::*;

struct Cards<'a, 'frame> {
    dc: &'a mut Scene<'frame>,
    area: &'a Layout,
    theme: &'a Theme,
}

pub(super) fn render(dc: &mut Scene<'_>, area: &Layout, theme: &Theme, app: &App) {
    let mut cards = Cards {
        dc,
        area,
        theme,
    };
    let x = area.left();
    let width = area.body_width();
    let top = area.body_top(app.section);
    let bottom = area.body_bottom(app.section);

    match app.section {
        DASHBOARD => cards.detect(app),
        BACKUP => cards.backup(app),
        VERIFY_PAGE => cards.verify(app, bottom),
        REPORTS => cards.report(app, top, bottom),
        REPAIR_PAGE => cards.repair(app, top, bottom),
        deployment::DEPLOY => cards.deployment(app, top, bottom),
        deployment::RESTORE => cards.handoff(app),
        _ if pages::structured(app.section) => {
            if area.split_details() {
                let (split, detail_x, detail_width) = area.detail_columns(app.section);
                cards.panel(rect(x, top, split, bottom - top));
                cards.panel(rect(detail_x, top, detail_width, bottom - top));
                cards.heading(
                    rect(detail_x + 12, top + 8, detail_width - 24, 24),
                    "Evidências e detalhes",
                );
            } else {
                cards.panel(rect(x, top, width, bottom - top));
            }
        }
        _ => {}
    }
}

impl Cards<'_, '_> {
    fn panel(&mut self, bounds: RECT) {
        surface(self.dc, bounds, PANEL, BORDER, 12);
    }

    fn heading(&mut self, bounds: RECT, caption: &str) {
        text(self.dc, self.theme.emphasis(), bounds, caption, TEXT, LINE);
    }

    fn line(&mut self, bounds: RECT, caption: &str, color: COLORREF) {
        text(self.dc, self.theme.body(), bounds, caption, color, LINE);
    }

    fn paragraph(&mut self, bounds: RECT, caption: &str, color: COLORREF) {
        text(
            self.dc,
            self.theme.body(),
            bounds,
            caption,
            color,
            DT_WORDBREAK | DT_NOPREFIX,
        );
    }

    fn metric(&mut self, x: i32, y: i32, width: i32, caption: &str, value: &str, color: COLORREF) {
        surface(self.dc, rect(x, y, width, 60), METRIC, BORDER, 9);
        self.line(rect(x + 11, y + 7, width - 22, 18), caption, MUTED);
        text(
            self.dc,
            self.theme.emphasis(),
            rect(x + 11, y + 27, width - 22, 25),
            value,
            color,
            LINE,
        );
    }

    fn notice(&mut self, bounds: RECT, title: &str, detail: &str, tone: (COLORREF, COLORREF)) {
        let (background, foreground) = tone;
        surface(self.dc, bounds, background, BORDER, 10);
        let width = bounds.right - bounds.left;
        icon(
            self.dc,
            VERIFY_PAGE,
            bounds.left + 12,
            bounds.top + 11,
            foreground,
        );
        text(
            self.dc,
            self.theme.emphasis(),
            rect(bounds.left + 35, bounds.top + 6, width - 45, 23),
            title,
            foreground,
            LINE,
        );
        self.paragraph(
            rect(
                bounds.left + 35,
                bounds.top + 32,
                width - 46,
                bounds.bottom - bounds.top - 37,
            ),
            detail,
            TEXT,
        );
    }

    fn detect(&mut self, app: &App) {
        let x = self.area.left();
        let width = self.area.body_width();
        let gap = self.area.gap();
        let column = (width - gap) / 2;
        let right = x + column + gap;
        let top = self.area.body_top(DASHBOARD);
        let px = |value| self.area.px(value);
        let Some(report) = &app.report else {
            self.notice(
                rect(x, top, width, px(88)),
                "Consultando discos e volumes",
                "Aguarde o inventário de dispositivos acessíveis.",
                (SELECTED, ACCENT),
            );
            self.empty(
                rect(x, top + px(102), column, px(210)),
                Icon::Windows,
                "Windows offline",
                "O resultado aparecerá após a consulta.",
            );
            self.empty(
                rect(right, top + px(102), column, px(210)),
                Icon::Disk,
                "Discos físicos",
                "O inventário será exibido nesta região.",
            );
            return;
        };

        let inventory = &report.inventory;
        let installations = &report.installations.installations;
        if inventory.disks.is_empty() && installations.is_empty() {
            self.empty(
                rect(x, top, width, px(207)),
                Icon::Disk,
                "Nenhum disco físico enumerado",
                "Confira a conexão da mídia e atualize o diagnóstico.",
            );
            self.notice(
                rect(x, top + px(221), width, px(67)),
                "Nenhuma instalação encontrada",
                "Nenhum Windows offline foi corroborado nos volumes consultados.",
                (METRIC, MUTED),
            );
            return;
        }

        let partial = !inventory.errors.is_empty()
            || inventory.disks.iter().any(|disk| !disk.errors.is_empty());
        let mut body = top;
        if partial || installations.len() != 1 {
            let (title, detail, tone) = if installations.len() > 1 {
                (
                    "Mais de um Windows offline encontrado",
                    "Consulte a integridade para selecionar a instalação que deseja analisar.",
                    (WARNING_SURFACE, WARNING),
                )
            } else if partial {
                (
                    "Dados de armazenamento incompletos",
                    "Consulte os detalhes dos discos para ver os erros de consulta.",
                    (WARNING_SURFACE, WARNING),
                )
            } else {
                (
                    "Nenhum Windows offline corroborado",
                    "Os discos abaixo foram enumerados durante a consulta.",
                    (METRIC, MUTED),
                )
            };
            self.notice(rect(x, body, width, px(68)), title, detail, tone);
            body += px(68) + gap;
        }

        if let Some(installation) = installations.first() {
            let height = px(179);
            self.candidate(rect(x, body, column, height), installation);
            if let Some(second) = installations.get(1) {
                self.candidate(rect(right, body, column, height), second);
                body += height + gap;
            } else {
                let metrics = [
                    ("Discos", inventory.disks.len()),
                    ("Volumes", inventory.volumes.len()),
                    ("Windows offline", installations.len()),
                ];
                let metric_gap = px(10);
                let metric_width = (column - 2 * metric_gap) / 3;
                for (index, (label, count)) in metrics.into_iter().enumerate() {
                    self.metric(
                        right + index as i32 * (metric_width + metric_gap),
                        body,
                        metric_width,
                        label,
                        &count.to_string(),
                        TEXT,
                    );
                }

                let volume_top = body + 60 + px(13);
                let row_count = inventory.volumes.len().clamp(1, 3) as i32;
                let volume_height = px(55 + 27 * row_count + 10);
                self.panel(rect(right, volume_top, column, volume_height));
                self.heading(
                    rect(right + px(18), volume_top + px(14), column - px(36), px(31)),
                    "Volumes consultados",
                );
                for (index, volume) in inventory.volumes.iter().take(3).enumerate() {
                    self.volume_row(
                        right,
                        volume_top + px(51 + index as i32 * 27),
                        column,
                        volume,
                    );
                }
                if inventory.volumes.is_empty() {
                    self.line(
                        rect(right + px(18), volume_top + px(51), column - px(36), px(27)),
                        "Nenhum volume enumerado",
                        MUTED,
                    );
                }
                let mut candidate_bottom = body + height;
                if report
                    .diagnosis
                    .associations
                    .iter()
                    .any(|item| !item.efi_partitions.is_empty() && item.mounted_efi.is_empty())
                {
                    let y = candidate_bottom + px(13);
                    self.notice(
                        rect(x, y, column, px(68)),
                        "Partição EFI sem caminho de acesso",
                        "Consulte os achados de inicialização em Diagnosticar.",
                        (METRIC, MUTED),
                    );
                    candidate_bottom = y + px(68);
                }
                body = candidate_bottom.max(volume_top + volume_height) + gap;
            }
        }

        let available = self.area.actions() - gap - body;
        if available < px(118) {
            return;
        }
        for (index, disk) in inventory.disks.iter().take(2).enumerate() {
            let volumes = inventory.volumes.iter().filter(|volume| {
                disk.number
                    .is_some_and(|number| volume.physical_disks.contains(&number))
            });
            self.disk(
                disk,
                volumes,
                x + index as i32 * (column + gap),
                body,
                column,
                available,
            );
        }
    }

    fn empty(&mut self, bounds: RECT, symbol: Icon, title: &str, detail: &str) {
        self.panel(bounds);
        let center = (bounds.left + bounds.right) / 2;
        let top = (bounds.top + bounds.bottom) / 2 - 45;
        surface(self.dc, rect(center - 16, top, 32, 32), ELEVATED, BORDER, 7);
        symbol.draw(self.dc, center - 8, top + 8, 16, MUTED);
        for (caption, font, y, color) in [
            (title, self.theme.emphasis(), top + 37, TEXT),
            (detail, self.theme.body(), top + 65, MUTED),
        ] {
            text(
                self.dc,
                font,
                rect(bounds.left + 12, y, bounds.right - bounds.left - 24, 24),
                caption,
                color,
                LINE | DT_CENTER,
            );
        }
    }

    fn candidate(
        &mut self,
        bounds: RECT,
        installation: &archrescue_windows_detection::Installation,
    ) {
        let px = |value| self.area.px(value);
        let x = bounds.left;
        let y = bounds.top;
        let width = bounds.right - x;
        let badge = width >= px(510);
        self.dc
            .gradient(bounds, CANDIDATE_START, PANEL, CANDIDATE_BORDER, px(12));
        self.icon_tile(deployment::DEPLOY, x + px(18), y + px(21));
        self.heading(
            rect(
                x + px(68),
                y + px(15),
                width - px(if badge { 260 } else { 82 }),
                px(32),
            ),
            "Windows offline",
        );
        self.mono_line(
            rect(
                x + px(68),
                y + px(43),
                width - px(if badge { 260 } else { 82 }),
                px(26),
            ),
            &display_path(&installation.path),
            MUTED,
        );
        if badge {
            self.badge(
                rect(x + width - px(190), y + px(29), px(172), px(26)),
                "Candidato corroborado",
            );
        }
        fill(
            self.dc,
            rect(x + px(18), y + px(77), width - px(36), 1),
            BORDER,
        );

        for (index, (label, value)) in [
            ("Kernel", installation.architecture.as_deref()),
            (
                "Carregador UEFI",
                installation.loader_architecture.as_deref(),
            ),
            ("Versão do kernel", installation.version.as_deref()),
        ]
        .into_iter()
        .enumerate()
        {
            self.pair(
                x,
                y + px(84 + index as i32 * 27),
                width,
                label,
                value.unwrap_or("Indisponível"),
                index == 2,
            );
        }
    }

    fn badge(&mut self, bounds: RECT, caption: &str) {
        surface(self.dc, bounds, BADGE, CANDIDATE_BORDER, 5);
        text(
            self.dc,
            self.theme.body(),
            bounds,
            caption,
            NAV_TEXT,
            LINE | DT_CENTER,
        );
    }

    fn mono_line(&mut self, bounds: RECT, caption: &str, color: COLORREF) {
        text(self.dc, self.theme.mono(), bounds, caption, color, LINE);
    }

    fn pair(&mut self, x: i32, y: i32, width: i32, label: &str, value: &str, mono: bool) {
        let inset = self.area.px(18);
        let split = width * 42 / 100;
        let row = self.area.px(27);
        self.line(rect(x + inset, y, split - inset - 4, row), label, MUTED);
        let value_area = rect(x + split, y, width - split - inset, row);
        if mono {
            self.mono_line(value_area, value, TEXT);
        } else {
            self.line(value_area, value, TEXT);
        }
    }

    fn volume_row(&mut self, x: i32, y: i32, width: i32, volume: &Volume) {
        let path = volume
            .paths
            .first()
            .map_or_else(|| "Sem caminho".into(), |path| display_path(path));
        let detail = match (volume.filesystem.as_deref(), &volume.space) {
            (Some(fs), Some(_)) => format!("{fs} · espaço consultado"),
            (Some(fs), None) => format!("{fs} · espaço indisponível"),
            (None, _) => "Sistema de arquivos indisponível".into(),
        };
        self.pair(x, y, width, &path, &detail, false);
    }

    fn icon_tile(&mut self, symbol: usize, x: i32, y: i32) {
        let size = self.area.px(41);
        surface(
            self.dc,
            rect(x, y, size, size),
            SELECTED,
            SELECTED_BORDER,
            9,
        );
        let icon_size = self.area.px(19);
        let inset = (size - icon_size) / 2;
        Icon::page(symbol).draw(self.dc, x + inset, y + inset, icon_size, ACCENT);
    }

    fn disk<'a>(
        &mut self,
        disk: &Disk,
        volumes: impl Iterator<Item = &'a Volume>,
        x: i32,
        top: i32,
        width: i32,
        available: i32,
    ) {
        let px = |value| self.area.px(value);
        let volumes = volumes.take(3).collect::<Vec<_>>();
        let access = if !disk.errors.is_empty() {
            Some((
                format!("Consulta parcial · {} erros", disk.errors.len()),
                WARNING,
            ))
        } else if disk.read_only == Some(true) {
            Some(("Somente leitura".into(), theme::SUCCESS))
        } else {
            None
        };
        let rows = volumes.len() as i32 + i32::from(access.is_some());
        let height = px(202 + rows * 27).min(available);
        self.panel(rect(x, top, width, height));
        let inset = px(18);
        self.icon_tile(DISKS, x + inset, top + px(19));
        let name = disk
            .number
            .map_or_else(|| "Disco".into(), |number| format!("Disco {number}"));
        let capacity = disk
            .capacity
            .map_or_else(|| "Capacidade indisponível".into(), bytes);
        self.heading(
            rect(x + px(68), top + px(13), width - px(126), px(32)),
            &name,
        );
        self.line(
            rect(x + px(68), top + px(42), width - px(126), px(21)),
            &capacity,
            MUTED,
        );
        self.badge(
            rect(x + width - px(63), top + px(27), px(45), px(26)),
            disk.style.as_deref().unwrap_or("—"),
        );

        let span = width - 2 * inset;
        let bar = rect(x + inset, top + px(87), span, px(17));
        surface(self.dc, bar, INPUT, BORDER, 4);
        if let Some(capacity) = disk.capacity.filter(|&size| size > 0) {
            let to_pixel = |position: u64| {
                (u128::from(position.min(capacity)) * span as u128 / u128::from(capacity)) as i32
            };
            for partition in &disk.partitions {
                let left = to_pixel(partition.offset);
                let right = to_pixel(partition.offset.saturating_add(partition.length));
                if right > left {
                    let (_, color) = partition_style(partition);
                    let length = (right - left - i32::from(left > 0) * 2).max(1);
                    surface(
                        self.dc,
                        rect(bar.left + left, bar.top, length, px(17)),
                        color,
                        color,
                        4,
                    );
                }
            }
        }

        if height < px(152) {
            return;
        }
        if disk.partitions.is_empty() {
            self.line(
                rect(x + inset, top + px(118), span, px(26)),
                "Sem partições observadas",
                MUTED,
            );
        } else {
            let columns = if width >= px(540) { 4 } else { 2 };
            let step = span / columns;
            for (index, partition) in disk.partitions.iter().take(columns as usize).enumerate() {
                let (label, color) = partition_style(partition);
                let x = x + inset + index as i32 * step;
                surface(self.dc, rect(x, top + px(130), 7, 7), color, color, 2);
                self.line(
                    rect(x + 11, top + px(120), step - 15, px(27)),
                    &format!("{label} · {}", bytes(partition.length)),
                    MUTED,
                );
            }
        }

        if height >= px(167) {
            fill(self.dc, rect(x + inset, top + px(167), span, 1), BORDER);
        }
        for (index, volume) in volumes.into_iter().enumerate() {
            let offset = px(188 + index as i32 * 27);
            if offset + px(27) <= height {
                self.volume_row(x, top + offset, width, volume);
            }
        }
        if let Some((label, color)) = access {
            let y = top + height - px(28);
            if height >= px(215) {
                self.line(rect(x + inset, y, span, px(23)), &label, color);
            }
        }
    }

    fn backup(&mut self, app: &App) {
        let x = self.area.left();
        let width = self.area.body_width();
        let (left, right, right_width) = self.area.columns(60);

        let top = self.area.backup_top(app);
        let form = self.area.backup_form(app);
        let bottom = form - self.area.gap();
        self.panel(rect(x, form, width, 105));
        self.field(rect(x + 13, form + 28, width - 26, 30));
        if app.preview.is_none() {
            self.field(rect(x, self.area.body_top(DASHBOARD) + 18, width - 156, 30));
        }

        if let Some(preview) = &app.preview {
            surface(
                self.dc,
                rect(x, top, left, (bottom - top).min(192)),
                SELECTED,
                BORDER,
                12,
            );
            self.heading(rect(x + 13, top + 7, left - 26, 26), "Origem e destino");
            self.line(
                rect(x + 13, top + 41, left - 26, 18),
                "Origem da seleção",
                MUTED,
            );
            self.line(
                rect(x + 13, top + 61, left - 26, 23),
                &preview.source.display().to_string(),
                TEXT,
            );
            fill(self.dc, rect(x + 13, top + 90, left - 26, 1), BORDER);
            self.line(
                rect(x + 13, top + 97, left - 26, 18),
                "Destino validado · Discos independentes",
                ACCENT,
            );
            self.line(
                rect(x + 13, top + 118, left - 26, 23),
                &preview.destination.display().to_string(),
                TEXT,
            );
            self.paragraph(rect(x + 13, top + 149, left - 26, (bottom - top).min(192) - 153), "O destino receberá o manifesto, o checkpoint e os arquivos verificados em uma pasta exclusiva.", MUTED);

            let count_width = (right_width - 6) / 3;
            self.metric(
                right,
                top,
                count_width,
                "Arquivos",
                &preview.files.to_string(),
                TEXT,
            );
            self.metric(
                right + count_width + 6,
                top,
                right_width - count_width - 6,
                "Tamanho",
                &bytes(preview.bytes),
                TEXT,
            );
            self.panel(rect(
                right,
                top + 68,
                right_width,
                (bottom - top - 68).min(154),
            ));
            self.heading(rect(right + 11, top + 72, right_width - 22, 25), "Seleção");
            if let Some(progress) = &app.progress {
                let y = self.area.body_top(DASHBOARD);
                self.dc
                    .gradient(rect(x, y, width, 58), SELECTED, PANEL, BORDER, 10);
                self.heading(rect(x + 13, y + 5, left - 26, 24), "Backup em andamento");
                let bytes = format!(
                    "{} gravados / {} planejados",
                    bytes(progress.acknowledged_bytes),
                    bytes(progress.planned_bytes),
                );
                self.line(rect(right, y + 5, right_width - 13, 24), &bytes, TEXT);
                progress_bar(
                    self.dc,
                    x + 13,
                    y + 38,
                    width - 26,
                    progress.acknowledged_bytes,
                    progress.planned_bytes,
                    ACCENT,
                );
            }
        } else {
            let checkpoints = app.checkpoints.as_ref();
            let empty = checkpoints.map_or(app.entries.is_empty(), Vec::is_empty);
            if empty {
                let (title, detail) = if checkpoints.is_some() {
                    (
                        "Nenhum checkpoint neste destino",
                        "Confira a pasta de destino ou selecione outra.",
                    )
                } else if app.listing_loaded {
                    (
                        "Esta pasta está vazia",
                        "Use Subir para voltar à pasta anterior.",
                    )
                } else {
                    (
                        "Abra uma pasta de origem",
                        "Confirme o caminho, use Abrir e selecione os itens.",
                    )
                };
                self.empty(
                    rect(x, top, left, bottom - top),
                    Icon::History,
                    title,
                    detail,
                );
                self.panel(rect(right, top, right_width, 115));
                self.heading(
                    rect(right + 12, top + 9, right_width - 24, 28),
                    "Prepare o destino",
                );
                self.paragraph(
                    rect(right + 12, top + 44, right_width - 24, 60),
                    "Informe uma pasta existente em outro disco e escolha a operação.",
                    MUTED,
                );
                return;
            }
            self.panel(rect(x, top, left, bottom - top));
            self.panel(rect(right, top, right_width, (bottom - top).min(156)));
            self.heading(
                rect(right + 12, top + 9, right_width - 24, 28),
                if checkpoints.is_some() {
                    "Checkpoint selecionado"
                } else {
                    "Itens selecionados"
                },
            );
            let title = if checkpoints.is_some() {
                "Operações no destino"
            } else {
                "Arquivos e pastas"
            };
            self.heading(rect(x + 12, top + 6, left - 24, 23), title);
            fill(self.dc, rect(x + 11, top + 35, left - 22, 24), ELEVATED);
            let header = if checkpoints.is_some() {
                ("Pasta da operação", "Estado registrado", "")
            } else {
                ("Nome", "Tipo", "Tamanho / acesso")
            };
            let row_x = x + 19;
            let row_width = left - 38;
            self.line(
                rect(row_x, top + 35, row_width * 57 / 100, 24),
                header.0,
                MUTED,
            );
            self.line(
                rect(row_x + row_width * 57 / 100, top + 35, row_width / 3, 24),
                header.1,
                MUTED,
            );
            self.line(
                rect(
                    row_x + row_width * 57 / 100 + row_width / 7,
                    top + 35,
                    row_width / 4,
                    24,
                ),
                header.2,
                MUTED,
            );
        }
    }

    fn verify(&mut self, app: &App, bottom: i32) {
        let x = self.area.left();
        let (left, right, right_width) = self.area.columns(60);
        let top = self.area.body_top(DASHBOARD);
        let Some(backup) = app
            .report
            .as_ref()
            .and_then(|report| report.transactions.last())
        else {
            self.empty(
                rect(x, top, left, 175),
                Icon::Check,
                "Nenhum backup nesta sessão",
                "Inicie ou retome um backup para obter um resultado verificável.",
            );
            self.panel(rect(right, top, right_width, 148));
            self.heading(
                rect(right + 13, top + 8, right_width - 26, 28),
                "Nova verificação",
            );
            return;
        };
        let tone = match backup.status.state {
            State::Completed => (SUCCESS_SURFACE, theme::SUCCESS),
            State::Cancelled => (WARNING_SURFACE, WARNING),
            State::Failed | State::Blocked => (ERROR_SURFACE, CRITICAL),
            _ => (PANEL, MUTED),
        };

        self.panel(rect(x, top, left, (bottom - top).min(365)));
        self.notice(
            rect(x + 11, top + 11, left - 22, 62),
            state_label(backup.status.state),
            &backup.status.operation_id,
            tone,
        );
        let metrics = [
            ("Planejados", backup.progress.planned_bytes),
            ("Gravados", backup.progress.acknowledged_bytes),
            ("Verificados", backup.progress.verified_bytes),
        ];
        let column = (left - 38) / 3;
        for (index, (label, value)) in metrics.into_iter().enumerate() {
            self.metric(
                x + 11 + index as i32 * (column + 8),
                top + 80,
                column,
                label,
                &bytes(value),
                if index == 2 { tone.1 } else { TEXT },
            );
        }
        self.panel(rect(right, top, right_width, 148));
        self.heading(
            rect(right + 13, top + 8, right_width - 26, 28),
            "Nova verificação",
        );
        self.paragraph(
            rect(right + 13, top + 40, right_width - 26, 52),
            "A releitura confere o conteúdo gravado com os hashes do manifesto.",
            MUTED,
        );
    }

    fn field(&mut self, bounds: RECT) {
        surface(self.dc, bounds, INPUT, BORDER, 7);
    }

    fn report(&mut self, app: &App, top: i32, bottom: i32) {
        let x = self.area.left();
        let width = self.area.body_width();
        let (left, right, right_width) = self.area.columns(60);

        if let Some(report) = &app.report {
            let findings = report.diagnosis.findings.len()
                + report
                    .integrity
                    .iter()
                    .map(|scan| scan.findings().count())
                    .sum::<usize>();
            let metrics = [
                ("Achados", findings),
                ("Backups", report.transactions.len()),
                ("Planos de reparo", report.repair_plans.len()),
                ("Erros da sessão", report.session_errors.len()),
            ];
            let column = (width - 24) / 4;
            for (index, (label, value)) in metrics.into_iter().enumerate() {
                self.metric(
                    x + index as i32 * (column + 8),
                    self.area.body_top(DASHBOARD),
                    column,
                    label,
                    &value.to_string(),
                    TEXT,
                );
            }
        } else {
            self.notice(
                rect(x, self.area.body_top(DASHBOARD), width, 60),
                "Sessão sem dados para exportar",
                "Atualize o diagnóstico para reunir as evidências.",
                (PANEL, MUTED),
            );
        }

        self.panel(rect(x, top, left, bottom - top));
        self.heading(
            rect(x + 13, top + 7, left - 26, 27),
            if app.technician {
                "Evidências e decisões da sessão"
            } else {
                "Decisões registradas"
            },
        );
        self.panel(rect(right, top, right_width, 147));
        self.field(rect(right + 13, top + 64, right_width - 26, 28));
        self.heading(
            rect(right + 13, top + 7, right_width - 26, 26),
            "Destino da exportação",
        );

        let latest_error = app
            .report
            .as_ref()
            .and_then(|report| report.session_errors.last());
        if let Some(error) =
            latest_error.filter(|_| app.error_status && app.operation_page == REPORTS)
        {
            self.notice(
                rect(right, top + 158, right_width, 115),
                "Falha na exportação",
                &error.context,
                (ERROR_SURFACE, CRITICAL),
            );
        } else if let Some(path) = &app.report_export {
            self.notice(
                rect(right, top + 158, right_width, 105),
                "Relatório verificado",
                &format!("report.json e report.txt\n{}", path.display()),
                (SUCCESS_SURFACE, theme::SUCCESS),
            );
        } else {
            self.notice(
                rect(right, top + 158, right_width, 90),
                "Arquivos exportados",
                "JSON para análise e TXT para leitura. Use uma pasta existente em outro disco.",
                (PANEL, MUTED),
            );
        }
    }

    fn repair(&mut self, app: &App, top: i32, bottom: i32) {
        let x = self.area.left();
        let (left, right, right_width) = self.area.columns(60);
        self.panel(rect(x, top, left, 200));
        for offset in [64, 120] {
            self.field(rect(x + 13, top + offset, left - 26, 28));
        }
        self.heading(rect(x + 13, top + 7, left - 26, 26), "Entradas da prévia");
        self.panel(rect(x, top + 210, left, bottom - top - 210));
        self.heading(
            rect(x + 13, top + 214, left - 26, 22),
            "Alvo, alterações e requisitos",
        );

        self.panel(rect(right, top, right_width, 163));
        self.heading(
            rect(right + 13, top + 7, right_width - 26, 26),
            "Execução do reparo",
        );
        self.paragraph(rect(right + 13, top + 40, right_width - 26, 75), "Somente laboratório. O BCDBoot exige clone registrado, preservação verificada e autorização vinculada ao plano.", WARNING);
        if let Some(report) = &app.report {
            let count = report.installations.installations.len();
            if count != 1 {
                self.notice(
                    rect(right, top + 174, right_width, 98),
                    "Candidato offline necessário",
                    "A prévia assistida exige exatamente uma instalação Windows identificada.",
                    (WARNING_SURFACE, WARNING),
                );
            }
        }
    }

    fn deployment(&mut self, app: &App, top: i32, bottom: i32) {
        let x = self.area.left();
        let width = self.area.body_width();
        let (left, right, right_width) = self.area.columns(60);
        let form_top = self.area.body_top(DASHBOARD);
        self.panel(rect(x, form_top, width, 160));
        if !deployment::media_loaded(app) {
            self.notice(
                rect(x, top, left, 75),
                "Execução da instalação indisponível",
                "Informe um WIM, ESD ou a raiz de uma mídia montada.",
                (METRIC, WARNING),
            );
            self.panel(rect(right, top, right_width, 75));
            return;
        }
        self.panel(rect(x, top, left, bottom - top));
        self.heading(
            rect(x + 13, top + 7, left - 26, 26),
            if deployment::qualification(app).is_some() {
                "Prévia de implantação"
            } else {
                "Mídia e índices"
            },
        );

        let review_height = bottom - top;
        self.panel(rect(right, top, right_width, review_height));
        let review = deployment::qualification(app);
        let (label, detail, color) = review.unwrap_or((
            "Revisão do alvo",
            "Selecione a mídia, o índice e a identidade GPT para preparar o plano.",
            MUTED,
        ));
        self.heading(rect(right + 13, top + 8, right_width - 26, 26), label);
        self.paragraph(
            rect(right + 13, top + 40, right_width - 26, review_height - 91),
            detail,
            color,
        );
        self.line(
            rect(right + 13, bottom - 64, right_width - 26, 22),
            "Execução indisponível",
            WARNING,
        );
    }

    fn handoff(&mut self, app: &App) {
        let x = self.area.left();
        let (left, right, right_width) = self.area.columns(60);
        let top = self.area.body_top(DASHBOARD);
        self.panel(rect(x, top, left, 196));
        for offset in [60, 116] {
            self.field(rect(x + 13, top + offset, left - 26, 29));
        }
        self.heading(
            rect(x + 13, top + 7, left - 26, 26),
            "Referência de restauração",
        );

        self.panel(rect(right, top, right_width, 196));
        self.heading(
            rect(right + 13, top + 7, right_width - 26, 26),
            "No Windows instalado",
        );
        self.line(
            rect(right + 13, top + 38, right_width - 26, 20),
            "archrescue-restore-agent.exe",
            ACCENT,
        );
        fill(
            self.dc,
            rect(right + 13, top + 66, right_width - 26, 1),
            BORDER,
        );
        self.paragraph(rect(right + 13, top + 78, right_width - 26, 105), "perfil · Identifique a conta de destino.\nprevia · Revise o mapeamento.\nrestaurar · Confirme a operação para restaurar.", TEXT);

        let latest_error = app
            .report
            .as_ref()
            .and_then(|report| report.session_errors.last());
        if let Some(error) =
            latest_error.filter(|_| app.error_status && app.operation_page == deployment::RESTORE)
        {
            self.notice(
                rect(x, top + 208, self.area.body_width(), 78),
                "Falha ao preparar a referência",
                &error.context,
                (ERROR_SURFACE, CRITICAL),
            );
        } else if let Some(path) = &app.handoff_export {
            self.notice(
                rect(x, top + 208, self.area.body_width(), 78),
                "Referência exportada",
                &path.display().to_string(),
                (SUCCESS_SURFACE, theme::SUCCESS),
            );
        } else {
            self.notice(rect(x, top + 208, self.area.body_width(), 78), "Restauração no Windows instalado", "A referência registra o manifesto e a seleção de origem. Os arquivos continuam na pasta de backup.", (PANEL, MUTED));
        }
    }
}

fn bytes(value: u64) -> String {
    if value >= 1 << 30 {
        format!("{:.1} GiB", value as f64 / (1_u64 << 30) as f64)
    } else if value >= 1 << 20 {
        format!("{:.1} MiB", value as f64 / (1_u64 << 20) as f64)
    } else {
        format!("{value} bytes")
    }
}

fn display_path(path: &std::path::Path) -> String {
    let path = path.display().to_string();
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{unc}");
    }
    path.strip_prefix(r"\\?\")
        .filter(|rest| rest.as_bytes().get(1) == Some(&b':'))
        .unwrap_or(&path)
        .to_owned()
}

fn partition_style(partition: &Partition) -> (&str, COLORREF) {
    if partition.efi {
        return ("ESP", PARTITION_EFI);
    }
    match partition.kind.as_str() {
        "e3c9e316-0b5c-4db8-817d-f92df00215ae" => ("Reservada", PARTITION_RESERVED),
        "de94bba4-06d1-4d40-a16a-bfd50179d6ac" | "mbr:27" => ("Recuperação", PARTITION_RECOVERY),
        "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7" | "mbr:07" => ("Dados básicos", PARTITION_DATA),
        _ => ("Partição", PARTITION_RESERVED),
    }
}
