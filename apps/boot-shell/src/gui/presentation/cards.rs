//! Presentation of typed session data in the reference card layout.

use archrescue_core::State;
use archrescue_storage::Disk;

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
    let top = Layout::body_top(app.section);
    let bottom = area.body_bottom(app.section);

    match app.section {
        DASHBOARD => cards.detect(app),
        BACKUP => cards.backup(app, top, bottom),
        VERIFY_PAGE => cards.verify(app, top, bottom),
        REPORTS => cards.report(app, top, bottom),
        REPAIR_PAGE => cards.repair(app, top, bottom),
        deployment::DEPLOY => cards.deployment(app, top, bottom),
        deployment::RESTORE => cards.handoff(app),
        _ if pages::structured(app.section) => {
            if area.split_details() {
                let split = (width - 12) * 42 / 100;
                cards.panel(rect(x, top, split, bottom - top));
                cards.panel(rect(x + split + 12, top, width - split - 12, bottom - top));
                cards.heading(
                    rect(x + split + 24, top + 8, width - split - 36, 24),
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
        self.panel(rect(x, y, width, 60));
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
        let column = (width - 12) / 2;
        let right = x + column + 12;
        let top = Layout::body_top(DASHBOARD);
        let Some(report) = &app.report else {
            self.notice(
                rect(x, top, width, 70),
                "Consultando discos e volumes",
                "Aguarde o inventário de dispositivos acessíveis.",
                (SELECTED, ACCENT),
            );
            return;
        };

        let inventory = &report.inventory;
        let installations = &report.installations.installations;
        let overview_height = if self.area.height < 640 { 144 } else { 164 };
        let bounds = rect(x, top, column, overview_height);
        if let Some(installation) = installations.first() {
            self.dc.gradient(bounds, SELECTED, PANEL, BORDER, 12);
            self.icon_tile(DASHBOARD, x + 12, top + 12);
            self.heading(rect(x + 48, top + 12, column - 60, 27), "Windows offline");

            let kernel = installation
                .architecture
                .as_deref()
                .unwrap_or("Indisponível");
            let loader = installation
                .loader_architecture
                .as_deref()
                .unwrap_or("Indisponível");
            let version = installation.version.as_deref().unwrap_or("Indisponível");
            let path = installation.path.display().to_string();
            let rows = [
                ("Instalação", path.as_str()),
                ("Kernel", kernel),
                ("Carregador UEFI", loader),
                ("Versão do arquivo", version),
            ];
            for (index, (label, value)) in rows.into_iter().enumerate() {
                let row = top + 48 + index as i32 * 21;
                let label_width = (column * 47 / 100).clamp(95, 155);
                self.line(rect(x + 12, row, label_width, 20), label, MUTED);
                self.line(
                    rect(x + label_width + 16, row, column - label_width - 28, 20),
                    value,
                    TEXT,
                );
            }
        } else {
            self.notice(
                bounds,
                "Windows offline",
                "Nenhum candidato corroborado. Confira os volumes consultados e atualize o diagnóstico.",
                (PANEL, MUTED),
            );
        }

        let metrics = [
            ("Discos", inventory.disks.len()),
            ("Volumes", inventory.volumes.len()),
            ("Windows", installations.len()),
        ];
        let metric_width = (column - 12) / 3;
        for (index, (label, count)) in metrics.into_iter().enumerate() {
            self.metric(
                right + index as i32 * (metric_width + 6),
                top,
                if index == 2 {
                    column - 2 * (metric_width + 6)
                } else {
                    metric_width
                },
                label,
                &count.to_string(),
                TEXT,
            );
        }

        let volume_top = top + 68;
        self.panel(rect(right, volume_top, column, overview_height - 68));
        self.heading(
            rect(right + 12, volume_top + 6, column - 24, 23),
            "Volumes consultados",
        );
        let visible_volumes = if overview_height > 150 { 3 } else { 2 };
        if inventory.volumes.is_empty() {
            self.line(
                rect(right + 12, volume_top + 32, column - 24, 19),
                "Nenhum volume enumerado",
                MUTED,
            );
        }
        for (index, volume) in inventory.volumes.iter().take(visible_volumes).enumerate() {
            let path = volume
                .paths
                .first()
                .map_or_else(|| "Sem caminho".into(), |path| path.display().to_string());
            let filesystem = volume.filesystem.as_deref().unwrap_or("FS indisponível");
            let capacity = volume.space.as_ref().map(|space| bytes(space.total));
            let caption = match capacity {
                Some(capacity) => format!("{path}   ·   {filesystem}   ·   {capacity}"),
                None => format!("{path}   ·   {filesystem}"),
            };
            self.line(
                rect(
                    right + 12,
                    volume_top + 32 + index as i32 * 18,
                    column - 24,
                    18,
                ),
                &caption,
                MUTED,
            );
        }

        let disk_top = top + overview_height + 12;
        if inventory.disks.is_empty() {
            self.notice(
                rect(x, disk_top, width, 93),
                "Nenhum disco enumerado",
                "Confira a conexão da mídia e atualize o diagnóstico.",
                (PANEL, MUTED),
            );
        } else {
            for (index, disk) in inventory.disks.iter().take(2).enumerate() {
                self.disk(disk, x + index as i32 * (column + 12), disk_top, column);
            }
        }

        let summary_top = disk_top + 132;
        if summary_top + 60 < self.area.actions() {
            let scan = pages::selected_scan(app);
            let integrity_color = scan.map_or(MUTED, |scan| match scan.status {
                archrescue_integrity::Status::FindingsDetected => WARNING,
                archrescue_integrity::Status::Blocked => CRITICAL,
                _ => TEXT,
            });
            self.metric(
                x,
                summary_top,
                column,
                "Integridade do Windows",
                pages::integrity_status(app),
                integrity_color,
            );
            let backup = report
                .transactions
                .iter()
                .rev()
                .find(|result| result.status.state == State::Completed);
            let label = backup.map_or_else(
                || "Sem backup na sessão".into(),
                |backup| format!("{} arquivos verificados", backup.progress.verified_files),
            );
            self.metric(
                right,
                summary_top,
                column,
                "Backup",
                &label,
                if backup.is_some() {
                    theme::SUCCESS
                } else {
                    MUTED
                },
            );
        }
    }

    fn icon_tile(&mut self, symbol: usize, x: i32, y: i32) {
        surface(self.dc, rect(x, y, 28, 28), SELECTED, BORDER, 7);
        icon(self.dc, symbol, x + 7, y + 7, ACCENT);
    }

    fn disk(&mut self, disk: &Disk, x: i32, top: i32, width: i32) {
        self.panel(rect(x, top, width, 120));
        self.icon_tile(DISKS, x + 12, top + 10);
        let name = disk
            .number
            .map_or_else(|| "Disco".into(), |number| format!("Disco {number}"));
        let capacity = disk
            .capacity
            .map_or_else(|| "Capacidade indisponível".into(), bytes);
        let style = disk.style.as_deref().unwrap_or("—");
        self.heading(rect(x + 48, top + 7, width - 112, 24), &name);
        self.line(rect(x + 48, top + 30, width - 60, 17), &capacity, MUTED);
        let badge_x = x + width - 60;
        surface(
            self.dc,
            rect(badge_x, top + 12, 48, 23),
            SELECTED,
            BORDER,
            6,
        );
        text(
            self.dc,
            self.theme.body(),
            rect(badge_x + 2, top + 12, 44, 23),
            style,
            ACCENT,
            LINE | DT_CENTER,
        );

        let span = width - 24;
        let bar = rect(x + 12, top + 55, span, 11);
        surface(self.dc, bar, INPUT, BORDER, 4);
        if let Some(capacity) = disk.capacity.filter(|&size| size > 0) {
            for partition in &disk.partitions {
                let to_pixel = |position: u64| {
                    (u128::from(position.min(capacity)) * span as u128 / u128::from(capacity))
                        as i32
                };
                let left = to_pixel(partition.offset);
                let right = to_pixel(partition.offset.saturating_add(partition.length));
                if right > left {
                    let color = if partition.efi {
                        theme::SUCCESS
                    } else {
                        ACCENT_END
                    };
                    fill(
                        self.dc,
                        rect(bar.left + left, bar.top + 2, right - left, 7),
                        color,
                    );
                }
            }
        }
        let descriptor = match disk.partitions.len() {
            0 => "Sem extensões de partição observadas".into(),
            1 => "1 partição observada".into(),
            count => format!("{count} partições observadas"),
        };
        self.line(rect(x + 12, top + 72, width - 24, 19), &descriptor, MUTED);
        let access = if !disk.errors.is_empty() {
            (
                format!("Consulta parcial · {} erros", disk.errors.len()),
                WARNING,
            )
        } else if disk.read_only == Some(true) {
            ("Somente leitura".into(), theme::SUCCESS)
        } else if disk.partitions.iter().any(|partition| partition.efi) {
            ("Partição EFI observada".into(), ACCENT)
        } else {
            return;
        };
        self.line(rect(x + 12, top + 94, width - 24, 18), &access.0, access.1);
    }

    fn backup(&mut self, app: &App, top: i32, bottom: i32) {
        let x = self.area.left();
        let width = self.area.body_width();
        let left = self.area.left_column();
        let right = x + left + 12;
        let right_width = width - left - 12;

        self.panel(rect(x, self.area.destination() - 9, width, 81));
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
            if let Some(progress) = app.progress.as_ref().filter(|_| bottom - top >= 270) {
                let y = bottom - 58;
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
            self.panel(rect(x, top, width, bottom - top));
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
                self.notice(
                    rect(x + 12, top + 13, width - 24, 77),
                    title,
                    detail,
                    (ELEVATED, ACCENT),
                );
                return;
            }
            let title = if checkpoints.is_some() {
                "Operações no destino"
            } else {
                "Arquivos e pastas"
            };
            self.heading(rect(x + 12, top + 6, width - 24, 23), title);
            fill(self.dc, rect(x + 11, top + 35, width - 22, 24), ELEVATED);
            let header = if checkpoints.is_some() {
                ("Pasta da operação", "Estado registrado", "")
            } else {
                ("Nome", "Tipo", "Tamanho / acesso")
            };
            let row_x = x + 19;
            let row_width = width - 38;
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

    fn verify(&mut self, app: &App, top: i32, bottom: i32) {
        let x = self.area.left();
        let width = self.area.body_width();
        let Some(backup) = app
            .report
            .as_ref()
            .and_then(|report| report.transactions.last())
        else {
            self.notice(
                rect(x, 127, width, 81),
                "Nenhum backup nesta sessão",
                "Inicie ou retome um backup para obter um resultado verificável.",
                (PANEL, MUTED),
            );
            return;
        };
        let tone = match backup.status.state {
            State::Completed => (SUCCESS_SURFACE, theme::SUCCESS),
            State::Cancelled => (WARNING_SURFACE, WARNING),
            State::Failed | State::Blocked => (ERROR_SURFACE, CRITICAL),
            _ => (PANEL, MUTED),
        };
        self.notice(
            rect(x, 127, width, 58),
            state_label(backup.status.state),
            &backup.status.operation_id,
            tone,
        );
        let metrics = [
            ("Planejados", backup.progress.planned_bytes),
            ("Gravados", backup.progress.acknowledged_bytes),
            ("Verificados", backup.progress.verified_bytes),
        ];
        let column = (width - 16) / 3;
        for (index, (label, value)) in metrics.into_iter().enumerate() {
            self.metric(
                x + index as i32 * (column + 8),
                191,
                column,
                label,
                &bytes(value),
                if index == 2 { tone.1 } else { TEXT },
            );
        }
        self.panel(rect(x, top, width, bottom - top));
        self.heading(
            rect(x + 13, top + 8, width - 26, 26),
            if app.technician {
                "Evidências por arquivo"
            } else {
                "Último resultado"
            },
        );
    }

    fn report(&mut self, app: &App, top: i32, bottom: i32) {
        let x = self.area.left();
        let width = self.area.body_width();
        let left = self.area.left_column();
        let right = x + left + 12;
        let right_width = width - left - 12;

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
                    129,
                    column,
                    label,
                    &value.to_string(),
                    TEXT,
                );
            }
        } else {
            self.notice(
                rect(x, 129, width, 60),
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
        let left = self.area.left_column();
        let right = x + left + 12;
        let right_width = self.area.body_width() - left - 12;
        self.panel(rect(x, top, left, 200));
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
        let left = self.area.left_column();
        let right = x + left + 12;
        let width = self.area.body_width();
        let right_width = width - left - 12;
        self.panel(rect(x, 126, width, 160));
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
        let left = self.area.left_column();
        let right = x + left + 12;
        let right_width = self.area.body_width() - left - 12;
        let top = 127;
        self.panel(rect(x, top, left, 196));
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
