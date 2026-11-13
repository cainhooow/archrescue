---
title: "ArchRescue — Roadmap Mestre e Espelho Offline do ClickUp"
project: "ArchRescue"
document_id: "AR-ROADMAP-001"
version: "1.1.0"
updated_at: "2026-11-09"
language: "pt-BR"
source: "ArchRescue_ClickUp_Backlog_Import.csv"
clickup_sync: "VERIFICADO EM 2026-11-09: 194 ITENS, 19 ÉPICOS, 175 SUBTAREFAS"
mvp: "MVP v0.1.0 DEFINIDO; AR-00-02 NO CLICKUP: planning; P0 v0.0.1 PENDENTE DE EXECUÇÃO"
---

# ArchRescue — Roadmap Mestre

> **Projeto brasileiro, gratuito e open-source.** Este documento é o espelho offline editável do planejamento importado para o ClickUp. Ele não substitui a verificação do estado real do ClickUp quando a integração estiver disponível.
>
> **Data de referência:** 09/11/2026 • **Versão do documento:** 1.1.0 • **Estado do projeto:** planejamento/MVP definido; desenvolvimento ainda não homologado • **Próxima execução:** ArchRescue P0 v0.0.1 e QA real em VM.

## 1. Escopo e fonte de verdade

- **Produto:** ArchRescue, plataforma de recuperação, manutenção, backup, migração e implantação do Windows, de origem brasileira, gratuita e open-source.
- **Distribuição A — ArchRescue Boot ISO:** ambiente inicializável, pensado para hardware físico e VMs, com painel de recuperação.
- **Distribuição B — ArchRescue Creator:** aplicativo para Windows que cria e atualiza pendrives de recuperação e pode produzir mídias personalizadas.
- **Componentes compartilhados:** ArchRescue Core (proposta em Rust), Image Studio (importação/customização de imagens Windows) e Restore Agent (fase pós-instalação).
- **ClickUp de referência:** [Desenvolvimento → ArchRescue](https://app.clickup.com/90171344329/v/f/1600600000000048/90176222943).
- **Identificadores de localização:** workspace `90171344329`, espaço `90176222943`, pasta `1600600000000048`, lista `1600600000000050` (`List`).
- **Baseline importado:** 17 épicos + 158 subtarefas = **175 itens**, conferidos no ClickUp.
- **Extensões sincronizadas:** AR-17 e AR-18 (2 épicos + 14 subtarefas); AR-P0-001, AR-QA-P0 e AR-INPLACE-001 (3 subtarefas adicionais em épicos existentes).
- **Estado remoto verificado em 09/11/2026:** **194 itens (19 épicos + 175 subtarefas)**; **193 `to do` e 1 `planning`**. O item em `planning` é **AR-00-02 / MVP-001**; não representa desenvolvimento executado.
- **Identificadores:** as 175 tarefas originais agora têm IDs reais reconciliados neste documento; AR-17/AR-18 e as três novas tarefas também têm links.
- **Natureza do registro:** snapshot da API naquela data; alterações posteriores exigem nova leitura antes de atualizar estados.

### 1.1. Regras de atualização e precedência

1. **Com ClickUp acessível:** conferir IDs e status das tarefas antes de alterar este arquivo; registrar `SYNC` no diário. ClickUp é a referência operacional compartilhada.
2. **Sem ClickUp:** atualizar **este arquivo** como fonte operacional temporária. Cada mudança de estado precisa de evidência, data e linha no diário de alterações.
3. **Ao reconectar:** reconciliar por código estável (ex.: `AR-04-03`) + título, nunca por número sequencial da planilha. O `Task ID` numérico do CSV foi uma referência de importação e **não é** o ID definitivo da tarefa do ClickUp.
4. **Conflitos:** não sobrescrever status `CONCLUÍDO` ou descrições alteradas remotamente sem inspeção. Registrar decisão de merge no histórico.
5. **Sem falso progresso:** na consulta de 09/11/2026, havia 193 tarefas em `to do` e 1 em `planning`; isso é **status de gestão**, não prova de implementação. Somente testes executados geram evidência de conclusão.
6. **Novos itens:** verificar duplicatas por ID e título; registrar operações bem-sucedidas antes de repetir chamadas de criação.
7. **Separar escopo de execução:** definição documental de P0/MVP não significa protótipo implementado, ISO gerada ou QA aprovado.

### 1.2. Convenção de status e evidências

| Estado | Significado | Como registrar |
|---|---|---|
| `BACKLOG` | Catalogado, ainda não iniciado neste espelho | Caixa `[ ]`; sem evidência de execução |
| `PRONTO` | Refinado, critérios e dependências claros | Motivo, responsável se houver |
| `EM_ANDAMENTO` | Execução comprovada | Branch/PR, logs ou referência de trabalho |
| `BLOQUEADO` | Não pode avançar | Bloqueio explícito, quem resolve, próximo passo |
| `EM_QA` | Implementado, aguardando verificação | Link para testes e evidências |
| `CONCLUÍDO` | Critérios aceitos com provas | Caixa `[x]`, data, evidência, resultado |
| `CANCELADO` | Retirado do escopo | Justificativa, decisão, impacto e data |

**Importante:** marcar `[x]` apenas quando houver evidência suficiente; os checkboxes de critérios de aceite podem ser marcados separadamente e não implicam que a tarefa foi concluída. Nenhuma data ou responsável foi inventado.

## 2. Visão de execução por estágios

| Estágio | Objetivo | Entrega de saída | Condição para seguir |
|---|---|---|---|
| **P0 v0.0.1 (próximo ciclo)** | Primeiros 10 recursos reais em ambiente inicializável | Protótipo/kit de build + testes VM, hashes, evidências | 10/10 testes reais e garantias de integridade; senão `BLOCKED`/`REJECTED` |
| **MVP v0.1.0** | Fluxo mínimo de recuperação e reinstalação assistida | Diagnóstico, reparo controlado, backup verificado, instalação em laboratório e restauração | E2E e negative tests sem perdas; parecer de distribuição |
| M0 — Fundação | Requisitos, arquitetura, segurança e viabilidade jurídica | ADRs, modelo de ameaça, decisão de escopo | Riscos críticos conhecidos e aprovados |
| M1 — Primeiro boot | ISO x64 UEFI, shell, UI, inventário de discos | ArchRescue inicializando em laboratório | Boot e leitura segura aprovados |
| M2 — Reparo | Diagnóstico/repair engine baseado em evidências | Reparos controlados e verificáveis | Regressão em VMs e tratamento de risco |
| M3 — Proteção dos dados | Explorador, backup seletivo, manifesto, integridade | Backup recuperável e conferido | Nenhum wipe sem pré-condições atendidas |
| M4 — Deployment | Instalação limpa com origem ISO/WIM | Instalação reproduzível de Windows | Identidade do disco/consentimento/testes E2E |
| M5 — Restauração | Restore Agent, perfis, apps e drivers | Migração e validação pós-instalação | Compatibilidade e evidências do restore |
| Creator | Gravar/atualizar mídias e gerenciar conteúdo | Aplicativo Creator para Windows | ISO base estável e mídia verificada |
| Studio | Importar/inspecionar/gerar imagens personalizadas | Image Studio, ISO original preservada | Build assinado/testado e licenças avaliadas |
| Transversal | QA, supply chain, documentação, licenças | Gates contínuos | Não liberar sem aprovação |
| Post-MVP | Plugins, IA opcional, recuperação avançada e legado | RFCs e protótipos | A definir em versões futuras |

> **As fases acima são marcos técnicos, não compromissos de datas.** O P0 v0.0.1 antecede o MVP v0.1.0. A escolha da base WinPE e dos artefatos públicos ainda depende de viabilidade técnica e revisão dos termos de redistribuição.

## 3. Marcos, dependências e gates

### 3.1. Dependências técnicas principais

| Item | Depende de | Observação |
|---|---|---|
| AR-01 (Boot ISO) | AR-00; parte de AR-12/AR-13 | Confirmar solução de build e distribuição do WinPE |
| AR-02 (Painel) | AR-01 runtime e shell | Interface deve funcionar de fato no WinPE |
| AR-03 (Inventário) | AR-01 | Leitura de discos estável antes de operações que alteram dados |
| AR-04 (Reparo) | AR-03 e políticas AR-12 | Nunca executar mudanças sem pré-checagem e plano explícito |
| AR-05 (Backup) | AR-03 e controles AR-12 | Destino validado; cópia verificável |
| AR-06 (Implantação) | AR-03, AR-05, AR-12 e AR-13 | Preservação e confirmação antes de formatar |
| AR-07 (Restore Agent) | AR-05 e AR-06 | Desenho de dados e retomada seguros |
| AR-08 (Apps/drivers) | AR-03, AR-06/AR-07 e AR-13 | Drivers compatíveis; licenças de instaladores |
| AR-10 (Creator) | AR-01, AR-03 e AR-12 | Aproveitar o boot validado; cuidado com disco escolhido |
| AR-09 (Image Studio) | AR-06, AR-12 e AR-13 | ISO original imutável, ambiente de trabalho isolado |
| AR-14/AR-15 (QA/docs) | Transversais | Não deixar para o fim |
| **AR-P0-001** | AR-00-02 e pré-requisitos de ambiente | Build, UI e ferramentas reais; não iniciar qualquer operação destrutiva |
| **AR-QA-P0** | AR-P0-001 | QA separado; boot em 2 plataformas VM, hashes, dados de teste e resultados reprodutíveis |
| **AR-INPLACE-001** | AR-00-02; AR-05/06/07 e modelo de ameaças | PoC isolada em VM, sem habilitação na versão inicial |
| AR-17/AR-18 (sincronizados) | M0 | Open source/benchmark são requisitos de produto; ambos cadastrados no ClickUp |

### 3.2. Gates de aprovação

- [ ] **G0 — MVP definido documentalmente, aprovação técnica pendente:** a decisão de escopo v0.1.0 foi registrada no ClickUp (`planning`). Formalizar revisão de riscos, artefatos de aceite e mudanças antes de iniciar operações destrutivas.
- [ ] **G1 — Distribuição viável:** parecer sobre WinPE/ADK e estratégia de distribuição compatível com software open-source.
- [ ] **G2 — Segurança de disco:** identidade forte, seleção confirmada, operação dry-run, proteção de mídia e testes contra escrita errada.
- [ ] **G3 — Boot comprovado:** ISO inicia em pelo menos duas configurações representativas de VM e em hardware de teste (conforme matriz aprovada).
- [ ] **G4 — Recuperação/backup comprovados:** cenários controlados, hashes/manifesto, logs e falhas simuladas.
- [ ] **G5 — Publicação:** CI, hashes, SBOM, documentação, licença validada, release notes e resultados de QA.

## 4. Definição de pronto (DoD) global

Uma tarefa só pode ser marcada como `CONCLUÍDO` quando: (a) o comportamento planejado foi implementado; (b) os critérios de aceite específicos foram cumpridos; (c) testes positivos/negativos e evidências foram anexados ou referenciados; (d) documentação e riscos foram atualizados; (e) alterações destrutivas receberam testes adicionais e revisão; (f) o status foi conciliado com ClickUp quando disponível. Para estudos/ADRs, basta documento aprovado e evidência de revisão — não código.

## 5. MVP definido e protótipo P0 — contrato de execução

**Estado de gestão:** o escopo do MVP foi definido nesta conversa e registrado como `planning` na [tarefa AR-00-02](https://app.clickup.com/t/1dfqjx6uw4w); **nenhum código, ISO ou teste foi declarado concluído**. A implementação inicia por [AR-P0-001](https://app.clickup.com/t/1dfqjx6uxnh), não por todas as funcionalidades do produto.

### 5.1. P0 v0.0.1 — primeiro protótipo executável (~10% do produto)

| Código | Funcionalidade real obrigatória | Evidência mínima para PASS |
|---|---|---|
| P0-F01 | Inicialização x64 UEFI do ArchRescue | Boot em VM, versão, shell iniciado e logs reais |
| P0-F02 | Interface gráfica pt-BR | UI funcional para operações implementadas, sem botões falsos |
| P0-F03 | Enumeração real de discos | IDs, barramento/capacidade e tratamento de erros |
| P0-F04 | Inspeção de GPT/EFI/NTFS | Volumes e tipos identificados em discos de teste |
| P0-F05 | Detecção de instalações Windows offline | Instalação válida reconhecida; falsos positivos recusados |
| P0-F06 | Informações reais de espaço | Espaço usado/livre ou indisponibilidade declarada |
| P0-F07 | Inspeção BCD/EFI somente leitura | Evidências de existência/ausência; nenhuma escrita no boot |
| P0-F08 | Explorador de arquivos offline | Navegação real, seleção e tratamento de permissões |
| P0-F09 | Backup seletivo para volume distinto | Destino validado, cópia, manifesto e hashes conferidos |
| P0-F10 | Logs e relatório técnico exportável | Artefatos reais com erros, limitações e operações |

**Aceitação P0:** teste executado em duas plataformas de VM representativas, imagens virtuais descartáveis e casos negativos de volume incorreto, pouco espaço, corrupção de metadados, falha de I/O, arquivo inacessível e cancelamento. Exigir comando/procedimento, logs, hash do artefato e resultado observável. `PASS` apenas com verificação real, `FAIL` quando não atende, `BLOCKED` por pré-requisito faltante e `NOT TESTED` quando não executado.

**Limites P0:** não formatar, não instalar Windows, não alterar BCD/EFI, não modificar a origem. Cópia é permitida **somente para destino autorizado e fisicamente distinguível**, com bloqueio de sobreposição e pré-checagem. O protótipo não pode ser homologado apenas com screenshots, `cargo test` ou existência de arquivo ISO.

**Dependências operacionais:** toolchain Rust/Win32, ADK + WinPE Add-on em Windows quando aplicável, máquina de build autorizada, laboratório QEMU/OVMF mais segunda plataforma VM. Se o host não suportar WinPE build/VM, produzir tudo que for possível, deixar o build reproduzível e registrar `BLOCKED` em vez de fabricar evidências.

### 5.2. MVP v0.1.0 — primeira jornada de recuperação completa

**Público:** técnicos de informática, com modo assistido para usuários finais. **Compatibilidade inicial:** Windows 11 x64, UEFI/GPT, NTFS e discos saudáveis; BitLocker somente com desbloqueio autorizado. Matriz exata de VM e hardware de laboratório será homologada.

**Incluído no MVP:**

1. Boot inicializável e UI real em pt-BR, com modos básico assistido e técnico.
2. Inventário seguro, identificação de instalações e diagnóstico EFI/BCD baseado em evidências.
3. Pelo menos um reparo de boot controlado, com plano de execução, pré-checagens e resultado verificável.
4. Backup seletivo das pastas de usuário, destino independente, manifesto, hash e teste de restauração.
5. Instalação assistida do Windows 11 por ISO compatível em cenários de laboratório controlados, com escolha inequívoca do disco e consentimento para qualquer alteração destrutiva.
6. Restore Agent básico para reconstruir diretórios Desktop/Documentos/Downloads/Imagens no novo perfil e reconciliar conflitos de arquivos.
7. Logs e relatório final com arquivos restaurados, perdidos/ignorados, falhas e estado do sistema.
8. Pipeline build/test, documentação em inglês e UI pt-BR, política de privacidade e revisão de licenças.

**Fora do MVP:** Creator completo, Image Studio, multiboot universal, injeção genérica em ISOs de terceiros, migração universal de programas e credenciais, IA autônoma, gerenciamento remoto, suporte irrestrito a hardware antigo/RAID/EFS, e recuperação de discos fisicamente degradados. Esses itens continuam no backlog.

**Gate de MVP:** uma VM com Windows danificado deve percorrer diagnóstico → tentativa de reparo → backup verificado → reinstalação autorizada → restauração → validação final. Testes negativos e falha segura são mandatórios; não habilitar caminhos destrutivos em produção sem QA específico e revisão de segurança.

### 5.3. Preserve & Rebuild — pesquisa vinculada, não recurso liberado

**Questão de produto:** é possível reduzir cópias e reconstruir rapidamente o perfil do Windows antigo durante uma reinstalação?

**Resposta arquitetural:** estudar reinstalação **sem formatação na mesma partição**, com preservação compatível via `Windows.old` e avaliação de USMT (`ScanState`, `LoadState`, hard-link store) em cenários suportados. Hard links evitam duplicar conteúdo físico no mesmo volume, mas **não são backup**, não atravessam volumes e deixam de existir quando o volume é formatado. Windows.old e USMT não são alternativas intercambiáveis e requerem testes de compatibilidade com o fluxo real de Setup.

A restauração do perfil só deve acontecer quando o Windows recém-instalado e o Restore Agent tiverem criado/identificado os usuários, validado ACLs e resolvido colisões. **Não executar cópia concorrente para o mesmo volume enquanto o instalador modifica arquivos sem garantias explícitas de consistência.** Comparar desempenho com migração entre dois volumes independentes antes de prometer ganho de tempo. Nenhum fluxo elimina o requisito de espaço livre para instalação, arquivos temporários e aplicativos.

**Status:** [AR-INPLACE-001](https://app.clickup.com/t/1dfqjx6uxnj) em `to do`; PoC em VMs/snapshots, fora da homologação P0. Só aprovar após testes de integridade, espaço, interrupção, BitLocker/EFS/ACL, perfil/SID e risco de perda de dados.

### 5.4. Prioridade de execução

1. Encerrar requisitos e riscos imediatos de G0/G1 para build e distribuição.
2. Implementar [AR-P0-001](https://app.clickup.com/t/1dfqjx6uxnh) com dez funções demonstráveis e componentes reais.
3. Executar [AR-QA-P0](https://app.clickup.com/t/1dfqjx6uxnk) de forma independente; reprovar se faltar boot ou evidência.
4. Iterar ao MVP v0.1.0 após segurança e compatibilidade das operações destrutivas.
5. Pesquisar [AR-INPLACE-001](https://app.clickup.com/t/1dfqjx6uxnj) em trilha paralela não bloqueante, sem apressar sua liberação.

## 6. Arquitetura-alvo (proposta sujeita a ADR)

```mermaid
flowchart TD
    A[ArchRescue Core — Rust] --> B[Boot Shell — WinPE]
    A --> C[Creator — Windows]
    A --> D[Restore Agent — Windows instalado]
    C --> E[Media Builder — ISO e USB]
    C --> F[Image Studio — WIM e ISO]
    B --> G[Inventário e diagnóstico]
    B --> H[Backup e reparos]
    B --> I[Deployment]
    I --> D
    G --> J[Manifestos, evidências e logs]
    H --> J
    D --> J
```

**Contratos candidatos:** `OperationPlan`, `DiskIdentity`, `BackupManifest`, `ISOManifest`, `RepairStep`, `Evidence`, `Result`. Empregar schemas versionados, validação estrita e checkpoints. Comunicação JSON nas bordas quando útil; implementação interna livre para modelos tipados e formatos eficientes. Não utilizar IA como autoridade final para escrever em discos.

## 7. Princípios invioláveis

1. **Privacidade:** execução offline primeiro, telemetria facultativa e sem coleta silenciosa de credenciais.
2. **Preservação:** nunca presumir que arquivos estão salvos sem backup e validação; nenhuma formatação sem consentimento.
3. **Discos degradados:** minimizar escritas e leituras agressivas; privilegiar imagem/cópia apropriada, se possível.
4. **BitLocker:** somente desbloqueio autorizado; não prometer contorno de criptografia ou recuperação de senhas.
5. **Drivers:** exportar e restaurar apenas pacotes compatíveis, confiáveis e quando possível assinados.
6. **Apps:** instaladores reais após implantação; copiar Program Files não é migração universal. Licenças e dados protegidos exigem tratamento específico.
7. **ISOs externas:** preservar a original, validar hash/origem, isolar build e não confiar em personalizações desconhecidas.
8. **Secure Boot:** suportar cadeia aprovada, sem pedir desativação como solução padrão.
9. **Open source verdadeiro:** publicar código próprio sob licença reconhecida; WinPE/Windows e terceiros continuam sujeitos a licenças próprias.
10. **Brasil primeiro:** pt-BR como idioma principal, documentação bilíngue progressiva, comunidade aberta, sem paywall funcional.

## 8. Backlog integral importado — 17 épicos e 158 subtarefas

**Legenda:** os checkboxes continuam desmarcados porque nenhuma implementação foi comprovada. Os IDs `AR-xx-yy` são identificadores estáveis locais; o título original e ID numérico de importação são mantidos. **Todos os 175 itens originais agora têm link para a tarefa real do ClickUp**, verificada em 09/11/2026. As descrições técnicas foram preservadas integralmente.


### AR-00 — Governança, requisitos e arquitetura

- **ID ClickUp verificado:** [`1dfqjx6uw49`](https://app.clickup.com/t/1dfqjx6uw49) • status: `to do` em 09/11/2026

- **Fase:** `M0` • **Prioridade de importação:** Alta • **ID CSV do épico:** `1` • **Subtarefas:** 9

- **Status local:** `BACKLOG` • **ClickUp:** `to do` verificado em 09/11/2026

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Estabelecer um desenho verificável antes de implementar ferramentas que tocam em dados de clientes.

**Subtarefas:**


#### AR-00-01 — Definir requisitos funcionais e perfis de uso

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `2` | **ClickUp:** [`1dfqjx6uw4v`](https://app.clickup.com/t/1dfqjx6uw4v)

**Objetivo e escopo**
Especificar modos Assistido, Técnico e Laboratório, persona de técnico e usuário leigo, operações permitidas por modo.

**Critérios de aceite**
- Documento versionado com jornadas principais, entradas, saídas e fluxos de erro.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-00-02 — Formalizar limites do MVP e roadmap M0–M6

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `3` | **ClickUp:** [`1dfqjx6uw4w`](https://app.clickup.com/t/1dfqjx6uw4w)

**Objetivo e escopo**
Separar o que entra no primeiro Boot ISO, no Creator e no Image Studio, deixando recursos avançados para versões posteriores.

**Critérios de aceite**
- Escopo com itens incluídos/excluídos, dependências e marcos de aprovação.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-00-03 — Elaborar ADR de arquitetura Rust + WinPE + agente pós-instalação

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `4` | **ClickUp:** [`1dfqjx6uw4u`](https://app.clickup.com/t/1dfqjx6uw4u)

**Objetivo e escopo**
Especificar crates compartilhadas, processos privilegiados, UI do WinPE, aplicação Creator e Restore Agent.

**Critérios de aceite**
- Diagrama de componentes, fronteiras de confiança e ADR aceito.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-00-04 — Definir contratos de dados e modelo de execução

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `5` | **ClickUp:** [`1dfqjx6uw4x`](https://app.clickup.com/t/1dfqjx6uw4x)

**Objetivo e escopo**
Modelar OperationPlan, DiskIdentity, BackupManifest, ISOManifest, RepairStep, Evidence e Result, com versões de schema.

**Critérios de aceite**
- Schemas versionados, exemplos válidos e política de compatibilidade.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-00-05 — Definir estados de operação, cancelamento e retomada

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `6` | **ClickUp:** [`1dfqjx6uw4y`](https://app.clickup.com/t/1dfqjx6uw4y)

**Objetivo e escopo**
Modelar pending/running/waiting/verified/failed/rolled_back, idempotência e checkpoints persistidos em volume seguro.

**Critérios de aceite**
- Máquina de estados testada contra interrupção, repetição e recuperação.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-00-06 — Definir política de mudanças destrutivas e consentimento

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `7` | **ClickUp:** [`1dfqjx6uw4z`](https://app.clickup.com/t/1dfqjx6uw4z)

**Objetivo e escopo**
Estabelecer confirmação explícita para formatação, escrita em partições, reparos não reversíveis e perda potencial de dados.

**Critérios de aceite**
- Matriz de risco e fluxos de consentimento com alvo físico identificado.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-00-07 — Criar padrões de documentação, nomenclatura e versionamento

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `8` | **ClickUp:** [`1dfqjx6uw50`](https://app.clickup.com/t/1dfqjx6uw50)

**Objetivo e escopo**
Padronizar estrutura do repo, convenções de código, semver, changelog, ADR, runbooks e nomenclatura de artefatos.

**Critérios de aceite**
- Padrões documentados e templates reutilizáveis no repositório.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-00-08 — Definir compatibilidade e suporte por hardware/firmware

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `9` | **ClickUp:** [`1dfqjx6uw51`](https://app.clickup.com/t/1dfqjx6uw51)

**Objetivo e escopo**
Matriz x64/UEFI/Secure Boot; definir BIOS legado como extensão sujeita a testes; VM Hyper-V, VirtualBox, VMware, QEMU.

**Critérios de aceite**
- Matriz publicada com ambientes suportados, parciais e não suportados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-00-09 — Criar modelo de ameaças e plano de privacidade

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `10` | **ClickUp:** [`1dfqjx6uw52`](https://app.clickup.com/t/1dfqjx6uw52)

**Objetivo e escopo**
Listar ameaças a dados de clientes, mídia removível, ISO adulterada, execução de binários e credenciais.

**Critérios de aceite**
- Threat model e mitigações priorizadas, com testes associados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-01 — Build, inicialização e ambiente WinPE

- **ID ClickUp verificado:** [`1dfqjx6uw4a`](https://app.clickup.com/t/1dfqjx6uw4a) • status: `to do` em 09/11/2026

- **Fase:** `M1` • **Prioridade de importação:** Alta • **ID CSV do épico:** `11` • **Subtarefas:** 9

- **Status local:** `BACKLOG` • **ClickUp:** `to do` verificado em 09/11/2026

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Entregar a primeira ISO do ArchRescue, com boot reproduzível e ambiente estável.

**Subtarefas:**


#### AR-01-01 — Provisionar pipeline ADK + WinPE add-on

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `12` | **ClickUp:** [`1dfqjx6uw53`](https://app.clickup.com/t/1dfqjx6uw53)

**Objetivo e escopo**
Documentar versões suportadas e construir ambiente reproduzível de build em Windows.

**Critérios de aceite**
- Build limpo com dependências fixadas, log e hashes.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-01-02 — Gerar ISO inicializável x64 UEFI

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `13` | **ClickUp:** [`1dfqjx6uw54`](https://app.clickup.com/t/1dfqjx6uw54)

**Objetivo e escopo**
Personalizar boot.wim e gerar mídia ISO com MakeWinPEMedia ou equivalente compatível.

**Critérios de aceite**
- Boot funcional em VM e bare metal UEFI, sem prompt obrigatório.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-01-03 — Integrar bootstrap do ArchRescue no WinPE

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `14` | **ClickUp:** [`1dfqjx6uw55`](https://app.clickup.com/t/1dfqjx6uw55)

**Objetivo e escopo**
Substituir shell inicial por launcher com detecção de erros e fallback técnico.

**Critérios de aceite**
- Aplicação abre após boot; falha permite diagnóstico e recuperação.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-01-04 — Validar componentes opcionais necessários do WinPE

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `15` | **ClickUp:** [`1dfqjx6uw56`](https://app.clickup.com/t/1dfqjx6uw56)

**Objetivo e escopo**
Identificar pacotes para rede, armazenamento, scripting e dependências concretas da UI.

**Critérios de aceite**
- Lista mínima de pacotes; testes que demonstram por que cada um é necessário.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-01-05 — Implementar runtime Rust compatível com WinPE

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `16` | **ClickUp:** [`1dfqjx6uw57`](https://app.clickup.com/t/1dfqjx6uw57)

**Objetivo e escopo**
Gerar executáveis x64, empacotar DLLs necessárias e validar ausência de dependências indisponíveis.

**Critérios de aceite**
- Aplicação abre em WinPE limpo; dependências e licenças documentadas.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-01-06 — Implementar localização da mídia e volumes após boot

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `17` | **ClickUp:** [`1dfqjx6uw58`](https://app.clickup.com/t/1dfqjx6uw58)

**Objetivo e escopo**
Descobrir onde estão imagens, ferramentas, planos e a partição de dados, independentemente da letra de unidade.

**Critérios de aceite**
- USB e ISO de VM mapeados por identidade, sem caminhos rígidos.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-01-07 — Adicionar recuperação de shell e modo diagnóstico

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `18` | **ClickUp:** [`1dfqjx6uw59`](https://app.clickup.com/t/1dfqjx6uw59)

**Objetivo e escopo**
Habilitar console controlado quando interface falhar, com exportação de logs para mídia externa.

**Critérios de aceite**
- Fallback testado em falha intencional da UI.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-01-08 — Montar suíte de testes de boot para VM

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `19` | **ClickUp:** [`1dfqjx6uw5a`](https://app.clickup.com/t/1dfqjx6uw5a)

**Objetivo e escopo**
Automatizar smoke tests em QEMU/Hyper-V quando disponíveis, validando tela inicial e acesso à mídia.

**Critérios de aceite**
- Logs e capturas de boot reproduzíveis em pelo menos duas VMs.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-01-09 — Verificar assinatura e cadeia de inicialização UEFI

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `20` | **ClickUp:** [`1dfqjx6uw5b`](https://app.clickup.com/t/1dfqjx6uw5b)

**Objetivo e escopo**
Estabelecer cadeia compatível com Secure Boot e certificados atuais, sem desabilitação automática.

**Critérios de aceite**
- Boot de teste com Secure Boot habilitado em ambiente suportado.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-02 — Painel Boot ISO e experiência do usuário

- **ID ClickUp verificado:** [`1dfqjx6uw4b`](https://app.clickup.com/t/1dfqjx6uw4b) • status: `to do` em 09/11/2026

- **Fase:** `M1` • **Prioridade de importação:** Alta • **ID CSV do épico:** `21` • **Subtarefas:** 9

- **Status local:** `EM_ANDAMENTO` • **ClickUp:** `to do` verificado em 09/11/2026

- **Wave 2:** A interface guiada oferece inspeção de mídia, seleção de disco GPT, prévia de implantação e exportação de referência de restauração. [Relatório da Wave 2](docs/qa/M4-M5-DEVELOPMENT-WAVE.md)

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Painel utilizável por técnicos e usuários assistidos sem depender do terminal.

**Subtarefas:**


#### AR-02-01 — Definir design system do painel ArchRescue

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `22` | **ClickUp:** [`1dfqjx6uw5c`](https://app.clickup.com/t/1dfqjx6uw5c)

**Objetivo e escopo**
Desenvolver interface escura, tipografia, ícones, grids, estados e modos de alto contraste.

**Critérios de aceite**
- Biblioteca visual e guia de componentes aprovados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-02-02 — Implementar dashboard inicial de diagnóstico

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `23` | **ClickUp:** [`1dfqjx6uw5d`](https://app.clickup.com/t/1dfqjx6uw5d)

**Objetivo e escopo**
Mostrar sistemas instalados, discos, capacidade, alertas, criptografia e próximos passos.

**Critérios de aceite**
- Dados exibidos refletem detecção real; mensagens de risco claras.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-02-03 — Criar navegação por oito categorias de utilidades

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `24` | **ClickUp:** [`1dfqjx6uw5e`](https://app.clickup.com/t/1dfqjx6uw5e)

**Objetivo e escopo**
Recuperação, Instalação, Discos, Backup, Hardware, Ferramentas, Rede, Aplicativos/Relatórios.

**Critérios de aceite**
- Todas as categorias acessíveis com navegação consistente.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-02-04 — Implementar assistente de recuperação orientado por etapas

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `25` | **ClickUp:** [`1dfqjx6uw5f`](https://app.clickup.com/t/1dfqjx6uw5f)

**Objetivo e escopo**
Wizard Detectar → Diagnosticar → Reparar → Backup → Instalar → Restaurar → Validar.

**Critérios de aceite**
- Usuário vê etapa atual, ação seguinte, cancelamento e retorno seguro.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-02-05 — Desenvolver modo Técnico avançado

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `26` | **ClickUp:** [`1dfqjx6uw5g`](https://app.clickup.com/t/1dfqjx6uw5g)

**Objetivo e escopo**
Expor parâmetros, evidências, comandos gerados e logs sem executar ações silenciosas.

**Critérios de aceite**
- Modo separa claramente leitura de comandos modificadores.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-02-06 — Criar centro de notificações e erros

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `27` | **ClickUp:** [`1dfqjx6uw5h`](https://app.clickup.com/t/1dfqjx6uw5h)

**Objetivo e escopo**
Mensagens acessíveis com código, gravidade, ação recomendada e detalhes exportáveis.

**Critérios de aceite**
- Erros de permissão, disco e rede são diferenciados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-02-07 — Implementar barras de progresso reais e cancelamento

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `28` | **ClickUp:** [`1dfqjx6uw5j`](https://app.clickup.com/t/1dfqjx6uw5j)

**Objetivo e escopo**
Eventos estruturados com contadores de bytes/arquivos/etapas e cancelamento cooperativo.

**Critérios de aceite**
- Progresso não fictício; cancelamento deixa estado rastreável.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-02-08 — Adicionar idioma pt-BR e base para i18n

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `29` | **ClickUp:** [`1dfqjx6uw5k`](https://app.clickup.com/t/1dfqjx6uw5k)

**Objetivo e escopo**
Extrair textos, traduções, formatos regionais e suporte a keyboard layout.

**Critérios de aceite**
- Interface pt-BR completa e ausência de strings hardcoded.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-02-09 — Testar usabilidade em tela pequena e resolução básica

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `30` | **ClickUp:** [`1dfqjx6uw5m`](https://app.clickup.com/t/1dfqjx6uw5m)

**Objetivo e escopo**
Garantir foco por teclado, escala de UI, navegação sem mouse e legibilidade em 1024x768.

**Critérios de aceite**
- Fluxos críticos executáveis sem clipping ou telas inacessíveis.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-03 — Inventário, discos e hardware

- **ID ClickUp verificado:** [`1dfqjx6uw4c`](https://app.clickup.com/t/1dfqjx6uw4c) • status: `to do` em 09/11/2026

- **Fase:** `M1` • **Prioridade de importação:** Alta • **ID CSV do épico:** `31` • **Subtarefas:** 9

- **Status local:** `EM_ANDAMENTO` • **ClickUp:** `to do` verificado em 09/11/2026

- **Wave 2:** O planejador coleta identidade GPT, firmware, capacidade, partições, volumes e estado de proteção. A matriz completa de diagnóstico segue aberta. [Relatório da Wave 2](docs/qa/M4-M5-DEVELOPMENT-WAVE.md)

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Conhecer o estado do dispositivo antes de qualquer reparo ou reinstalação.

**Subtarefas:**


#### AR-03-01 — Enumerar discos e volumes com identidade estável

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `32` | **ClickUp:** [`1dfqjx6uw5p`](https://app.clickup.com/t/1dfqjx6uw5p)

**Objetivo e escopo**
Identificar modelo, serial, barramento, capacidade, partições e letras atuais sem confundir volumes.

**Critérios de aceite**
- Mapa coerente mesmo quando letras mudam e com múltiplos USBs.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-03-02 — Detectar instalações Windows offline

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `33` | **ClickUp:** [`1dfqjx6uw5n`](https://app.clickup.com/t/1dfqjx6uw5n)

**Objetivo e escopo**
Localizar Windows, perfis e caminhos relevantes; identificar versão, build, arquitetura e status acessível.

**Critérios de aceite**
- Lista correta em dual boot, Windows.old e unidades secundárias.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-03-03 — Identificar GPT/MBR e partições EFI/MSR/Recovery

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `34` | **ClickUp:** [`1dfqjx6uw5v`](https://app.clickup.com/t/1dfqjx6uw5v)

**Objetivo e escopo**
Analisar partições, seus tipos e relação com instalações e entradas de boot.

**Critérios de aceite**
- Visualização correta em UEFI, GPT e casos com ESP duplicada.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-03-04 — Exibir estado SMART/NVMe e alertas de risco

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `35` | **ClickUp:** [`1dfqjx6uw5q`](https://app.clickup.com/t/1dfqjx6uw5q)

**Objetivo e escopo**
Integrar consulta de saúde quando hardware/controlador permitir; indicar dados indisponíveis.

**Critérios de aceite**
- Falhas de leitura e alertas não são tratados como disco saudável.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-03-05 — Detectar BitLocker e solicitar desbloqueio autorizado

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `36` | **ClickUp:** [`1dfqjx6uw5r`](https://app.clickup.com/t/1dfqjx6uw5r)

**Objetivo e escopo**
Exibir estado de criptografia, métodos permitidos e chave de recuperação sem armazená-la em logs.

**Critérios de aceite**
- Nenhum acesso indevido; falhas informam credencial necessária.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-03-06 — Criar painel de hardware e drivers ausentes

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `37` | **ClickUp:** [`1dfqjx6uw5t`](https://app.clickup.com/t/1dfqjx6uw5t)

**Objetivo e escopo**
Identificar CPU, RAM, controladores, PCI IDs e dispositivos relevantes para implantação.

**Critérios de aceite**
- Inventário com exportação JSON e identificação de incompatibilidades.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-03-07 — Implementar leitura segura de sistemas de arquivos

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `38` | **ClickUp:** [`1dfqjx6uw5u`](https://app.clickup.com/t/1dfqjx6uw5u)

**Objetivo e escopo**
Ler NTFS e FAT e detectar volumes inacessíveis ou RAW sem montar em escrita por padrão.

**Critérios de aceite**
- Modo de inspeção não altera volumes e documenta exceções.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-03-08 — Construir diagnóstico de partições e boot entries

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `39` | **ClickUp:** [`1dfqjx6uw5w`](https://app.clickup.com/t/1dfqjx6uw5w)

**Objetivo e escopo**
Correlacionar ESP, BCD, WinRE e carregadores com sistemas detectados.

**Critérios de aceite**
- Relatório reproduzível para um disco com duas instalações.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-03-09 — Criar visualizador de partições com marcação de risco

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `40` | **ClickUp:** [`1dfqjx6uw5x`](https://app.clickup.com/t/1dfqjx6uw5x)

**Objetivo e escopo**
Exibir layout físico e destacar volume de origem, destino de backup e alvo de instalação.

**Critérios de aceite**
- Não permitir confirmação apenas por letra de drive.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-04 — Reparo de inicialização e recuperação do Windows

- **ID ClickUp verificado:** [`1dfqjx6uw4d`](https://app.clickup.com/t/1dfqjx6uw4d) • status: `to do` em 09/11/2026

- **Fase:** `M2` • **Prioridade de importação:** Alta • **ID CSV do épico:** `41` • **Subtarefas:** 11

- **Status local:** `EM_ANDAMENTO` • **ClickUp:** `to do` verificado em 09/11/2026

- **Wave 2:** A normalização da raiz da ESP FAT32 foi exercitada em WinPE descartável. A execução do reparo e a inicialização do Windows reparado seguem abertas. [Relatório da Wave 2](docs/qa/M4-M5-DEVELOPMENT-WAVE.md)

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Reparar primeiro, com evidência e sem deteriorar as chances de recuperar dados.

**Subtarefas:**


#### AR-04-01 — Definir motor de diagnóstico baseado em evidências

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `42` | **ClickUp:** [`1dfqjx6uw5y`](https://app.clickup.com/t/1dfqjx6uw5y)

**Objetivo e escopo**
Criar catálogo de checks, hipóteses, pré-condições, confiança e ações; sem comandos cegos.

**Critérios de aceite**
- Cada sugestão aponta para evidência coletada e rollback possível.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-04-02 — Implementar diagnóstico BCD e entradas UEFI

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `43` | **ClickUp:** [`1dfqjx6uw5z`](https://app.clickup.com/t/1dfqjx6uw5z)

**Objetivo e escopo**
Inspecionar BCD e EFI, localizar entradas órfãs, incorretas ou conflitantes.

**Critérios de aceite**
- Casos preparados produzem diagnósticos distintos.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-04-03 — Implementar reparo controlado de BCD e EFI

- [ ] **Concluída** | **Status:** `EM_ANDAMENTO` | **Prioridade:** Alta | **ID CSV:** `44` | **ClickUp:** [`1dfqjx6uw60`](https://app.clickup.com/t/1dfqjx6uw60)

**Objetivo e escopo**
Usar bcdboot/bcdedit e procedimentos específicos; sempre preservar configuração anterior quando possível.

**Critérios de aceite**
- Boot restaurado em casos simulados; nunca altera disco não escolhido.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Wave 2: a raiz GPT/FAT32 abriu no WinPE descartável. O erro 3 nos caminhos internos ainda bloqueia o backup verificado e, por consequência, a execução do reparo. [Reprodução e próximos passos](docs/qa/M4-M5-DEVELOPMENT-WAVE.md#f-safety-validation)
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-04-04 — Criar verificação e reparo de arquivos do Windows offline

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `45` | **ClickUp:** [`1dfqjx6uw61`](https://app.clickup.com/t/1dfqjx6uw61)

**Objetivo e escopo**
Orquestrar DISM/SFC offline com pré-requisitos, origem adequada e tratamento de códigos de saída.

**Critérios de aceite**
- Logs mostram comando, retorno e verificação posterior.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-04-05 — Implementar reversão de atualizações pendentes e problemáticas

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `46` | **ClickUp:** [`1dfqjx6uw62`](https://app.clickup.com/t/1dfqjx6uw62)

**Objetivo e escopo**
Avaliar servicing state e executar opções suportadas, com salvaguardas.

**Critérios de aceite**
- Cenários de update quebrado documentados e testados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-04-06 — Integrar pontos de restauração disponíveis

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `47` | **ClickUp:** [`1dfqjx6uw63`](https://app.clickup.com/t/1dfqjx6uw63)

**Objetivo e escopo**
Detecção e execução pelo ambiente compatível/WinRE quando houver pontos utilizáveis.

**Critérios de aceite**
- Ausência de pontos é tratada sem erro ou promessa falsa.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-04-07 — Implementar reparo de arquivos de sistema e disco com pré-checagem

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `48` | **ClickUp:** [`1dfqjx6uw64`](https://app.clickup.com/t/1dfqjx6uw64)

**Objetivo e escopo**
Oferecer verificações de filesystem e operações CHKDSK somente após avaliação de saúde e risco.

**Critérios de aceite**
- Nunca executa reparo de escrita automaticamente em disco degradado.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-04-08 — Criar fluxo de diagnóstico de BSOD e logs offline

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `49` | **ClickUp:** [`1dfqjx6uw66`](https://app.clickup.com/t/1dfqjx6uw66)

**Objetivo e escopo**
Ler event logs/dumps disponíveis, correlacionar bugchecks e drivers recentes.

**Critérios de aceite**
- Resumo indica evidências, não apenas causa presumida.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-04-09 — Criar política de tentativa, escalonamento e parada

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `50` | **ClickUp:** [`1dfqjx6uw65`](https://app.clickup.com/t/1dfqjx6uw65)

**Objetivo e escopo**
Limitar ciclos de reparo, verificar resultados e oferecer backup ou atendimento manual.

**Critérios de aceite**
- Nenhum loop infinito ou repetição de comando destrutivo.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-04-10 — Criar biblioteca de playbooks de erros recorrentes

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `51` | **ClickUp:** [`1dfqjx6uw67`](https://app.clickup.com/t/1dfqjx6uw67)

**Objetivo e escopo**
Playbooks para bootmgr/BCD, Winload, EFI ausente, update interrompido, corrupção e driver de storage.

**Critérios de aceite**
- Cada playbook tem detecção, remediação, verificação e risco.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-04-11 — Testar reparos com snapshots de VM danificada

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `52` | **ClickUp:** [`1dfqjx6uw69`](https://app.clickup.com/t/1dfqjx6uw69)

**Objetivo e escopo**
Matriz de falhas induzidas e restauração por snapshot após teste.

**Critérios de aceite**
- Taxa de sucesso medida; falhas e falsos positivos registrados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-05 — Backup, cópia de dados e recuperação de arquivos

- **ID ClickUp verificado:** [`1dfqjx6uw4e`](https://app.clickup.com/t/1dfqjx6uw4e) • status: `to do` em 09/11/2026

- **Fase:** `M3` • **Prioridade de importação:** Alta • **ID CSV do épico:** `53` • **Subtarefas:** 10

- **Status local:** `EM_ANDAMENTO` • **ClickUp:** `to do` verificado em 09/11/2026

- **Wave 2:** A verificação do conteúdo de backup passou a atender o Restore Agent sem depender da instalação de origem. [Relatório da Wave 2](docs/qa/M4-M5-DEVELOPMENT-WAVE.md)

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Preservar e verificar dados do cliente antes de operações irreversíveis.

**Subtarefas:**


#### AR-05-01 — Construir explorador de arquivos offline

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `54` | **ClickUp:** [`1dfqjx6uw68`](https://app.clickup.com/t/1dfqjx6uw68)

**Objetivo e escopo**
Árvore de volumes/pastas, filtros, busca, visualização de tamanho e seleção múltipla.

**Critérios de aceite**
- Funciona em perfis de usuário e discos externos; oculta locais inacessíveis corretamente.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-05-02 — Detectar bibliotecas e perfis automaticamente

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `55` | **ClickUp:** [`1dfqjx6uw6b`](https://app.clickup.com/t/1dfqjx6uw6b)

**Objetivo e escopo**
Encontrar Desktop, Documents, Downloads, Pictures, Favorites, AppData selecionável e perfis não padrão.

**Critérios de aceite**
- Inventário correto com múltiplas contas, pastas redirecionadas e Windows.old.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-05-03 — Criar perfis de backup seletivo editáveis

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `56` | **ClickUp:** [`1dfqjx6uw6a`](https://app.clickup.com/t/1dfqjx6uw6a)

**Objetivo e escopo**
Presets documentos, perfis, navegadores, dados comerciais, tudo selecionado e exclusões.

**Critérios de aceite**
- Preview fiel da seleção e estimativa de tamanho.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-05-04 — Selecionar destino com proteção contra auto-sobrescrita

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `57` | **ClickUp:** [`1dfqjx6uw6c`](https://app.clickup.com/t/1dfqjx6uw6c)

**Objetivo e escopo**
Aceitar SSD USB, segundo disco e SMB; bloquear destino dentro de partição que será formatada.

**Critérios de aceite**
- Teste comprova bloqueio para origem/destino incompatíveis.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-05-05 — Desenvolver cópia resiliente com retomada

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `58` | **ClickUp:** [`1dfqjx6uw6d`](https://app.clickup.com/t/1dfqjx6uw6d)

**Objetivo e escopo**
Cópia em blocos, fila persistida, retry limitado, preservação de metadados e cancelamento.

**Critérios de aceite**
- Retoma após interrupção sem duplicar nem corromper arquivos.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-05-06 — Gerar manifesto de backup e hashes verificáveis

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `59` | **ClickUp:** [`1dfqjx6uw6e`](https://app.clickup.com/t/1dfqjx6uw6e)

**Objetivo e escopo**
Salvar lista, tamanhos, hashes quando viável, erros e versão do formato.

**Critérios de aceite**
- Verificador independente detecta arquivos faltantes ou modificados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-05-07 — Implementar verificação pós-backup antes da instalação

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `60` | **ClickUp:** [`1dfqjx6uw6f`](https://app.clickup.com/t/1dfqjx6uw6f)

**Objetivo e escopo**
Comparar manifesto, arquivos críticos e espaço, registrando divergências.

**Critérios de aceite**
- Instalação destrutiva não avança silenciosamente com backup incompleto.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-05-08 — Implementar exportação de dados de navegador e perfis quando permitido

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `61` | **ClickUp:** [`1dfqjx6uw6g`](https://app.clickup.com/t/1dfqjx6uw6g)

**Objetivo e escopo**
Migrar bookmarks e arquivos de configuração portáveis, respeitando criptografia e segredos.

**Critérios de aceite**
- Não promete copiar senhas, tokens ou sessões protegidas.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-05-09 — Adicionar mecanismo de imagem/clonagem em modo seguro

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `62` | **ClickUp:** [`1dfqjx6uw6h`](https://app.clickup.com/t/1dfqjx6uw6h)

**Objetivo e escopo**
Planejar ou integrar imagem de disco bit a bit com logging, checks e destino externo.

**Critérios de aceite**
- Nunca clonar sobre origem; validação de tamanho e integridade.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-05-10 — Criar trilha para discos fisicamente degradados

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `63` | **ClickUp:** [`1dfqjx6uw6j`](https://app.clickup.com/t/1dfqjx6uw6j)

**Objetivo e escopo**
Quando SMART e I/O indicarem risco, interromper reparos e recomendar imagem/leitura mínima.

**Critérios de aceite**
- Falhas físicas acionam política conservadora e aviso ao usuário.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-06 — Instalação limpa e implantação do Windows

- **ID ClickUp verificado:** [`1dfqjx6uw4f`](https://app.clickup.com/t/1dfqjx6uw4f) • status: `to do` em 09/11/2026

- **Fase:** `M4` • **Prioridade de importação:** Alta • **ID CSV do épico:** `64` • **Subtarefas:** 10

- **Status local:** `EM_ANDAMENTO` • **ClickUp:** `to do` verificado em 09/11/2026

- **Wave 2:** Inspeção de WIM, seleção do índice e do disco GPT, qualificação, plano com SHA-256 e exportação foram implementados. A execução da instalação segue desabilitada. [Relatório da Wave 2](docs/qa/M4-M5-DEVELOPMENT-WAVE.md)

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Instalar Windows de modo automático, mas somente após escolha e confirmação conscientes do alvo.

**Subtarefas:**


#### AR-06-01 — Inspecionar fontes ISO/WIM/ESD/SWM

- [ ] **Concluída** | **Status:** `EM_ANDAMENTO` | **Prioridade:** Alta | **ID CSV:** `65` | **ClickUp:** [`1dfqjx6uw6k`](https://app.clickup.com/t/1dfqjx6uw6k)

**Objetivo e escopo**
Ler metadados, edições, arquiteturas, índices e compatibilidade sem confiar em nome do arquivo.

**Critérios de aceite**
- Catálogo correto e erros claros para imagens incompatíveis.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-06-02 — Criar assistente de instalação limpa

- [ ] **Concluída** | **Status:** `EM_ANDAMENTO` | **Prioridade:** Alta | **ID CSV:** `66` | **ClickUp:** [`1dfqjx6uw6m`](https://app.clickup.com/t/1dfqjx6uw6m)

**Objetivo e escopo**
Escolher edição, disco, modo de partição, licença, idioma e políticas pós-instalação.

**Critérios de aceite**
- Plano exibido integralmente antes de permitir escrita no disco.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-06-03 — Gerar plano de implantação imutável e revisável

- [ ] **Concluída** | **Status:** `EM_ANDAMENTO` | **Prioridade:** Alta | **ID CSV:** `67` | **ClickUp:** [`1dfqjx6uw6n`](https://app.clickup.com/t/1dfqjx6uw6n)

**Objetivo e escopo**
Registrar DiskIdentity e operações de criar/apagar/aplicar imagem com hash e confirmação explícita.

**Critérios de aceite**
- Plano persiste sem mudar alvo entre revisão e execução.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-06-04 — Automatizar criação de partições UEFI/GPT

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `68` | **ClickUp:** [`1dfqjx6uw6p`](https://app.clickup.com/t/1dfqjx6uw6p)

**Objetivo e escopo**
Provisionar ESP/MSR/Windows/Recovery conforme configurações e requisitos do alvo.

**Critérios de aceite**
- Máquina inicia em UEFI após implantação em discos de teste.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-06-05 — Implementar aplicação de imagem Windows

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `69` | **ClickUp:** [`1dfqjx6uw6q`](https://app.clickup.com/t/1dfqjx6uw6q)

**Objetivo e escopo**
Orquestrar DISM apply-image ou Setup com fontes válidas e verificação do resultado.

**Critérios de aceite**
- Instala Windows em VM a partir de imagem oficial de teste.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-06-06 — Configurar Windows Setup/unattend de forma suportada

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `70` | **ClickUp:** [`1dfqjx6uw6r`](https://app.clickup.com/t/1dfqjx6uw6r)

**Objetivo e escopo**
Gerar respostas compatíveis sem desativar proteções essenciais indiscriminadamente.

**Critérios de aceite**
- Instalação termina sem intervenção para opções suportadas.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-06-07 — Provisionar arquivos de boot e entrada UEFI

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `71` | **ClickUp:** [`1dfqjx6uw6t`](https://app.clickup.com/t/1dfqjx6uw6t)

**Objetivo e escopo**
Criar BCD de instalação e configurar boot de modo explícito no disco escolhido.

**Critérios de aceite**
- Sem dependência de USB após conclusão do boot inicial.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-06-08 — Adicionar validação de requisitos de hardware

- [ ] **Concluída** | **Status:** `EM_ANDAMENTO` | **Prioridade:** Alta | **ID CSV:** `72` | **ClickUp:** [`1dfqjx6uw6u`](https://app.clickup.com/t/1dfqjx6uw6u)

**Objetivo e escopo**
Checar arquitetura, armazenamento, RAM, TPM/Secure Boot quando exigidos e exibir bloqueios.

**Critérios de aceite**
- Caminho suportado e não suportado claramente separados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-06-09 — Implementar recuperação de falha na implantação

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `73` | **ClickUp:** [`1dfqjx6uw6v`](https://app.clickup.com/t/1dfqjx6uw6v)

**Objetivo e escopo**
Checkpoints em volume externo/seguro e explicação de como retomar ou reverter.

**Critérios de aceite**
- Falha de energia simulada produz instrução consistente de recuperação.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-06-10 — Criar testes end-to-end de reinstalação em VM

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `74` | **ClickUp:** [`1dfqjx6uw6w`](https://app.clickup.com/t/1dfqjx6uw6w)

**Objetivo e escopo**
Automatizar Windows 11 x64 com discos virtuais distintos, backup e validações pós-boot.

**Critérios de aceite**
- Instalação bem-sucedida com evidência do sistema resultante.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-07 — Restore Agent e migração de contas/perfis

- **ID ClickUp verificado:** [`1dfqjx6uw4g`](https://app.clickup.com/t/1dfqjx6uw4g) • status: `to do` em 09/11/2026

- **Fase:** `M5` • **Prioridade de importação:** Alta • **ID CSV do épico:** `75` • **Subtarefas:** 10

- **Status local:** `EM_ANDAMENTO` • **ClickUp:** `to do` verificado em 09/11/2026

- **Wave 2:** O agente independente restaurou arquivos em um perfil Windows descartável com mapeamento de pastas conhecidas e verificação de SHA-256. A migração mais ampla do perfil segue aberta. [Relatório da Wave 2](docs/qa/M4-M5-DEVELOPMENT-WAVE.md)

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Restaurar usuários e dados com confiabilidade, sem prometer clonagem de aplicativos e senhas.

**Subtarefas:**


#### AR-07-01 — Desenhar protocolo seguro entre Boot e Restore Agent

- [ ] **Concluída** | **Status:** `EM_ANDAMENTO` | **Prioridade:** Alta | **ID CSV:** `76` | **ClickUp:** [`1dfqjx6uw6x`](https://app.clickup.com/t/1dfqjx6uw6x)

**Objetivo e escopo**
Definir handoff de manifesto, estado, hashes e credenciais temporárias sem segredos em texto claro.

**Critérios de aceite**
- Agente recusa manifesto adulterado e identifica operação única.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-07-02 — Implementar agente temporário no Windows recém-instalado

- [ ] **Concluída** | **Status:** `EM_ANDAMENTO` | **Prioridade:** Alta | **ID CSV:** `77` | **ClickUp:** [`1dfqjx6uw6y`](https://app.clickup.com/t/1dfqjx6uw6y)

**Objetivo e escopo**
Criar serviço/tarefa de inicialização com permissões mínimas e auto-remoção após conclusão.

**Critérios de aceite**
- Execução pós-primeiro boot concluída e remoção confirmada.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-07-03 — Inventariar nomes de contas e perfis antigos

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `78` | **ClickUp:** [`1dfqjx6uw6z`](https://app.clickup.com/t/1dfqjx6uw6z)

**Objetivo e escopo**
Extrair SIDs, nomes, perfis locais, mapeamentos e colisões de nomes com autorização.

**Critérios de aceite**
- Relatório diferencia identidade local, Microsoft e domínio.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-07-04 — Recriar conta local quando selecionada

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `79` | **ClickUp:** [`1dfqjx6uw70`](https://app.clickup.com/t/1dfqjx6uw70)

**Objetivo e escopo**
Permitir recriar nome de conta sem copiar hashes de senha/segredos; política de senha inicial segura.

**Critérios de aceite**
- Conta criada com nome correto e requisitos de acesso claros.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-07-05 — Restaurar diretórios e permissões de usuários

- [ ] **Concluída** | **Status:** `EM_ANDAMENTO` | **Prioridade:** Alta | **ID CSV:** `80` | **ClickUp:** [`1dfqjx6uw72`](https://app.clickup.com/t/1dfqjx6uw72)

**Objetivo e escopo**
Mapear diretórios antigos para perfis novos, aplicar ACLs/SIDs de destino de forma controlada.

**Critérios de aceite**
- Usuário novo acessa arquivos; nenhum privilégio herdado indevido.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-07-06 — Integrar USMT quando aplicável

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `81` | **ClickUp:** [`1dfqjx6uw71`](https://app.clickup.com/t/1dfqjx6uw71)

**Objetivo e escopo**
Avaliar ScanState/LoadState e cenários offline, compatibilidade e licenciamento.

**Critérios de aceite**
- Migração testada em caso suportado; fallback documentado.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-07-07 — Restaurar configurações portáveis de aplicativos

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `82` | **ClickUp:** [`1dfqjx6uw73`](https://app.clickup.com/t/1dfqjx6uw73)

**Objetivo e escopo**
Migrar arquivos de configuração explicitamente selecionados com mapeamentos por aplicativo.

**Critérios de aceite**
- Configuração funciona em versões compatíveis; incompatibilidades registradas.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-07-08 — Criar reconciliação de conflitos de arquivos

- [ ] **Concluída** | **Status:** `EM_ANDAMENTO` | **Prioridade:** Alta | **ID CSV:** `83` | **ClickUp:** [`1dfqjx6uw74`](https://app.clickup.com/t/1dfqjx6uw74)

**Objetivo e escopo**
Opções substituir/manter ambos/ignorar, com preview e aplicação em lote.

**Critérios de aceite**
- Nenhum overwrite silencioso de dado existente.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-07-09 — Validar migração e produzir relatório de diferenças

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `84` | **ClickUp:** [`1dfqjx6uw75`](https://app.clickup.com/t/1dfqjx6uw75)

**Objetivo e escopo**
Comparar backup/restore e comunicar arquivos omitidos, bloqueados ou incompatíveis.

**Critérios de aceite**
- Relatório final contém contagem, bytes e exceções.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-07-10 — Assegurar limpeza de artefatos temporários

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `85` | **ClickUp:** [`1dfqjx6uw76`](https://app.clickup.com/t/1dfqjx6uw76)

**Objetivo e escopo**
Remover credenciais transitórias, tokens, tarefas agendadas e staging após operação.

**Critérios de aceite**
- Auditoria confirma que não restam segredos no novo Windows.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-08 — Drivers, aplicativos e perfis de instalação

- **ID ClickUp verificado:** [`1dfqjx6uw4h`](https://app.clickup.com/t/1dfqjx6uw4h) • status: `to do` em 09/11/2026

- **Fase:** `M5` • **Prioridade de importação:** Alta • **ID CSV do épico:** `86` • **Subtarefas:** 9

- **Status local:** `BACKLOG` • **ClickUp:** `to do` verificado em 09/11/2026

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Transformar a restauração em ambiente pronto para uso, respeitando compatibilidade e licenças.

**Subtarefas:**


#### AR-08-01 — Exportar inventário e pacotes de drivers antigos

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `87` | **ClickUp:** [`1dfqjx6uw77`](https://app.clickup.com/t/1dfqjx6uw77)

**Objetivo e escopo**
Detectar INF e versões; exportar via meios suportados quando possível e registrar origem.

**Critérios de aceite**
- Pacotes com hashes, arquitetura, hardware IDs e assinatura.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-08-02 — Filtrar e restaurar somente drivers compatíveis

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `88` | **ClickUp:** [`1dfqjx6uw78`](https://app.clickup.com/t/1dfqjx6uw78)

**Objetivo e escopo**
Confrontar Hardware IDs, arquitetura, assinatura e versão; não forçar drivers inadequados.

**Critérios de aceite**
- Instalação de storage/network correta nos testes; conflitos reportados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-08-03 — Criar catálogo declarativo de aplicativos

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `89` | **ClickUp:** [`1dfqjx6uw79`](https://app.clickup.com/t/1dfqjx6uw79)

**Objetivo e escopo**
Metadados de instalador, hash, licença, modo silencioso, requisitos, versão e pós-checagem.

**Critérios de aceite**
- Catálogo com Chrome e ferramentas básicas em ambientes autorizados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-08-04 — Implementar instalação de aplicativos pós-boot

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `90` | **ClickUp:** [`1dfqjx6uw7a`](https://app.clickup.com/t/1dfqjx6uw7a)

**Objetivo e escopo**
Usar WinGet quando disponível ou instaladores offline verificados com logs e retries.

**Critérios de aceite**
- Selecionados instalados; falhas isoladas não comprometem todo workflow.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-08-05 — Adicionar perfil ArchGTi de atendimento padrão

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `91` | **ClickUp:** [`1dfqjx6uw7b`](https://app.clickup.com/t/1dfqjx6uw7b)

**Objetivo e escopo**
Preset com programas autorizados, drivers e configuração inicial; sem instalar sem consentimento.

**Critérios de aceite**
- Um clique escolhe perfil; preview mostra todo software incluído.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-08-06 — Detectar programas anteriormente instalados

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `92` | **ClickUp:** [`1dfqjx6uw7c`](https://app.clickup.com/t/1dfqjx6uw7c)

**Objetivo e escopo**
Ler registro/inventário offline para recomendar reinstalação, sem copiar executáveis indiscriminadamente.

**Critérios de aceite**
- Catálogo de correspondência com confiança e alternativas.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-08-07 — Tratar aplicativos com licenciamento/termos específicos

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `93` | **ClickUp:** [`1dfqjx6uw7e`](https://app.clickup.com/t/1dfqjx6uw7e)

**Objetivo e escopo**
Marcar instaladores comerciais e exigir fonte/licença adequada, incluindo WinRAR e AnyDesk.

**Critérios de aceite**
- Nenhum pacote redistribuído sem direitos verificados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-08-08 — Adicionar validação pós-instalação de apps

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `94` | **ClickUp:** [`1dfqjx6uw7d`](https://app.clickup.com/t/1dfqjx6uw7d)

**Objetivo e escopo**
Testar versão instalada, executável, retorno de instalador e reboot pendente.

**Critérios de aceite**
- Relatório de sucesso/falha por programa e logs acessíveis.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-08-09 — Criar biblioteca offline de instaladores versionada

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `95` | **ClickUp:** [`1dfqjx6uw7f`](https://app.clickup.com/t/1dfqjx6uw7f)

**Objetivo e escopo**
Pacotes por arquitetura e versão, com manifesto de hashes e atualizações manuais.

**Critérios de aceite**
- Mídia opera sem internet com versões aprovadas.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-09 — Image Studio e ISOs personalizadas

- **ID ClickUp verificado:** [`1dfqjx6uw4j`](https://app.clickup.com/t/1dfqjx6uw4j) • status: `to do` em 09/11/2026

- **Fase:** `Studio` • **Prioridade de importação:** Alta • **ID CSV do épico:** `96` • **Subtarefas:** 10

- **Status local:** `BACKLOG` • **ClickUp:** `to do` verificado em 09/11/2026

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Permitir importação e personalização com integridade e rastreabilidade.

**Subtarefas:**


#### AR-09-01 — Implementar importação e validação de ISO

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `97` | **ClickUp:** [`1dfqjx6uw7g`](https://app.clickup.com/t/1dfqjx6uw7g)

**Objetivo e escopo**
Localizar imagens e metadados; calcular SHA-256; classificar oficial/personalizada/não verificável.

**Critérios de aceite**
- ISO alterada ou corrompida exibe risco sem presumir confiança.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-09-02 — Detectar índices, edições e componentes Windows

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `98` | **ClickUp:** [`1dfqjx6uw7h`](https://app.clickup.com/t/1dfqjx6uw7h)

**Objetivo e escopo**
Inspecionar WIM/ESD/SWM e relacionar arquitetura, build e pacote de idioma.

**Critérios de aceite**
- Usuário escolhe índice válido com preview.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-09-03 — Definir sandbox de trabalho e cópia imutável da ISO

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `99` | **ClickUp:** [`1dfqjx6uw7j`](https://app.clickup.com/t/1dfqjx6uw7j)

**Objetivo e escopo**
Nunca sobrescrever original; preparar scratch com espaço suficiente e limpeza transacional.

**Critérios de aceite**
- Falha no meio da montagem não destrói o arquivo original.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-09-04 — Implementar montagem, edição e commit de WIM

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `100` | **ClickUp:** [`1dfqjx6uw7k`](https://app.clickup.com/t/1dfqjx6uw7k)

**Objetivo e escopo**
Orquestrar DISM com logs, status, validação de mount state e descarte seguro.

**Critérios de aceite**
- Mudanças persistidas em cópia e rollback testado.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-09-05 — Injetar drivers certificados em imagens offline

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `101` | **ClickUp:** [`1dfqjx6uw7m`](https://app.clickup.com/t/1dfqjx6uw7m)

**Objetivo e escopo**
Separar drivers do boot.wim e install.wim; validar assinatura, compatibilidade e necessidade.

**Critérios de aceite**
- Imagem final contém pacotes esperados sem duplicatas indesejadas.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-09-06 — Integrar atualizações e idiomas suportados

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `102` | **ClickUp:** [`1dfqjx6uw7n`](https://app.clickup.com/t/1dfqjx6uw7n)

**Objetivo e escopo**
Aplicar pacotes compatíveis e validar ordem de servicing, build e espaço.

**Critérios de aceite**
- Imagem reinicia após pacote de teste aprovado.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-09-07 — Gerar unattend e plano de aplicativos pós-instalação

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `103` | **ClickUp:** [`1dfqjx6uw7p`](https://app.clickup.com/t/1dfqjx6uw7p)

**Objetivo e escopo**
Configurações declarativas e agentes; não executar MSI/EXE genericamente em WIM offline.

**Critérios de aceite**
- Arquivo de resposta validado e apps instalados no SO efetivo.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-09-08 — Reconstruir ISO bootável UEFI com manifesto

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `104` | **ClickUp:** [`1dfqjx6uw7q`](https://app.clickup.com/t/1dfqjx6uw7q)

**Objetivo e escopo**
Usar ferramentas de geração adequadas, hashes, assinaturas quando aplicável e relatório de diffs.

**Critérios de aceite**
- ISO nova inicializa em VM e mantém integridade do boot.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-09-09 — Validar limitações de ISOs modificadas por terceiros

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `105` | **ClickUp:** [`1dfqjx6uw7r`](https://app.clickup.com/t/1dfqjx6uw7r)

**Objetivo e escopo**
Checar componentes ausentes, scripts suspeitos, inconsistência e descontinuação segura.

**Critérios de aceite**
- Não executa scripts de terceiros na preparação sem aprovação.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-09-10 — Criar biblioteca de imagens com versões e rollback

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `106` | **ClickUp:** [`1dfqjx6uw7u`](https://app.clickup.com/t/1dfqjx6uw7u)

**Objetivo e escopo**
Indexar fontes, derivados, perfis, hashes e histórico de geração.

**Critérios de aceite**
- É possível rastrear exatamente a origem de qualquer ISO criada.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-10 — Creator: preparação e atualização de mídias USB

- **ID ClickUp verificado:** [`1dfqjx6uw4k`](https://app.clickup.com/t/1dfqjx6uw4k) • status: `to do` em 09/11/2026

- **Fase:** `Creator` • **Prioridade de importação:** Alta • **ID CSV do épico:** `107` • **Subtarefas:** 11

- **Status local:** `BACKLOG` • **ClickUp:** `to do` verificado em 09/11/2026

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Entregar experiência semelhante à preparação automatizada de mídia, mas específica do ArchRescue.

**Subtarefas:**


#### AR-10-01 — Definir arquitetura do aplicativo Creator para Windows

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `108` | **ClickUp:** [`1dfqjx6uw7t`](https://app.clickup.com/t/1dfqjx6uw7t)

**Objetivo e escopo**
Escolher GUI compatível, serviço privilegiado de gravação e compartilhamento de crates Rust.

**Critérios de aceite**
- ADR mostra limites de privilégio, UX e empacotamento.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-10-02 — Implementar descoberta de USBs e discos de destino

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `109` | **ClickUp:** [`1dfqjx6uw7v`](https://app.clickup.com/t/1dfqjx6uw7v)

**Objetivo e escopo**
Exibir modelo, serial, capacidade, tipo e removibilidade; nunca escolher automaticamente.

**Critérios de aceite**
- Discos do sistema protegidos e identificação física explícita.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-10-03 — Criar fluxo de confirmação de formatação segura

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `110` | **ClickUp:** [`1dfqjx6uw7w`](https://app.clickup.com/t/1dfqjx6uw7w)

**Objetivo e escopo**
Mostrar serial, tamanho, partições que serão apagadas e digit-confirm quando risco alto.

**Critérios de aceite**
- Tentativa de escolher disco interno bloqueada por padrão.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-10-04 — Implementar criação de partições UEFI boot + dados

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `111` | **ClickUp:** [`1dfqjx6uw7x`](https://app.clickup.com/t/1dfqjx6uw7x)

**Objetivo e escopo**
Preparar FAT32 EFI/boot e NTFS para ISOs quando adequado, com casos de dispositivo único.

**Critérios de aceite**
- Pendrive inicializa em UEFI e armazena arquivos maiores que 4GB.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-10-05 — Gravar ambiente ArchRescue e configurações de boot

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `112` | **ClickUp:** [`1dfqjx6uw7y`](https://app.clickup.com/t/1dfqjx6uw7y)

**Objetivo e escopo**
Copiar arquivos mínimos e configurar cadeia boot usando fontes validadas.

**Critérios de aceite**
- USB inicializa em hardware e VM com passthrough quando possível.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-10-06 — Criar seletor de perfil Essencial, Completo e Personalizado

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `113` | **ClickUp:** [`1dfqjx6uw7z`](https://app.clickup.com/t/1dfqjx6uw7z)

**Objetivo e escopo**
Modularizar ferramentas, catálogos, drivers e idioma conforme perfil.

**Critérios de aceite**
- Manifesto identifica conteúdo efetivo e uso estimado de espaço.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-10-07 — Implementar inclusão de ISOs Windows e instaladores

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `114` | **ClickUp:** [`1dfqjx6uw80`](https://app.clickup.com/t/1dfqjx6uw80)

**Objetivo e escopo**
Copiar fontes escolhidas para partição de dados, validar espaço e hashes.

**Critérios de aceite**
- Arquivos listados corretamente no Boot ISO após iniciar.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-10-08 — Adicionar atualização incremental de conteúdo da mídia

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `115` | **ClickUp:** [`1dfqjx6uw81`](https://app.clickup.com/t/1dfqjx6uw81)

**Objetivo e escopo**
Atualizar catálogos e dados sem formatar; atualizar boot por fluxo transacional próprio.

**Critérios de aceite**
- Interrupção preserva ambiente anterior ou informa recuperação.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-10-09 — Gerar ISO ArchRescue personalizada pelo Creator

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `116` | **ClickUp:** [`1dfqjx6uw82`](https://app.clickup.com/t/1dfqjx6uw82)

**Objetivo e escopo**
Reutilizar pipeline de empacotamento e perfis sem exigir pendrive físico.

**Critérios de aceite**
- Saída inicia em VM e apresenta manifesto de versão.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-10-10 — Verificar mídia gravada e emitir relatório

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `117` | **ClickUp:** [`1dfqjx6uw83`](https://app.clickup.com/t/1dfqjx6uw83)

**Objetivo e escopo**
Comparar manifestos/hashes, checar partição e arquivos críticos e apontar teste de boot recomendado.

**Critérios de aceite**
- Relatório explicita que verificação de arquivo não substitui boot real.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-10-11 — Empacotar Creator com instalação, atualização e assinatura

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `118` | **ClickUp:** [`1dfqjx6uw84`](https://app.clickup.com/t/1dfqjx6uw84)

**Objetivo e escopo**
Gerar MSI/EXE, minimizar privilégio, assinar builds de produção e fornecer uninstaller.

**Critérios de aceite**
- Instalação e remoção testadas em Windows suportado.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-11 — Ferramentas técnicas, rede e utilidades

- **ID ClickUp verificado:** [`1dfqjx6uw4m`](https://app.clickup.com/t/1dfqjx6uw4m) • status: `to do` em 09/11/2026

- **Fase:** `M3+` • **Prioridade de importação:** Normal • **ID CSV do épico:** `119` • **Subtarefas:** 8

- **Status local:** `BACKLOG` • **ClickUp:** `to do` verificado em 09/11/2026

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Concentrar utilidades práticas para atendimento e laboratório com risco controlado.

**Subtarefas:**


#### AR-11-01 — Criar terminal avançado com auditoria de comandos

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `120` | **ClickUp:** [`1dfqjx6uw85`](https://app.clickup.com/t/1dfqjx6uw85)

**Objetivo e escopo**
Acesso técnico a CMD/PowerShell conforme componentes disponíveis, distinguindo modo leitura/escrita.

**Critérios de aceite**
- Sessões registradas sem segredos e isoladas do assistente.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-11-02 — Implementar editor de registro offline com backup

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `121` | **ClickUp:** [`1dfqjx6uw86`](https://app.clickup.com/t/1dfqjx6uw86)

**Objetivo e escopo**
Selecionar hive, exportar antes de alterar, visualizar diffs e confirmação.

**Critérios de aceite**
- Teste de alteração/reversão em hive descartável.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-11-03 — Criar ferramentas de rede e SMB

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `122` | **ClickUp:** [`1dfqjx6uw87`](https://app.clickup.com/t/1dfqjx6uw87)

**Objetivo e escopo**
DHCP/manual, DNS, ping, teste de porta e montagem autenticada de share.

**Critérios de aceite**
- Backup em SMB com desconexão e reconexão tratadas.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-11-04 — Implementar transferência de backup via rede

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `123` | **ClickUp:** [`1dfqjx6uw88`](https://app.clickup.com/t/1dfqjx6uw88)

**Objetivo e escopo**
Suportar cópia para compartilhamento confiável com resume e checagem de espaço.

**Critérios de aceite**
- Backup íntegro após interrupção temporária do link.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-11-05 — Adicionar verificação de arquivos, hash e comparação

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `124` | **ClickUp:** [`1dfqjx6uw89`](https://app.clickup.com/t/1dfqjx6uw89)

**Objetivo e escopo**
Calcular SHA-256, comparar arquivos e gerar inventário exportável.

**Critérios de aceite**
- Hash idêntico/alterado demonstrado em testes.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-11-06 — Criar utilitário de partições com preview de alterações

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `125` | **ClickUp:** [`1dfqjx6uw8a`](https://app.clickup.com/t/1dfqjx6uw8a)

**Objetivo e escopo**
Oferecer ações suportadas com simulação, confirmação e limites em volumes críticos.

**Critérios de aceite**
- Sem comando executado antes de preview aceito.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-11-07 — Adicionar recuperação de arquivos deletados como plugin opcional

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `126` | **ClickUp:** [`1dfqjx6uw8b`](https://app.clickup.com/t/1dfqjx6uw8b)

**Objetivo e escopo**
Avaliar ferramenta de carving/forense segura, somente leitura, sem sobrescrever origem.

**Critérios de aceite**
- Fluxo obriga destino em outro disco e informa limitações.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-11-08 — Adicionar scanner de integridade de arquivos do sistema offline

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `127` | **ClickUp:** [`1dfqjx6uw8c`](https://app.clickup.com/t/1dfqjx6uw8c)

**Objetivo e escopo**
Agrupar verificação e evidências sem duplicar motor de reparo.

**Critérios de aceite**
- Resultado reutiliza checks existentes e exporta diagnóstico.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-12 — Segurança, integridade e proteção dos dados

- **ID ClickUp verificado:** [`1dfqjx6uw4n`](https://app.clickup.com/t/1dfqjx6uw4n) • status: `to do` em 09/11/2026

- **Fase:** `Transversal` • **Prioridade de importação:** Alta • **ID CSV do épico:** `128` • **Subtarefas:** 10

- **Status local:** `BACKLOG` • **ClickUp:** `to do` verificado em 09/11/2026

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Reduzir risco operacional e proteger dados de clientes durante todo o fluxo.

**Subtarefas:**


#### AR-12-01 — Criar guardas de autorização para operações críticas

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `129` | **ClickUp:** [`1dfqjx6uw8d`](https://app.clickup.com/t/1dfqjx6uw8d)

**Objetivo e escopo**
Toda ação de escrita consulta plano, alvo físico, consentimento e estado do backup.

**Critérios de aceite**
- Testes automatizados provam recusa sem confirmação.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-12-02 — Implementar política de disco degradado e leitura mínima

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `130` | **ClickUp:** [`1dfqjx6uw8e`](https://app.clickup.com/t/1dfqjx6uw8e)

**Objetivo e escopo**
Se SMART/I/O sugerem dano, interromper operações de reparo não essenciais e priorizar imagem/backup.

**Critérios de aceite**
- Teste com falhas injetadas ativa modo emergência.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-12-03 — Definir tratamento seguro de credenciais BitLocker/SMB

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `131` | **ClickUp:** [`1dfqjx6uw8f`](https://app.clickup.com/t/1dfqjx6uw8f)

**Objetivo e escopo**
Credenciais em memória pelo menor tempo possível, mascaramento e descarte; nunca no manifesto.

**Critérios de aceite**
- Busca automatizada não encontra segredos em logs/arquivos.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-12-04 — Criar verificação criptográfica de ferramentas e pacotes

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `132` | **ClickUp:** [`1dfqjx6uw8g`](https://app.clickup.com/t/1dfqjx6uw8g)

**Objetivo e escopo**
Manifestos assinados ou hashes pinados e checagem de origem.

**Critérios de aceite**
- Pacote modificado é rejeitado antes da execução.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-12-05 — Assinar artefatos e implementar supply-chain controls

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `133` | **ClickUp:** [`1dfqjx6uw8h`](https://app.clickup.com/t/1dfqjx6uw8h)

**Objetivo e escopo**
Assinatura de binários, SBOM, provenance, dependências fixadas e auditoria de supply chain.

**Critérios de aceite**
- Release contém hashes, SBOM e trilha de build.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-12-06 — Estabelecer isolamento de ISO e instaladores não confiáveis

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `134` | **ClickUp:** [`1dfqjx6uw8j`](https://app.clickup.com/t/1dfqjx6uw8j)

**Objetivo e escopo**
Inspecionar scripts/arquivos suspeitos, reduzir execução durante build e usar sandbox de teste.

**Critérios de aceite**
- Entradas desconhecidas nunca recebem privilégio silencioso.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-12-07 — Proteger temporários e trilhas de execução

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `135` | **ClickUp:** [`1dfqjx6uw8k`](https://app.clickup.com/t/1dfqjx6uw8k)

**Objetivo e escopo**
Volume temporário definido, permissões adequadas, limpeza segura e política para logs sensíveis.

**Critérios de aceite**
- Revisão demonstra ausência de artefatos de dados do cliente.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-12-08 — Implementar testes de fuzzing e corrupção de manifestos

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `136` | **ClickUp:** [`1dfqjx6uw8m`](https://app.clickup.com/t/1dfqjx6uw8m)

**Objetivo e escopo**
Testar parser de ISO/WIM/manifest, nomes de arquivos, symlinks/reparse points e estados truncados.

**Critérios de aceite**
- Inputs malformados não causam escrita fora do destino.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-12-09 — Criar política de consentimento, retenção e eliminação

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `137` | **ClickUp:** [`1dfqjx6uw8n`](https://app.clickup.com/t/1dfqjx6uw8n)

**Objetivo e escopo**
Informar o que é copiado, para onde, tempo e responsabilidade; permitir descartar dados auxiliares.

**Critérios de aceite**
- Checklist de privacidade integrado à UI e docs.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-12-10 — Revisar ameaças em cada release com gate de segurança

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `138` | **ClickUp:** [`1dfqjx6uw8p`](https://app.clickup.com/t/1dfqjx6uw8p)

**Objetivo e escopo**
Revisão obrigatória de riscos antes de publicar ISO e Creator.

**Critérios de aceite**
- Nenhum release sai com bloqueadores de integridade abertos.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-13 — Compatibilidade, licenças e distribuição

- **ID ClickUp verificado:** [`1dfqjx6uw4q`](https://app.clickup.com/t/1dfqjx6uw4q) • status: `to do` em 09/11/2026

- **Fase:** `Transversal` • **Prioridade de importação:** Alta • **ID CSV do épico:** `139` • **Subtarefas:** 7

- **Status local:** `BACKLOG` • **ClickUp:** `to do` verificado em 09/11/2026

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Garantir que o ArchRescue seja distribuível e juridicamente utilizável.

**Subtarefas:**


#### AR-13-01 — Verificar direitos de redistribuição do WinPE e ADK

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `140` | **ClickUp:** [`1dfqjx6uw8q`](https://app.clickup.com/t/1dfqjx6uw8q)

**Objetivo e escopo**
Avaliar documentação e termos aplicáveis ao uso comercial, distribuição da ISO e ferramentas incorporadas.

**Critérios de aceite**
- Parecer técnico-jurídico de go/no-go antes de release público.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-13-02 — Revisar licenciamento de instaladores incluídos

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `141` | **ClickUp:** [`1dfqjx6uw8r`](https://app.clickup.com/t/1dfqjx6uw8r)

**Objetivo e escopo**
Verificar permissões de distribuição e uso empresarial de AnyDesk, WinRAR, Chrome e alternativas.

**Critérios de aceite**
- Somente programas autorizados entram no catálogo distribuído.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-13-03 — Definir estratégia de obtenção de ISOs oficiais Windows

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `142` | **ClickUp:** [`1dfqjx6uw8t`](https://app.clickup.com/t/1dfqjx6uw8t)

**Objetivo e escopo**
Não empacotar imagens/licenças do Windows sem base adequada; permitir importação pelo cliente.

**Critérios de aceite**
- Fluxo orienta origem legítima e valida metadados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-13-04 — Elaborar termos de uso e avisos de risco de perda de dados

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `143` | **ClickUp:** [`1dfqjx6uw8u`](https://app.clickup.com/t/1dfqjx6uw8u)

**Objetivo e escopo**
Descrever limitações técnicas, responsabilidade, autorização de assistência e backup.

**Critérios de aceite**
- Textos claros revisados para uso operacional.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-13-05 — Definir política de suporte a Windows 10 e legado

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `144` | **ClickUp:** [`1dfqjx6uw8v`](https://app.clickup.com/t/1dfqjx6uw8v)

**Objetivo e escopo**
Distinguir recuperação de sistemas antigos de recomendação de instalação e suporte.

**Critérios de aceite**
- Matriz de suporte incorpora EOL, firmware e licenciamento.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-13-06 — Publicar checksum, assinatura e instruções de validação

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `145` | **ClickUp:** [`1dfqjx6uw8x`](https://app.clickup.com/t/1dfqjx6uw8x)

**Objetivo e escopo**
Disponibilizar SHA-256/assinatura e verificação do download para ISO/Creator.

**Critérios de aceite**
- Usuário consegue comprovar artefato íntegro.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-13-07 — Definir canais de distribuição e atualização

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `146` | **ClickUp:** [`1dfqjx6uw8w`](https://app.clickup.com/t/1dfqjx6uw8w)

**Objetivo e escopo**
Escolher releases assinados, versionamento, mirrors confiáveis e políticas de atualização offline.

**Critérios de aceite**
- Atualização valida autenticidade e oferece rollback seguro.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-14 — QA, laboratório e testes de regressão

- **ID ClickUp verificado:** [`1dfqjx6uw4p`](https://app.clickup.com/t/1dfqjx6uw4p) • status: `to do` em 09/11/2026

- **Fase:** `Transversal` • **Prioridade de importação:** Alta • **ID CSV do épico:** `147` • **Subtarefas:** 10

- **Status local:** `EM_ANDAMENTO` • **ClickUp:** `to do` verificado em 09/11/2026

- **Wave 2:** A regressão em Linux e Windows e a validação focada em WinPE estão registradas. O W10Val permanece abaixo da reserva de 15 GiB. [Relatório da Wave 2](docs/qa/M4-M5-DEVELOPMENT-WAVE.md)

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Não aprovar funcionalidades críticas apenas por compilarem: provar em cenários reproduzíveis.

**Subtarefas:**


#### AR-14-01 — Criar laboratório automatizado de VMs e snapshots

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `148` | **ClickUp:** [`1dfqjx6uw8y`](https://app.clickup.com/t/1dfqjx6uw8y)

**Objetivo e escopo**
Template de Windows 11 íntegro e danificado, discos múltiplos, dual boot e firmware UEFI.

**Critérios de aceite**
- Testes retornam estado inicial via snapshot.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-14-02 — Testar boot em Hyper-V, VirtualBox, VMware e QEMU

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `149` | **ClickUp:** [`1dfqjx6uw8z`](https://app.clickup.com/t/1dfqjx6uw8z)

**Objetivo e escopo**
Executar matriz real com drivers de armazenamento/rede e resolução de UI.

**Critérios de aceite**
- Resultados por plataforma, versão e Secure Boot documentados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-14-03 — Validar reparo EFI/BCD com cenários de falha

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `150` | **ClickUp:** [`1dfqjx6uw90`](https://app.clickup.com/t/1dfqjx6uw90)

**Objetivo e escopo**
Construir fixtures de BCD ausente, EFI quebrado, múltiplos sistemas e drive letters variadas.

**Critérios de aceite**
- Diagnósticos corretos e boot restaurado em cenários suportados.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-14-04 — Testar backup e restauração com muitos arquivos

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `151` | **ClickUp:** [`1dfqjx6uw91`](https://app.clickup.com/t/1dfqjx6uw91)

**Objetivo e escopo**
Unicode, caminhos longos, junctions, permissões, perfis múltiplos e arquivos grandes.

**Critérios de aceite**
- Hash, quantidade e metadados comparados antes/depois.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-14-05 — Testar degradação física simulada e falhas de I/O

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `152` | **ClickUp:** [`1dfqjx6uw92`](https://app.clickup.com/t/1dfqjx6uw92)

**Objetivo e escopo**
Injetar timeouts, read errors e pouco espaço para testar fail-safe.

**Critérios de aceite**
- Nenhuma operação destrutiva continua em condição bloqueadora.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-14-06 — Testar retomada após reinicialização e queda de energia

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `153` | **ClickUp:** [`1dfqjx6uw93`](https://app.clickup.com/t/1dfqjx6uw93)

**Objetivo e escopo**
Interromper backup/implantação/restore nas etapas e verificar idempotência.

**Critérios de aceite**
- Operação retoma ou falha de forma segura sem perda silenciosa.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-14-07 — Testar seleção de disco e prevenção de escrita no alvo errado

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `154` | **ClickUp:** [`1dfqjx6uw94`](https://app.clickup.com/t/1dfqjx6uw94)

**Objetivo e escopo**
Cenários com vários USBs, serial igual, drive letters alteradas e dispositivo removido.

**Critérios de aceite**
- Plano não executa se identidade física mudar.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-14-08 — Executar testes E2E Creator → USB → Boot → Recovery

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `155` | **ClickUp:** [`1dfqjx6uw95`](https://app.clickup.com/t/1dfqjx6uw95)

**Objetivo e escopo**
Criar USB com Creator, iniciar hardware/VM e executar check de descoberta, backup e restauração.

**Critérios de aceite**
- Relatório de evidências e logs rastreáveis.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-14-09 — Implantar CI, lint, testes unitários e artefatos assinados

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `156` | **ClickUp:** [`1dfqjx6uw96`](https://app.clickup.com/t/1dfqjx6uw96)

**Objetivo e escopo**
Cargo test/clippy/audit, builds replicáveis e releases com evidências.

**Critérios de aceite**
- Pipeline bloqueia falhas e expõe cobertura de testes.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-14-10 — Criar política de severidade de bugs e release gates

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Alta | **ID CSV:** `157` | **ClickUp:** [`1dfqjx6uw97`](https://app.clickup.com/t/1dfqjx6uw97)

**Objetivo e escopo**
Critérios P0/P1/P2, regressões, QA manual e checklist go/no-go.

**Critérios de aceite**
- Releases bloqueados por bugs de boot, perda de dados ou segurança.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-15 — Observabilidade, relatórios e documentação

- **ID ClickUp verificado:** [`1dfqjx6uw4r`](https://app.clickup.com/t/1dfqjx6uw4r) • status: `to do` em 09/11/2026

- **Fase:** `Transversal` • **Prioridade de importação:** Normal • **ID CSV do épico:** `158` • **Subtarefas:** 8

- **Status local:** `BACKLOG` • **ClickUp:** `to do` verificado em 09/11/2026

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Explicar tecnicamente o que foi feito e tornar o projeto sustentável.

**Subtarefas:**


#### AR-15-01 — Implementar logs estruturados por operação

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `159` | **ClickUp:** [`1dfqjx6uw99`](https://app.clickup.com/t/1dfqjx6uw99)

**Objetivo e escopo**
Correlation ID, timestamp, estágio, ação, saída, duração e redaction de segredos.

**Critérios de aceite**
- Rastreio completo sem dados sensíveis.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-15-02 — Criar relatórios técnicos HTML/PDF/JSON

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `160` | **ClickUp:** [`1dfqjx6uw98`](https://app.clickup.com/t/1dfqjx6uw98)

**Objetivo e escopo**
Listar diagnóstico, backups, operações executadas, falhas, softwares e validações.

**Critérios de aceite**
- Relatório legível para cliente e evidência detalhada para técnico.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-15-03 — Criar tela de histórico e auditoria

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `161` | **ClickUp:** [`1dfqjx6uw9a`](https://app.clickup.com/t/1dfqjx6uw9a)

**Objetivo e escopo**
Exibir operações por máquina/mídia, eventos e status, armazenados em local escolhido.

**Critérios de aceite**
- Filtros e exportação funcionais sem telemetria obrigatória.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-15-04 — Escrever manual de recuperação com exemplos

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `162` | **ClickUp:** [`1dfqjx6uw9b`](https://app.clickup.com/t/1dfqjx6uw9b)

**Objetivo e escopo**
Documentar reparo de boot, backup, ponto de restauração, imagem Windows e avisos.

**Critérios de aceite**
- Instruções completas para técnico sem conhecimento do código.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-15-05 — Escrever manual do Creator e Image Studio

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `163` | **ClickUp:** [`1dfqjx6uw9c`](https://app.clickup.com/t/1dfqjx6uw9c)

**Objetivo e escopo**
Tutorial de USB, ISO, atualização, drivers, perfis e troubleshooting.

**Critérios de aceite**
- Cada fluxo de UI tem guia e captura atualizada.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-15-06 — Preparar runbooks de falhas críticas

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `164` | **ClickUp:** [`1dfqjx6uw9d`](https://app.clickup.com/t/1dfqjx6uw9d)

**Objetivo e escopo**
Pendrive não inicia, BitLocker, disco ausente, backup falho, ISO incompatível, falha no primeiro boot.

**Critérios de aceite**
- Runbook contém diagnóstico, cautelas e retorno seguro.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-15-07 — Criar documentação para agentes de desenvolvimento

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `165` | **ClickUp:** [`1dfqjx6uw9e`](https://app.clickup.com/t/1dfqjx6uw9e)

**Objetivo e escopo**
Handoff de arquitetura, módulos, testes, comandos e limites de segurança.

**Critérios de aceite**
- Novos agentes conseguem localizar contratos e reproduzir build.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-15-08 — Criar plano de telemetria opcional e privacidade

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `166` | **ClickUp:** [`1dfqjx6uw9f`](https://app.clickup.com/t/1dfqjx6uw9f)

**Objetivo e escopo**
Dados agregados opt-in; evitar coleta de arquivos, contas, serial e informações pessoais.

**Critérios de aceite**
- Telemetria desativada por padrão e auditável.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




### AR-16 — Recursos avançados e integrações futuras

- **ID ClickUp verificado:** [`1dfqjx6uw4t`](https://app.clickup.com/t/1dfqjx6uw4t) • status: `to do` em 09/11/2026

- **Fase:** `Post-MVP` • **Prioridade de importação:** Baixa • **ID CSV do épico:** `167` • **Subtarefas:** 8

- **Status local:** `BACKLOG` • **ClickUp:** `to do` verificado em 09/11/2026

- [ ] **Épico concluído e validado** (marcar apenas quando subtarefas, critérios e evidências estiverem completos)

**Objetivo:** Reservar evoluções sem colocar o MVP em risco nem prometer capacidades não testadas.

**Subtarefas:**


#### AR-16-01 — Planejar módulo de migração de dados de sistemas comerciais

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `168` | **ClickUp:** [`1dfqjx6uw9g`](https://app.clickup.com/t/1dfqjx6uw9g)

**Objetivo e escopo**
Perfis por aplicativo para dados locais, bancos e configurações; evitar copiar bases em execução.

**Critérios de aceite**
- Plugin de exemplo e critérios de consistência de banco.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-16-02 — Avaliar imagem forense e recuperação de partições

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `169` | **ClickUp:** [`1dfqjx6uw9h`](https://app.clickup.com/t/1dfqjx6uw9h)

**Objetivo e escopo**
Selecionar integrações permitidas para imagem setorial e recovery com origem somente leitura.

**Critérios de aceite**
- PoC documenta eficácia, licenças e riscos.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-16-03 — Avaliar suporte ao modo BIOS/Legacy

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `170` | **ClickUp:** [`1dfqjx6uw9j`](https://app.clickup.com/t/1dfqjx6uw9j)

**Objetivo e escopo**
Verificar hardware-alvo, compatibilidade de boot, mídias e testes adicionais.

**Critérios de aceite**
- Decisão de suporte baseada em matriz de testes.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-16-04 — Criar catálogo extensível de playbooks locais

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `171` | **ClickUp:** [`1dfqjx6uw9k`](https://app.clickup.com/t/1dfqjx6uw9k)

**Objetivo e escopo**
Permitir adicionar regras assinadas, com versão, rollback e validação estática.

**Critérios de aceite**
- Pacotes desconhecidos recusados; catálogo pode ser desabilitado.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-16-05 — Planejar assistente IA opcional para interpretação de evidências

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `172` | **ClickUp:** [`1dfqjx6uw9m`](https://app.clickup.com/t/1dfqjx6uw9m)

**Objetivo e escopo**
Modelo local/remoto analisa logs redigidos e recomenda passos sem executar comandos automaticamente.

**Critérios de aceite**
- Sugestões auditáveis e aprovadas por motor determinístico.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-16-06 — Projetar recuperação remota e backups em rede avançados

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `173` | **ClickUp:** [`1dfqjx6uw9n`](https://app.clickup.com/t/1dfqjx6uw9n)

**Objetivo e escopo**
Avaliar SSH/SFTP/SMB e implantação centralizada, mantendo consentimento por máquina.

**Critérios de aceite**
- Prova de conceito separada do MVP.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-16-07 — Projetar catálogo de drivers por fornecedor

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `174` | **ClickUp:** [`1dfqjx6uw9p`](https://app.clickup.com/t/1dfqjx6uw9p)

**Objetivo e escopo**
Indexação de hardware IDs, modelos, versão e fontes oficiais verificadas.

**Critérios de aceite**
- Estratégia de atualização segura documentada.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.




#### AR-16-08 — Criar política de plugins terceiros e sandbox

- [ ] **Concluída** | **Status:** `BACKLOG` | **Prioridade:** Normal | **ID CSV:** `175` | **ClickUp:** [`1dfqjx6uw9q`](https://app.clickup.com/t/1dfqjx6uw9q)

**Objetivo e escopo**
API, manifests, permissões, assinatura e proibição de acesso irrestrito por padrão.

**Critérios de aceite**
- Proof-of-concept com sandbox e bloqueio de plugin adulterado.
- Registrar testes, logs/evidências e limitações de compatibilidade relevantes.

**Notas de implementação**
- Tratar falhas e pré-condições explicitamente.
- Reaproveitar contratos e componentes compartilhados do ArchRescue Core.



## 9. Políticas de versões e entregáveis

**Proposta:** versões semânticas (`MAJOR.MINOR.PATCH`) para Core, Boot ISO e Creator; artefatos identificados por commit, data, hash e matriz de compatibilidade. A versão do projeto e o nome de release deverão ser aprovados no ADR. Evitar numerar cronograma como se M0–M5 significassem lançamento prometido.

| Artefato | Conteúdo mínimo para publicação | Situação |
|---|---|---|
| Código-fonte público | Licença, README, build reproduzível, testes | Planejado |
| Boot ISO / kit de build | Só disponibilizar binário após avaliação de direitos WinPE | Bloqueado por revisão jurídica/técnica |
| Creator | Instalador assinado quando viável, operação segura, logs, hashes | Planejado |
| Restore Agent | Escopo limitado, mínimo privilégio, limpeza de segredos | Planejado |
| Image Studio | Originais intactos, build isolado, manifesto e reversibilidade | Planejado |
| Documentação | pt-BR primeiro, inglês pela comunidade, tutoriais e suporte | Planejado |

## 10. Registro de decisões arquiteturais e questões abertas

| ID | Assunto | Estado em 08/11/2026 | Próxima ação |
|---|---|---|---|
| D-001 | Nome ArchRescue | Definido na conversa | Padronizar marca e IDs |
| D-002 | Gratuito e open-source, origem brasileira | **Diretriz confirmada pelo idealizador** | Especificar política de licença e contribuição |
| D-003 | Duas distribuições: Boot ISO e Creator | Definido conceitualmente | Aprovar ADR e fronteiras |
| D-004 | Core em Rust | Proposta técnica preferencial | POC em WinPE e Windows |
| D-005 | WinPE como base inicial do Boot | Proposta, sujeito a direitos de redistribuição | Parecer/alternativas livres |
| D-006 | GPLv3 como licença inicial | Sugestão, **não escolhida definitivamente** | Audit de dependências e decisão formal |
| D-007 | pt-BR por padrão, i18n comunitário | Diretriz | Estratégia de traduções |
| D-008 | ISO externa e Windows customizado | Funcionalidade planejada | Definir matriz suportada e segurança |
| D-009 | Zero telemetria obrigatória | Diretriz | Política de privacidade |
| D-010 | MVP funcional exato | **ESCOPO DEFINIDO; ClickUp `planning`** | Executar P0 v0.0.1 e homologar antes da expansão |
| D-011 | Licenças de WinRAR/AnyDesk e terceiros | Não verificadas para redistribuição | Selecionar mecanismos legais de instalação |
| D-012 | Windows legado/BIOS | Fora da base inicial proposta | Avaliar por benchmark |
| D-013 | Vínculo de IDs nativos do ClickUp | **RECONCILIADO 09/11/2026** | Revalidar em atualizações futuras, evitando recriar tarefas |

## 11. Extensões sincronizadas no ClickUp (AR-17 / AR-18)

> Estes itens surgiram **depois** do CSV original de 175 tarefas. Foram criados e verificados no ClickUp em 09/11/2026. O cadastro significa **planejamento**; nenhuma tarefa foi implementada ou concluída.

### AR-17 — Open Source, Comunidade e Transparência (SINCRONIZADO)

**ClickUp:** [Épico AR-17](https://app.clickup.com/t/1dfqjx6uxmx) • `to do` • 8 subtarefas

- **Fase:** Transversal • **Status ClickUp:** `to do`, sincronizado em 09/11/2026 • **Motivo:** diretriz brasileira, gratuita e open-source confirmada após geração do CSV.
- [ ] **[AR-17-01 — Aprovar licença do código próprio](https://app.clickup.com/t/1dfqjx6uxmz)** — Verificar compatibilidade de dependências e possível adoção de GPLv3, Apache-2.0 ou alternativa; manter WinPE e binários de terceiros fora do escopo de licenciamento do código próprio. **Aceite:** ADR e arquivo LICENSE revisados.
- [ ] **[AR-17-02 — Definir modelo de distribuição legal e builds locais](https://app.clickup.com/t/1dfqjx6uxn0)** — Considerar Creator/kit de construção para componentes proprietários; impedir publicação não autorizada de WinPE/Windows. **Aceite:** fluxo legal documentado e reproduzível.
- [ ] **[AR-17-03 — Estruturar repositório público](https://app.clickup.com/t/1dfqjx6uxn1)** — README pt-BR/en, arquitetura, issues, milestones, changelog e instruções para build. **Aceite:** contribuinte consegue reproduzir uma build do código aberto.
- [ ] **[AR-17-04 — Publicar CONTRIBUTING, CODE_OF_CONDUCT e SECURITY](https://app.clickup.com/t/1dfqjx6uxn2)** — Templates de issue/PR, política de vulnerabilidades, releases e suporte. **Aceite:** ciclo comunitário testado com issue/PR exemplo.
- [ ] **[AR-17-05 — Definir governança de mantenedores e RFCs](https://app.clickup.com/t/1dfqjx6uxn3)** — Processo de decisões, revisões, compatibilidade, breaking changes e releases. **Aceite:** governança versionada no repositório.
- [ ] **[AR-17-06 — Criar fluxo comunitário de traduções](https://app.clickup.com/t/1dfqjx6uxn4)** — pt-BR padrão; inglês e novos idiomas; arquivos externos para i18n. **Aceite:** mudança de idioma sem recompilar Core; guia de contribuição.
- [ ] **[AR-17-07 — Publicar releases transparentes](https://app.clickup.com/t/1dfqjx6uxn5)** — Checksums, SBOM, provenance, CI verificável e documentação de compilação. **Aceite:** cada artefato público tem origem rastreável.
- [ ] **[AR-17-08 — Formalizar gratuidade, privacidade e roadmap público](https://app.clickup.com/t/1dfqjx6uxn6)** — Sem funções essenciais pagas; coleta opt-in; publicidade e marcas de terceiros controladas. **Aceite:** políticas públicas e consistentes com a UI.

### AR-18 — Benchmark Competitivo e Validação do Produto (SINCRONIZADO)

**ClickUp:** [Épico AR-18](https://app.clickup.com/t/1dfqjx6uxmy) • `to do` • 6 subtarefas

- **Fase:** M0/Transversal • **Status ClickUp:** `to do`, sincronizado em 09/11/2026 • **Motivo:** diferenciar o ArchRescue de projetos existentes sem alegar exclusividade sem prova.
- [ ] **[AR-18-01 — Construir matriz de concorrentes](https://app.clickup.com/t/1dfqjx6uxn7)** — Hiren's BootCD PE, MediCat USB, Sergei Strelec, Lazesoft, NTLite, Ventoy, Rescuezilla e produtos de migração. **Aceite:** capacidades, limites, custo e licença com fontes.
- [ ] **[AR-18-02 — Benchmark de falhas de boot](https://app.clickup.com/t/1dfqjx6uxn8)** — Reproduzir falhas de EFI/BCD e comparar reparos, passos e taxa de sucesso. **Aceite:** cenários e logs reproduzíveis.
- [ ] **[AR-18-03 — Benchmark de backup e reinstalação](https://app.clickup.com/t/1dfqjx6uxn9)** — Comparar seleção, verificação de dados, reinstalação e recuperação de perfis. **Aceite:** resultados com hashes e controle de integridade.
- [ ] **[AR-18-04 — Benchmark de usabilidade e tempo técnico](https://app.clickup.com/t/1dfqjx6uxna)** — Medir intervenção humana, número de cliques, tempo e erros em VM/hardware. **Aceite:** métricas e metodologia publicadas.
- [ ] **[AR-18-05 — Testar compatibilidade e falhas induzidas](https://app.clickup.com/t/1dfqjx6uxnb)** — Secure Boot, BitLocker autorizado, múltiplos discos, mídia removível e interrupções de energia simuladas. **Aceite:** matriz de compatibilidade e limites.
- [ ] **[AR-18-06 — Publicar análise de diferencial e riscos](https://app.clickup.com/t/1dfqjx6uxnc)** — Traduzir resultados em ajustes de escopo e RFCs, sem alegação de unicidade. **Aceite:** relatório revisado e prioridades atualizadas.

**Contagem geral verificada no ClickUp em 09/11/2026:** **175 itens originais + 16 itens AR-17/18 + 3 novas subtarefas de P0/QA/Preserve & Rebuild = 194 tarefas**, sendo **19 épicos e 175 subtarefas**. **193 `to do` e 1 `planning`**; nenhuma execução ou aprovação de QA foi inferida.

## 12. Matriz de riscos prioritários

| Risco | Probabilidade preliminar | Impacto | Mitigação e referência |
|---|---|---|---|
| Seleção errada de disco/formatação | Média | Crítico | DiskIdentity, confirmação forte, dry-run e teste AR-03/06/10/12/14 |
| Perda ou corrupção de backup | Média | Crítico | Manifesto, hash, checkpoint, restore test AR-05/12/14 |
| Disco fisicamente degradado | Alta em cenários de resgate | Alto | Avisos SMART, política de leitura, imagem AR-03/05/12 |
| WinPE não redistribuível na forma imaginada | A verificar | Crítico | Revisão de termos e alternativa de build local AR-13/17 |
| Dependências de interface incompatíveis com WinPE | Média | Alto | POC Boot Shell na primeira fase AR-01/02 |
| Secure Boot e variantes UEFI | Média | Alto | Testes com cadeias assinadas e laboratório AR-01/14 |
| ISO de terceiros com scripts ou malware | Média | Crítico | Validar origem/hash, isolamento e política AR-09/12 |
| Migração de apps ou licenças impossível | Alta em alguns cenários | Médio/Alto | Inventário e reinstalação condicionada AR-07/08/13 |
| Falso sucesso de reparo | Média | Alto | Evidências antes/depois, reboot e validação AR-04/14 |
| Divergência ClickUp ↔ Markdown | Média | Médio | IDs estáveis, diário de sincronização e revisão de conflitos |
| Preservação `Windows.old`/hard links confundida com backup externo | Média | Crítico | Bloquear qualquer formatação no modo preservador; teste de recuperação e aviso explícito AR-INPLACE-001 |
| Escritas concorrentes no volume durante Windows Setup | Média | Crítico | Não permitir em MVP; validar somente em imagens descartáveis e alternativas de volume separado |
| Prototipagem sem boot real ou QA falsamente positivo | Média | Alto | AR-P0-001 + AR-QA-P0, 10/10 evidências de execução e matriz `PASS/FAIL/BLOCKED` |

> A probabilidade é uma estimativa **qualitativa inicial**, não produto de análise estatística. Revisar formalmente no modelo de ameaças.

## 13. Registro de alterações e sincronização

### 13.1. Changelog do roadmap

| Data | Versão | Mudança | Origem | Sincronização |
|---|---|---|---|---|
| 2026-11-08 | 1.0.0 | Criação do espelho com 17 épicos e 158 subtarefas a partir do CSV de importação; inclusão de gates, riscos, decisões, open-source e benchmark propostos | Conversa + CSV gerado | **NÃO VALIDADA** via API |
| 2026-11-08 | 1.0.0 | Usuário informou que importou o CSV no ClickUp | Relato do usuário | Confirmado posteriormente via API |
| 2026-11-09 | 1.1.0 | API validou 175 itens originais; sincronizou AR-17/AR-18 e seus 14 filhos | ClickUp | **Concluído** |
| 2026-11-09 | 1.1.0 | MVP v0.1.0 definido; AR-00-02 em `planning`; criado P0 v0.0.1 com 10 capacidades | Decisão de produto + ClickUp | **Concluído para planejamento; implementação pendente** |
| 2026-11-09 | 1.1.0 | Criadas AR-P0-001, AR-QA-P0 e AR-INPLACE-001 com dependências | ClickUp | **Concluído para cadastro** |
| 2026-11-09 | 1.1.0 | Snapshot API: 194 itens; 19 épicos, 175 subtarefas; 193 `to do`, 1 `planning`; reconciliados 175 IDs originais | ClickUp + CSV | **Verificado** |
| 2026-11-11 | 1.1.0 | AR-02 a AR-07 e AR-14: BACKLOG → EM_ANDAMENTO no espelho local. Entrega parcial do planejamento, da restauração seletiva e da validação em VM | [Relatório da Wave 2](docs/qa/M4-M5-DEVELOPMENT-WAVE.md) | **PENDENTE**. Limite diário do conector |

### 13.2. Fila de sincronização pendente

- [x] **SYNC-001:** consultados 175 itens originais; 17 épicos e 158 subtarefas, com hierarquia e `to do` validados em 09/11/2026.
- [x] **SYNC-002:** IDs remotos das 175 tarefas originais reconciliados a partir do título único e ID CSV, sem confundir IDs artificiais do importador com IDs ClickUp.
- [x] **SYNC-003:** snapshot real de 194 itens e 193 `to do` / 1 `planning`; não foram alegados estados de execução.
- [x] **SYNC-004:** AR-17 e AR-18 cadastrados com 14 subtarefas e links reais; sem recriação duplicada.
- [x] **SYNC-005:** escopo MVP registrado em AR-00-02, três tarefas P0/QA/PoC criadas e suas dependências verificadas na criação.
- [ ] **SYNC-006:** vincular evidências reais de build, ISO, boot em VM e testes quando o agente executar P0.
- [ ] **SYNC-007:** conciliar status após o primeiro desenvolvimento e QA; não marcar `complete` apenas por geração de arquivos.
- [ ] **SYNC-008:** revisar licenciamento do WinPE/ADK e decidir o mecanismo de distribuição de binários antes do release.
- [ ] **SYNC-009:** reler estados no ClickUp e anexar commits e evidências da Wave 2 a AR-02, AR-03, AR-04, AR-05, AR-06, AR-07 e AR-14. Preservar AR-QA-M50 e o gate de redistribuição. [Relatório da Wave 2](docs/qa/M4-M5-DEVELOPMENT-WAVE.md)

### 13.3. Como atualizar offline (procedimento obrigatório)

1. Localize a tarefa pelo **código estável**, não apenas pelo texto do título.
2. Atualize `Status`, caixa `[ ]/[x]`, evidências, bloqueios e, se aplicável, a descrição e critérios.
3. Acrescente um evento nesta tabela de alteração com **data**, **ID**, **de → para**, **motivo**, **evidência**, **sincronizado?**. Nunca elimine registros de mudança.
4. Se surgir trabalho não previsto, crie um item `AR-NEW-####` em uma seção de propostas; só atribua o ID definitivo depois de conciliar o ClickUp.
5. Na reconexão, identifique itens correspondentes e atualize o ClickUp; depois anote o ID real e marque o evento como sincronizado. Divergência = inspeção, não overwrite automático.

#### Modelo para registrar atualização

```markdown
| AAAA-MM-DD | AR-05-06 | BACKLOG → EM_ANDAMENTO | Iniciada implementação do manifesto | PR #... / log... | NÃO |
```

#### Diário de mudanças operacionais (preencher daqui em diante)

| Data | ID | Estado anterior → novo | Motivo/decisão | Evidência | Sincronizado no ClickUp? |
|---|---|---|---|---|---|
| 2026-11-09 | AR-00-02 | `to do` → `planning` | Escopo P0 v0.0.1 / MVP v0.1.0 definido em decisão de produto | [Tarefa](https://app.clickup.com/t/1dfqjx6uw4w) | SIM |
| 2026-11-09 | AR-17 / AR-18 | proposta → `to do` | Dois épicos e 14 subtarefas incluídos no ClickUp | [AR-17](https://app.clickup.com/t/1dfqjx6uxmx) / [AR-18](https://app.clickup.com/t/1dfqjx6uxmy) | SIM |
| 2026-11-09 | AR-P0-001 / AR-QA-P0 / AR-INPLACE-001 | inexistente → `to do` | Tarefas e dependências criadas; implementação ainda não iniciada | [P0](https://app.clickup.com/t/1dfqjx6uxnh) / [QA](https://app.clickup.com/t/1dfqjx6uxnk) / [PoC](https://app.clickup.com/t/1dfqjx6uxnj) | SIM |

## 14. Tarefas operacionais adicionadas após a importação

Estas três subtarefas foram criadas em 09/11/2026 em épicos existentes, possuem prioridade e dependências registradas e permanecem `to do`.

### AR-P0-001 — Protótipo executável v0.0.1

- **ClickUp:** [1dfqjx6uxnh](https://app.clickup.com/t/1dfqjx6uxnh) • **Épico pai:** [AR-01](https://app.clickup.com/t/1dfqjx6uw4a) • **Status:** `to do` • **Prioridade:** Urgent.
- **Dependência registrada:** aguarda [AR-00-02 / MVP-001](https://app.clickup.com/t/1dfqjx6uw4w).
- **Escopo:** implementar as dez capacidades P0-F01 a P0-F10 com artefato executável, build reproduzível e funcionamento real em VM.
- **Aceite:** 10/10 capacidades verificadas, ações de origem somente leitura, backup seguro para destino distinto, hashes e logs, matrizes de teste positivas e negativas, e pelo menos duas VMs representativas com boot real.
- **Evidências:** `docs/evidence/P0-ENVIRONMENT-AUDIT.md`, `P0-BUILD-RESULTS.md`, `P0-BOOT-EVIDENCE.md`, `P0-STORAGE-SAFETY.md`, artefatos e SHA-256.

### AR-QA-P0 — Homologação independente do protótipo

- **ClickUp:** [1dfqjx6uxnk](https://app.clickup.com/t/1dfqjx6uxnk) • **Épico pai:** [AR-14](https://app.clickup.com/t/1dfqjx6uw4p) • **Status:** `to do` • **Prioridade:** Urgent.
- **Dependência registrada:** aguarda [AR-P0-001](https://app.clickup.com/t/1dfqjx6uxnh).
- **Escopo:** validação E2E de boot, 10 capacidades, diferenças entre VMs, origem preservada, backup de arquivos de teste, conflito de volume, falhas de disco/arquivos e cancelamento.
- **Veredito obrigatório:** `P0 ACCEPTED`, `P0 REJECTED` ou `P0 BLOCKED`; nenhuma evidência ausente pode ser tratada como PASS.

### AR-INPLACE-001 — PoC Preserve & Rebuild

- **ClickUp:** [1dfqjx6uxnj](https://app.clickup.com/t/1dfqjx6uxnj) • **Épico pai:** [AR-07](https://app.clickup.com/t/1dfqjx6uw4g) • **Status:** `to do` • **Prioridade:** High.
- **Dependência registrada:** aguarda [AR-00-02 / MVP-001](https://app.clickup.com/t/1dfqjx6uw4w).
- **Escopo:** comparar USMT hard links, migração no mesmo volume com Windows.old e backup convencional com restauração; analisar ganho real, espaço, ACL, perfil, apps e fallbacks.
- **Aceite:** benchmarks e hashes reais em VMs descartáveis; nunca tratar hard-link store como backup independente ou permitir formatação em um fluxo de preservação.

### Política do agente executor para a primeira iteração

- Ler o roadmap, `AGENTS.md`, status Git, arquitetura/ambiente antes de editar.
- Priorizar código executável e testes de verdade, não apenas UI estática ou documentação.
- Desenvolver o Core em Rust, UI pt-BR, documentos técnicos em inglês; WinPE somente via método de build e redistribuição legitimamente autorizado.
- Commits atômicos e documentação de QA; usar `.agent-tmp/` para scratch.
- Sem push, deploy, release pública ou escrita em disco físico sem autorização explícita.
- Jamais declarar P0 aprovado quando o ambiente Windows/ADK, boot em VM ou teste de backup estiverem bloqueados; registrar `BLOCKED` ou `NOT TESTED`.

---

## 15. Leituras e referências para implementação

- [Windows PE (Microsoft Learn)](https://learn.microsoft.com/windows-hardware/manufacture/desktop/winpe-intro)
- [Personalização de WinPE (Microsoft Learn)](https://learn.microsoft.com/windows-hardware/manufacture/desktop/winpe-mount-and-customize)
- [DISM Image Management](https://learn.microsoft.com/windows-hardware/manufacture/desktop/dism-image-management-command-line-options-s14)
- [Windows ADK](https://learn.microsoft.com/windows-hardware/get-started/adk-install)
- [ClickUp — importação de planilhas](https://help.clickup.com/hc/en-us/articles/6310834724247-Use-the-Spreadsheets-Importer)
- [Licenças aprovadas pela OSI](https://opensource.org/licenses)

> **Nota de responsabilidade:** estes links são referências técnicas. Termos de redistribuição, requisitos do ADK e compatibilidade de software devem ser verificados no momento da implementação e de cada release.

---

**Próximo passo deliberado:** o escopo está definido; executar **AR-P0-001 v0.0.1** e homologar **AR-QA-P0**. Preservar a distinção entre **planejado**, **cadastrado**, **implementado**, **testado** e **homologado**. Este documento permanece editável offline.

## 16. Engineering current-state addendum — 2026-11-10

P0 automated technical validation is complete for the recorded lab scope and accepted for continued development. The historical corrective report preserves the observations and the human review status at the time it was written. The owner's first comprehensive human review remains pending until actual product implementation reaches the 50% milestone. Public WinPE distribution remains gated by the separate licensing decision.

| Roadmap area | Evidence-backed state |
|---|---|
| AR-01 WinPE | A new local ISO was built from `ae04e7d`. QEMU/OVMF booted and ran the recovery workflow. VirtualBox booted the ISO with Secure Boot enabled. |
| AR-02 guided interface | Six stages, technician detail, real progress, checkpoint discovery and local reports implemented. The backup and resume flow ran in WinPE at 800×600 and 1280×720. Installation and restoration stages, full keyboard coverage and the new GUI cancellation walkthrough remain pending. |
| AR-03 inventory and diagnostics | PE architecture and kernel-resource evidence, WinRE file evidence, structured findings and same-disk GPT relationships implemented. An actual offline Windows image and synthetic negative cases were checked. Ambiguous ESP runtime and wider hardware coverage remain pending. |
| AR-04 controlled repair | Typed planning, exact target and command preview, explicit authorization, preservation requirements and laboratory execution guard implemented. The QEMU repair clone failed to boot before repair. Its ESP could be listed in WinPE, but safe canonicalization failed on both USB and SATA, so the verified EFI backup prerequisite blocked BCDBoot. |
| AR-05 backup | Durable versioned manifests and safe resume verified through a real WinPE interruption, restart, GUI continuation and independent SHA-256 readback of the backup and restored directory. Rich metadata and power-loss qualification remain open. |
| AR-14 and AR-15 QA and evidence | Targeted Linux, Windows and WinPE evidence and environment guardrails are recorded. W10Val measured 11.6 GiB free at final closure, below the 15 GiB floor for substantial builds. |

Track implementation against stable roadmap IDs and observable product acceptance criteria. For each entry record its exact source revision, user-facing workflow, acceptance fixture, result and open conditions. Count an entry toward product implementation when its scoped acceptance criteria have evidence. Keep infrastructure, QA, documentation, review and legal gates in the traceability matrix with their own states. Establish a stable weighted product denominator before reporting a global percentage. The existing roadmap and evidence are sufficient to report individual capabilities, and a global percentage has not been assigned.

The [development wave report](docs/qa/M1-M3-DEVELOPMENT-WAVE.md) contains the detailed evidence and ClickUp reconciliation. The [workspace incident report](docs/evidence/ENVIRONMENT-HYGIENE.md) contains the inventory, removals, verified retention and remaining capacity limit.

## 17. Wave 3 engineering update — 2026-11-12

| Roadmap area | Evidence-backed state |
|---|---|
| AR-02 recovery interface | The native dashboard, navigation, findings, disk inventory, backup, planning, handoff and reports follow the owner design. WinPE ran the final shell at revision `54bd125` with live disk and Windows data. The report records viewport and input coverage. |
| AR-03 diagnostics | Passive, versioned integrity inspection detected missing and truncated Windows files on the protected x64 clone. Servicing evidence separates current observations from historical CBS entries. Windows 11 qualification, active-loader attribution, full protected-file verification and broader fixture coverage remain open. |
| AR-04 / AR-05 recovery and backup | Findings link to preservation. The WinPE session verified an independent backup and rejected a same-source report export. EFI repair qualification remains open. |
| AR-06 / AR-07 deployment and restore | WinPE inspected a real WIM, exported a capacity-blocked plan and exported the verified backup handoff. The installation executor and deployment-to-restore journey remain open. |
| AR-14 / AR-QA-M50 | The report links runtime screenshots, versioned evidence, regression and storage results. The 50% human-review milestone retains its existing gate. |

See the [Wave 3 UI and Windows integrity report](docs/qa/WAVE-3-UI-WINDOWS-INTEGRITY.md) for the scoped acceptance matrix and ClickUp reconciliation.

**SYNC — 2026-11-11.** ClickUp descriptions for AR-02, AR-03, AR-04, AR-05, AR-06, AR-07 and AR-14 record the verified Wave 3 scope. A connector readback confirmed the notes and existing statuses. See [ClickUp reconciliation](artifacts/evidence/wave3/clickup-reconciliation.json).

## 18. Wave 4 engineering update — 2026-11-12

| Roadmap area | Evidence-backed state |
|---|---|
| AR-02 / AR-03 guided recovery | The WinPE shell shares a storage and recovery catalog. Its custom picker accepts typed resource selections. The WinPE fixture exercised a suggested Windows source, a protected disk, an independent folder and keyboard and pointer input. |
| AR-04 boot repair | The guided page uses same-disk GPT ESP candidates and an independent preservation destination. BCD association, the existing ESP path blocker and repair execution still require qualification. |
| AR-05 backup | Salvar meus arquivos suggested a profile Documents folder, copied 29 bytes to a second disk and verified the copy. The catalog rediscovered the operation after application restart and the shell reread the payload. |
| AR-06 / AR-07 deployment and restore | WinPE inspected a real WIM selected from the catalog and exported a verified restoration reference. Installation execution and guided account/folder mapping remain open. |
| AR-11 / AR-15 diagnostics and evidence | The integrated console ran a read-only network diagnostic and displayed its output and exit code. The guided report export wrote JSON and text. The emergency console remains available. |
| AR-14 / AR-QA-M50 | Linux and both Windows cross-target checks passed. WinPE validated the core workflows. Full installed-Windows regression and additional picker and EFI fixtures remain open. The human-review milestone stays at the existing evidence-backed 50% gate. |

Seven existing ClickUp descriptions were updated and verified by readback. See the [Wave 4 report](docs/qa/WAVE-4-INTELLIGENT-RECOVERY-UX.md) for the acceptance matrix, hashes and runtime limitations.

## 19. Wave 5 engineering update — 2026-11-13

| Roadmap area | Evidence-backed state |
|---|---|
| AR-03 / AR-06 deployment safety | A signed host registration and independent WinPE checks bound the reviewed plan to a fresh VirtualBox GPT disk. WinPE applied Windows 10 Pro x64, verified the GPT, EFI and BCD, and the disposable guest booted Windows 10 Pro 22H2 build 19045.2965. |
| AR-05 backup | ArchRescue preserved five representative files on a separate GPT/NTFS VDI. Independent host readback matched source and payload hashes and sizes. |
| AR-07 Restore Agent | The installed `RecoveryQA` account restored five files through the existing agent and four actual Known Folder mappings. Independent readback matched all three copies, including the Unicode path, and confirmed the conflicting file was preserved. |
| AR-02 GUI | The QA build includes a target-specific confirmation and backend status. A complete GUI execution walkthrough remains open. |
| AR-14 QA | The first full laboratory recovery passed. Linux, both Windows cross-target checks, and 71 native tests each in installed Windows and WinPE passed. The complete destructive negative runtime matrix remains open. |

The [Wave 5 report](docs/qa/WAVE-5-FIRST-COMPLETE-RECOVERY.md) records the
execution, boot, restore, negative cases and regression results. ClickUp
reconciliation is pending for existing AR-03, AR-05, AR-06, AR-07 and AR-14
tasks after the connector reached its daily limit and the browser required
sign-in. The evidence-backed 50% human-review milestone and separate WinPE/ADK
distribution gate remain in place.
