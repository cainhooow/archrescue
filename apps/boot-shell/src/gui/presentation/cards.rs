//! Presentation of typed session data in the reference card layout.

use archrescue_core::State;
use archrescue_storage::Disk;

use super::*;

struct Cards<'a> {
    dc: HDC,
    area: &'a Layout,
    theme: &'a Theme,
}

pub(super) unsafe fn render(dc: HDC, area: &Layout, theme: &Theme, app: &App) {
    let cards = Cards {
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
            if width >= 880 {
                let split = width * 43 / 100;
                cards.panel(rect(x, top, split, bottom - top));
                cards.panel(rect(x + split + 4, top, width - split - 4, bottom - top));
                cards.heading(
                    rect(x + split + 15, top + 8, width - split - 28, 24),
                    "Detalhes",
                );
            } else {
                cards.panel(rect(x, top, width, bottom - top));
            }
        }
        _ => {}
    }
}

impl Cards<'_> {
    unsafe fn panel(&self, bounds: RECT) {
        surface(self.dc, bounds, PANEL, BORDER, 12);
    }

    unsafe fn heading(&self, bounds: RECT, caption: &str) {
        text(self.dc, self.theme.emphasis, bounds, caption, TEXT, LINE);
    }

    unsafe fn line(&self, bounds: RECT, caption: &str, color: COLORREF) {
        text(self.dc, self.theme.body, bounds, caption, color, LINE);
    }

    unsafe fn paragraph(&self, bounds: RECT, caption: &str, color: COLORREF) {
        text(
            self.dc,
            self.theme.body,
            bounds,
            caption,
            color,
            DT_WORDBREAK | DT_NOPREFIX,
        );
    }

    unsafe fn metric(
        &self,
        x: i32,
        y: i32,
        width: i32,
        caption: &str,
        value: &str,
        color: COLORREF,
    ) {
        self.panel(rect(x, y, width, 60));
        self.line(rect(x + 11, y + 7, width - 22, 18), caption, MUTED);
        text(
            self.dc,
            self.theme.emphasis,
            rect(x + 11, y + 27, width - 22, 25),
            value,
            color,
            LINE,
        );
    }

    unsafe fn notice(&self, bounds: RECT, title: &str, detail: &str, tone: (COLORREF, COLORREF)) {
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
            self.theme.emphasis,
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

    unsafe fn detect(&self, app: &App) {
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

        surface(self.dc, rect(x, top, column, 107), SELECTED, BORDER, 12);
        let windows = &report.installations.installations;
        if let Some(item) = windows.first() {
            self.heading(rect(x + 12, top + 8, column - 24, 23), "Windows offline");
            self.line(
                rect(x + 12, top + 34, column - 24, 19),
                &item.path.display().to_string(),
                TEXT,
            );
            let kernel = item.architecture.as_deref().unwrap_or("indisponível");
            let loader = item
                .loader_architecture
                .as_deref()
                .unwrap_or("indisponível");
            self.line(
                rect(x + 12, top + 57, column - 24, 19),
                &format!("Kernel {kernel}  ·  UEFI {loader}"),
                MUTED,
            );
            self.line(
                rect(x + 12, top + 80, column - 24, 18),
                item.version
                    .as_deref()
                    .unwrap_or("Versão de arquivo indisponível"),
                MUTED,
            );
        } else {
            self.heading(rect(x + 12, top + 10, column - 24, 23), "Windows offline");
            self.paragraph(
                rect(x + 12, top + 40, column - 24, 59),
                "Nenhum candidato corroborado. Confira as unidades e atualize o diagnóstico.",
                MUTED,
            );
        }

        let inventories = [
            ("Discos", report.inventory.disks.len()),
            ("Volumes", report.inventory.volumes.len()),
            ("Windows", windows.len()),
        ];
        let small = (column - 12) / 3;
        for (index, (label, count)) in inventories.into_iter().enumerate() {
            self.metric(
                right + index as i32 * (small + 6),
                top,
                small,
                label,
                &count.to_string(),
                TEXT,
            );
        }
        self.panel(rect(right, top + 67, column, 40));
        let volumes = report
            .inventory
            .volumes
            .iter()
            .filter_map(|volume| volume.paths.first())
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("   ");
        let accessible = if volumes.is_empty() {
            "Sem caminho de acesso"
        } else {
            &volumes
        };
        self.line(
            rect(right + 11, top + 74, column - 22, 25),
            accessible,
            MUTED,
        );

        let disk_top = top + 119;
        if report.inventory.disks.is_empty() {
            self.notice(
                rect(x, disk_top, width, 95),
                "Nenhum disco enumerado",
                "Confira a conexão da mídia e atualize o diagnóstico.",
                (PANEL, MUTED),
            );
        } else {
            for (index, disk) in report.inventory.disks.iter().take(2).enumerate() {
                self.disk(disk, x + index as i32 * (column + 12), disk_top, column);
            }
        }

        let summary_top = disk_top + 106;
        let integrity = pages::selected_scan(app);
        let integrity_label = pages::integrity_status(app);
        let integrity_color = integrity.map_or(MUTED, |scan| match scan.status {
            archrescue_integrity::Status::FindingsDetected => WARNING,
            archrescue_integrity::Status::Blocked => CRITICAL,
            _ => TEXT,
        });
        self.metric(
            x,
            summary_top,
            column,
            "Integridade do Windows",
            integrity_label,
            integrity_color,
        );
        let backup = report
            .transactions
            .iter()
            .rev()
            .find(|result| result.status.state == State::Completed);
        let backup_label = if app.progress.is_some() {
            "Operação em andamento".into()
        } else if let Some(backup) = backup {
            format!("{} arquivos verificados", backup.progress.verified_files)
        } else {
            "Sem backup conhecido".into()
        };
        self.metric(
            right,
            summary_top,
            column,
            "Backup",
            &backup_label,
            if backup.is_some() {
                theme::SUCCESS
            } else {
                TEXT
            },
        );
        let next = integrity.map_or_else(
            || {
                if windows.is_empty() {
                    "Confira os volumes disponíveis."
                } else {
                    "Analise os componentes do Windows offline."
                }
            },
            |scan| scan.recommendation(),
        );
        self.line(
            rect(x, summary_top + 65, width, 20),
            &format!("Próxima ação  ·  {next}"),
            MUTED,
        );
    }

    unsafe fn disk(&self, disk: &Disk, x: i32, top: i32, width: i32) {
        self.panel(rect(x, top, width, 95));
        let number = disk
            .number
            .map_or_else(|| "Disco".into(), |number| format!("Disco {number}"));
        let capacity = disk
            .capacity
            .map_or_else(|| "Capacidade indisponível".into(), bytes);
        let style = disk.style.as_deref().unwrap_or("Estilo indisponível");
        self.heading(
            rect(x + 11, top + 5, width - 22, 24),
            &format!("{number}  ·  {style}"),
        );
        self.line(rect(x + 11, top + 30, width - 22, 17), &capacity, MUTED);

        let span = width - 22;
        fill(self.dc, rect(x + 11, top + 55, span, 10), ELEVATED);
        if let Some(capacity) = disk.capacity.filter(|&size| size > 0) {
            for partition in &disk.partitions {
                let to_pixel = |position: u64| {
                    (u128::from(position.min(capacity)) * span as u128 / u128::from(capacity))
                        as i32
                };
                let left = to_pixel(partition.offset);
                let right = to_pixel(partition.offset.saturating_add(partition.length));
                let color = if partition.efi { ACCENT } else { 0x00885772 };
                fill(
                    self.dc,
                    rect(x + 11 + left, top + 55, right - left, 10),
                    color,
                );
            }
        }
        let descriptor = if disk.partitions.is_empty() {
            "Sem extensões de partição observadas".into()
        } else {
            format!(
                "{}{}",
                match disk.partitions.len() {
                    1 => "1 partição observada".into(),
                    count => format!("{count} partições observadas"),
                },
                if disk.read_only == Some(true) {
                    "  ·  Somente leitura"
                } else {
                    ""
                }
            )
        };
        self.line(rect(x + 11, top + 71, width - 22, 17), &descriptor, MUTED);
    }

    unsafe fn backup(&self, app: &App, top: i32, bottom: i32) {
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

    unsafe fn verify(&self, app: &App, top: i32, bottom: i32) {
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

    unsafe fn report(&self, app: &App, top: i32, bottom: i32) {
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

    unsafe fn repair(&self, app: &App, top: i32, bottom: i32) {
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

    unsafe fn deployment(&self, app: &App, top: i32, bottom: i32) {
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

    unsafe fn handoff(&self, app: &App) {
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
