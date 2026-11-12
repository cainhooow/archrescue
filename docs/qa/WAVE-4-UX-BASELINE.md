# Wave 4 UX baseline

Local starting revision `7355215a52e648987f74e5cdccd7b106dab3ce10` on main. The workspace was clean. The last four commits already contained the native design translation and compact layout fixes. Both official HTML pages rendered in Chrome. All 84 PNGs were reviewed by surface. The picker follows the explorer's scrollable rows and the confirmation dialog's dimmed shell, rounded panel, and purple primary action.

| Journey | Existing entry point and required decisions | Path fields |
|---|---|---:|
| Save documents | Proteger dados → source root → Abrir → navigate/select → destination → Planejar → Iniciar backup → Sim | 2 |
| Select backup destination | Enter the independent destination directory in the backup form | 1 |
| Locate EFI | Inspect diagnosis/disk details and transfer the ESP path to Reparar if discovery did not fill it | 0–1 |
| Plan EFI repair | Review ESP, enter preservation directory, generate preview. The GUI cannot execute BCDBoot | 1–2 |
| Locate Windows media | Enter the image or mounted media directory, inspect, select index and GPT disk | 1 |
| Select a prior backup | Enter destination and list checkpoints, or enter the operation directory in the consuming form | 1 |
| Prepare restore | Enter the operation directory and export directory, then verify and export the reference | 2 |
| Export session report | Enter an independent directory, then export | 1 |
| Open diagnostics console | Console action opens a separate WinPE command prompt | 0 |

This is a code and design inventory. It records required fields and controls. Interaction counts require an observed runtime walkthrough. Baseline backup copy and resume each require one explicit confirmation. The restore handoff does not execute an offline profile write. Deployment stops at a planning and export boundary.
