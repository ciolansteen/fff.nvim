# Plan de implementare: fff.el — Suport Emacs pentru fff.nvim

> **Status**: Draft tehnic v1.1  
> **Audiență**: Contribuitori Rust + Emacs Lisp  
> **Bază de cod analizată**: `dmtrKovalenko/fff.nvim` @ `2465c2ca`

---

## Rezumat executiv

fff.nvim are un backend Rust de înaltă calitate (`fff-core`) izolat de frontend-ul Neovim. Calea optimă pentru Emacs este un **Emacs dynamic module** implementat într-un nou crate Rust `crates/fff-emacs`, care folosește crate-ul [`emacs`](https://crates.io/crates/emacs) (echivalentul `mlua` din lumea Emacs). Deasupra acestuia se construiește un pachet Emacs Lisp `fff.el` care se integrează nativ cu ecosistemul `consult`/`vertico`/`embark`.

**MCP nu necesită nicio modificare** — `fff-mcp` funcționează deja ca subprocess și se conectează la orice client MCP din Emacs (`gptel`, `mcp.el`).

### Punct de pornire: JonasThowsen/fff.el

**⚠️ Nu pornim de la zero.** Există deja o implementare funcțională:
- **Repo**: https://github.com/JonasThowsen/fff.el
- **Ce face**: FFI direct la `libfff_c` + `consult` UI (`fff-find-file`, `fff-grep`, `fff-grep-fuzzy`), distribuit ca Nix flake, activ la 2026-04-15
- **Problema identificată**: struct offsets hardcodate (`offset 0, 8, 32, 104, 120`) — risc de mismatch silențios la fiecare upgrade upstream
- **Plan**: fork → fix struct offsets via `cbindgen` generat automat → PR înapoi la Jonas (codul e al lui, fix-ul îi aparține)
- **Etică**: menționăm explicit autorul în orice derivat, nu preluăm fără credit

---

## 0. Clarificare esențială: fff în Neovim NU se integrează în Telescope

> Această secțiune există pentru a evita o greșeală de design frecventă.

O presupunere naturală este că fff.nvim funcționează ca un backend pentru Telescope sau `vim.ui.select` — adică se integrează în tooling-ul de completion existent din Neovim. **Presupunerea e greșită.**

`picker_ui.lua` are **97KB** de cod — fff are propria interfață grafică completă: floating window, renderer, scrollbar, highlighting, preview panel, keybindings proprii. Este un picker standalone, ca Telescope însuși, nu un backend pentru Telescope.

```
Neovim: fff = UI custom 97KB  +  backend Rust (fff-core)
                 ↑
        construit de la zero,
        fără vim.ui.select, fără Telescope
```

### De ce Emacs e diferit — și mai bun în acest caz

Emacs are un contract standardizat: **`completing-read`**. Orice funcție care returnează candidați prin `completing-read` este interceptată automat de framework-ul de completion instalat de utilizator:

| Dacă utilizatorul are | Primește automat |
|---|---|
| `vertico` | UI vertical cu preview |
| `ivy` | UI ivy cu acțiuni |
| `helm` | UI helm cu secțiuni |
| `marginalia` | adnotări (frecency, git status) în coloana dreaptă |
| `embark` | acțiuni (open, copy-path, git-diff, grep-in-dir) via `C-;` |
| `consult` | source async cu narrowing și preview live |

**Concluzie**: pentru Emacs NU construim UI. Contribuția noastră este exclusiv backend-ul Rust (scoring Smith-Waterman, frecency LMDB, git status, query parser). UI-ul vine gratis din stack-ul deja instalat al utilizatorului.

Această abordare este idiomatică în Emacs (ex: `consult-fd` = `fd` subprocess + `completing-read`, zero UI propriu) și produce o integrare superioară celei din Neovim — utilizatorul nu trebuie să reînvețe keybindings, acțiunile embark funcționează imediat, marginalia afișează scorurile fff automat.

---

## 1. Decizie arhitecturală

### Opțiunile evaluate

| Opțiune | Pros | Cons | Verdict |
|---|---|---|---|
| **A: Dynamic module (`fff-emacs` crate)** | Performanță maximă, acces direct la `fff-core`, API identic cu Neovim | Necesită compilare, Emacs ≥27 | ✅ **RECOMANDAT** |
| **B: MCP subprocess** | Zero cod nou Rust, funcționează azi | Latență JSON-RPC, nu adaugă frecency în Emacs, nu e interactive-first | ❌ Insuficient pentru UX |
| **C: Elisp pur + `fd`/`rg`** | Portabilitate maximă | Reimplementare parțială, pierde scoring-ul Rust, duplicate de `consult-fd` | ❌ Nu valorifică investiția existentă |

### Decizia: Dynamic Module

**Justificare**: `fff-nvim/src/lib.rs` folosește `mlua` pentru exact același pattern — expune funcțiile Rust ca funcții Lua. Crate-ul `emacs` (de la ubolonton) oferă un API simetric: `#[defun]` vs `#[lua_function]`, `IntoLisp` vs `IntoLua`. Costul de implementare este similar cu `fff-nvim`, dar obținem acces complet la scoring-ul de frecency, git status, și motorul fuzzy Smith-Waterman — lucruri imposibil de reprodus în Elisp.

Precedente în ecosistemul Emacs: `vterm.so`, `tree-sitter`, `sqlite3`, `pdf-tools`, `emacs-libgit`.

**Strategie de distribuție**: Pre-built binaries per platformă (ca și Neovim) + compilare locală opțională. MELPA primește `fff.el`; `.so`-ul este descărcat sau compilat la `fff-setup`.

---

## 2. Diagramă de arhitectură

```
┌─────────────────────────────────────────────────────────────────┐
│                         EMACS PROCESS                           │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                      fff.el (Elisp)                      │   │
│  │                                                           │   │
│  │  fff-find-file    fff-grep    fff-recent    fff-setup     │   │
│  │       │               │           │             │         │   │
│  │  ┌────▼───────────────▼───────────▼─────────┐  │         │   │
│  │  │         fff-ui.el  (completing-read /     │  │         │   │
│  │  │          consult source / vertico)        │  │         │   │
│  │  └─────────────────────────────────────────-┘  │         │   │
│  │                       │                         │         │   │
│  │  ┌────────────────────▼─────────────────────┐  │         │   │
│  │  │         fff-emacs.so  (dynamic module)    │  │         │   │
│  │  │                                           │  │         │   │
│  │  │  (fff-emacs crate, Rust)                  │  │         │   │
│  │  │  • fff_create_instance                    │  │         │   │
│  │  │  • fff_search_files                       │  │         │   │
│  │  │  • fff_grep                               │  │         │   │
│  │  │  • fff_record_visit                       │  │         │   │
│  │  │  • fff_get_frecency_list                  │  │         │   │
│  │  │  • fff_destroy_instance                   │  │         │   │
│  │  └──────────────┬────────────────────────────┘  │         │   │
│  └─────────────────│────────────────────────────────┘         │   │
│                    │                                            │   │
└────────────────────│────────────────────────────────────────────┘
                     │ direct function calls (no IPC)
         ┌───────────▼─────────────────────────────┐
         │              fff-core  (Rust)             │
         │  FilePicker · FrecencyTracker (LMDB)      │
         │  QueryParser · SmithWaterman · Git2        │
         └─────────────────────────────────────────-┘

── MCP (separat, zero modificări) ──────────────────────────────────
  Emacs (gptel/mcp.el)  ←──stdio JSON-RPC──→  fff-mcp binary
```

---

## 3. Fișiere noi de creat

### Rust — `crates/fff-emacs/`

```
crates/fff-emacs/
├── Cargo.toml
└── src/
    ├── lib.rs          # punct de intrare: #[emacs::module], #[defun] exports
    ├── instance.rs     # gestionare SharedPicker + SharedFrecency per proiect
    ├── types.rs        # conversii IntoLisp: SearchResult → list of plists
    ├── search.rs       # fff_search_files, fff_search_async
    ├── grep.rs         # fff_grep, fff_grep_async
    ├── frecency.rs     # fff_record_visit, fff_get_frecency_list, fff_frecency_scores
    └── error.rs        # FffError → Emacs error signals
```

### Emacs Lisp — `lisp/`

```
lisp/
├── fff.el              # autoloads, setup, (require 'fff-emacs-module)
├── fff-ui.el           # completing-read wrapper + vertico annotations
├── fff-consult.el      # consult async source (fff-consult-find, fff-consult-grep)
├── fff-embark.el       # embark actions (open, copy-path, git-diff, grep-in-dir)
└── fff-mcp.el          # helper pentru configurare fff-mcp cu gptel/mcp.el
```

### Build & packaging

```
scripts/build-emacs.sh  # wrapper: cargo build --package fff-emacs --release
scripts/install-emacs.sh # descarcă prebuilt sau compilează
Makefile                 # target: emacs-module
```

---

## 4. Codul Rust care trebuie adăugat

### `crates/fff-emacs/Cargo.toml`

```toml
[package]
name = "fff-emacs"
version = "0.6.0"
edition = "2024"
description = "Emacs dynamic module for fff file finder"
license = "MIT"

[lib]
crate-type = ["cdylib"]

[features]
default = []
zlob = ["fff/zlob"]

[dependencies]
fff = { package = "fff-search", path = "../fff-core", version = "0.6.0",
        features = ["mimalloc-collect"] }
fff-query-parser = { path = "../fff-query-parser", version = "0.6.0" }
emacs = "0.18"          # emacs-module.h bindings de înaltă calitate
once_cell = { workspace = true }
parking_lot = { workspace = true }
rayon = { workspace = true }
dirs = { workspace = true }
git2 = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
mimalloc = { workspace = true }
```

### `crates/fff-emacs/src/lib.rs` (schelet)

```rust
use emacs::{Env, Result, Value, defun};

emacs::plugin_is_GPL_compatible!();

#[emacs::module(name = "fff-emacs")]
fn init(env: &Env) -> Result<()> {
    env.message("fff-emacs dynamic module loaded")?;
    Ok(())
}

/// Creează o instanță fff pentru directorul `root`.
/// Returnează un integer handle opac.
#[defun]
fn fff_create_instance(env: &Env, root: String) -> Result<Value<'_>> {
    let handle = instance::create(root)?;
    handle.into_lisp(env)
}

/// Caută fișiere. Returnează o listă de plist-uri Elisp.
/// Fiecare plist: (:path "..." :score N :git-status "M" :frecency N)
#[defun]
fn fff_search_files(
    env: &Env,
    handle: i64,
    query: String,
    limit: Option<i64>,
) -> Result<Value<'_>> {
    search::run(env, handle, &query, limit.unwrap_or(200))
}

/// Grep în directorul instanței. Returnează listă de plist-uri cu :path :line :col :content.
#[defun]
fn fff_grep(
    env: &Env,
    handle: i64,
    pattern: String,
    mode: Option<String>,  // "plain" | "regex" | "fuzzy"
) -> Result<Value<'_>> {
    grep::run(env, handle, &pattern, mode.as_deref().unwrap_or("fuzzy"))
}

/// Înregistrează o vizită la un fișier (actualizează frecency în SQLite/LMDB).
#[defun]
fn fff_record_visit(env: &Env, handle: i64, path: String) -> Result<Value<'_>> {
    frecency::record_visit(handle, &path)?;
    env.intern("t")
}

/// Returnează lista de fișiere sortate după frecency (pentru recentf-integration).
#[defun]
fn fff_get_frecency_list(env: &Env, handle: i64, limit: Option<i64>) -> Result<Value<'_>> {
    frecency::get_list(env, handle, limit.unwrap_or(50))
}

/// Eliberează resursele instanței.
#[defun]
fn fff_destroy_instance(_env: &Env, handle: i64) -> Result<()> {
    instance::destroy(handle)
}
```

### Ce NU se reutilizează din `fff-c`

`fff-c` (libfff.so) este un `cdylib` cu C ABI destinat limbajelor cu C FFI (Node, Bun, Python). Emacs dynamic modules nu sunt C FFI consumers — ele implementează `emacs_module_init` și primesc un `emacs_env*`. Prin urmare, creăm un crate separat care importă `fff-core` direct (ca și `fff-nvim`), fără intermediarul C. Aceasta elimină overhead-ul de conversie C→Rust și dă acces la tipurile Rust native.

### Ce se reutilizează din `fff-nvim`

Logica din `fff-nvim/src/`:
- `instance.rs` → adaptat: același pattern `Arc<Mutex<FilePicker>>` + `Arc<Mutex<FrecencyTracker>>`
- `path_shortening.rs` → copiat 1:1 (nu are dependențe Neovim)
- `error.rs` → adaptat: `IntoLuaResult` devine `IntoEmacsResult`

---

## 5. API public Emacs Lisp

### Funcții de bază (expuse de modulul dinamic via `#[defun]`)

```elisp
;; Lifecycle
(fff-emacs-create-instance ROOT-DIR)       → integer handle
(fff-emacs-destroy-instance HANDLE)        → nil
(fff-emacs-module-version)                 → string "0.6.0"

;; Search — returnează listă de plist-uri
(fff-emacs-search-files HANDLE QUERY &optional LIMIT)
;; → ((:path "src/main.rs" :display "src/main.rs" :score 142 
;;    :git-status "M" :frecency 0.87) ...)

;; Grep — returnează listă de plist-uri
(fff-emacs-grep HANDLE PATTERN &optional MODE)
;; → ((:path "src/lib.rs" :line 42 :col 8 :content "fn search(" 
;;    :ranges ((8 . 14))) ...)

;; Frecency
(fff-emacs-record-visit HANDLE PATH)       → t
(fff-emacs-frecency-list HANDLE &optional LIMIT)
;; → ((:path "..." :frecency 0.95 :last-visited "2025-01-15") ...)
```

### API high-level Elisp (`fff.el`)

```elisp
;; Entry points principale — cel mai probabil le vei binda la taste
(fff-find-file &optional DIR)              ; completing-read + vertico
(fff-grep &optional DIR)                   ; completing-read cu preview
(fff-recent-files)                         ; frecency list în completing-read
(fff-find-file-other-window &optional DIR)

;; Setup
(fff-setup)                                ; inițializează modulul, descarcă dacă lipsă
(fff-setup-keybindings)                    ; C-c f f, C-c f g, C-c f r

;; Consult sources (pentru (consult-buffer) extended)
fff-consult--source-files                  ; consult source async
fff-consult--source-grep                   ; consult source grep
```

### Integrare `embark`

```elisp
;; fff-embark.el definește:
(embark-define-keymap fff-embark-file-map
  "Actions pe candidații fff"
  ("o"  find-file                    "open")
  ("O"  find-file-other-window       "open other window")  
  ("w"  fff-embark-copy-path         "copy path")
  ("d"  fff-embark-dired             "dired la director")
  ("g"  fff-embark-grep-in-dir       "grep în directorul fișierului")
  ("D"  fff-embark-git-diff          "git diff")
  ("m"  magit-file-dispatch          "magit"))
```

---

## 6. Integrare cu ecosistemul Emacs

### `completing-read` (standard)

`fff.el` implementează un wrapper simplu:

```elisp
(defun fff-find-file (&optional dir)
  (interactive)
  (let* ((handle (fff--get-or-create-instance (or dir default-directory)))
         (candidates (fff--make-candidates handle ""))
         (chosen (completing-read "Find file: " 
                                  (fff--async-collection handle)
                                  nil nil nil 'fff-history)))
    (find-file (fff--candidate-path chosen))))
```

Funcționează cu orice frontend (Helm, Ivy, Ido, Icomplete) fără modificări suplimentare.

### `vertico` + `orderless`

```elisp
;; Vertico afișează automat lista — nu necesită cod special.
;; Adăugăm annotații cu marginalia:
(add-to-list 'marginalia-annotators-heavy
             '(fff-candidate . fff--marginalia-annotate))

;; fff--marginalia-annotate extrage din plist: git-status, frecency, dimensiune
```

### `consult` (async source — PRIORITATE)

`consult` are un sistem de async sources perfect pentru fff:

```elisp
;; fff-consult.el
(defvar fff-consult--source-files
  `(:name "fff Files"
    :narrow ?f
    :category fff-candidate
    :face consult-file
    :history file-name-history
    :state ,#'consult--file-state
    :new ,#'find-file
    :enabled ,(lambda () (fff--module-available-p))
    :items ,(lambda ()
              (fff--search-sync (fff--current-handle) "")))
  "Consult source pentru fișiere fff.")

(defun fff-consult-find (&optional dir)
  "Caută fișiere cu fff, folosind consult pentru UI async."
  (interactive)
  (let ((default-directory (or dir default-directory)))
    (consult--read
     (fff--consult-async-source (fff--current-handle))
     :prompt "fff: "
     :lookup #'consult--lookup-member
     :state (consult--file-preview)
     :category 'fff-candidate
     :sort nil)))  ; fff face deja sortarea prin scoring
```

Folosim `consult--async-pipeline` pentru a trimite query-ul la Rust la fiecare keystroke cu debounce, exact ca `consult-ripgrep`.

### `corfu` (completion în buffer)

Nu este relevant pentru file finding. Dacă fff-grep produce referințe de cod, pot fi integrate cu `xref` standard, nu cu `corfu`.

---

## 7. Frecency + memorie în Emacs

### Stocarea existentă

`fff-core` folosește **LMDB** (via crate-ul `heed`) pentru frecency — o bază de date embedded, zero-copy, extrem de rapidă. Baza de date este stocată în `~/.local/share/fff/` (respectă XDG).

**Aceasta este aceeași bază de date folosită de Neovim**. Dacă utilizatorul folosește ambele editoare, frecency-ul este **partajat automat** — fișierele vizitate în Neovim apar cu prioritate ridicată în Emacs și viceversa. Acesta este un avantaj major față de implementările native Emacs.

### Integrare cu `recentf`

```elisp
;; fff-frecency.el
(defun fff--sync-to-recentf ()
  "Sincronizează lista fff frecency cu recentf-list."
  (when (fff--module-available-p)
    (let ((fff-recent (fff-emacs-frecency-list (fff--global-handle) 100)))
      (dolist (item fff-recent)
        (recentf-add-file (plist-get item :path))))))

;; Hook pe save-buffer pentru a înregistra vizite
(defun fff--record-current-file ()
  (when (and buffer-file-name (fff--module-available-p))
    (fff-emacs-record-visit (fff--get-or-create-instance default-directory)
                             buffer-file-name)))

(add-hook 'find-file-hook #'fff--record-current-file)
(add-hook 'after-save-hook #'fff--record-current-file)
```

### Integrare cu `savehist`

Nu este necesară: LMDB persistă independent de `savehist`. Dacă utilizatorul vrea `savehist`-style portabilitate (export text), adăugăm o comandă `fff-export-frecency`.

---

## 8. MCP în Emacs

`fff-mcp` **nu necesită modificări**. Binar-ul existing funcționează cu orice client MCP:

### Cu `gptel` (recomandat pentru Claude/GPT)

```elisp
;; fff-mcp.el
(defun fff-setup-gptel-mcp ()
  "Configurează fff-mcp ca tool în gptel."
  (when (executable-find "fff-mcp")
    (require 'gptel-integrations nil t)
    ;; gptel suportă MCP via stdio subprocess
    (setq gptel-mcp-servers
          (append gptel-mcp-servers
                  `((:name "fff"
                     :command "fff-mcp"
                     :args ("--root" ,(expand-file-name default-directory))))))))
```

### Cu `mcp.el`

```elisp
(with-eval-after-load 'mcp
  (mcp-add-server "fff"
    :command "fff-mcp"
    :args (list "--root" default-directory)))
```

### Cu `ellama`

`ellama` folosește `llm` backend — poate fi extins cu tool calls. Furnizăm un helper `fff-ellama-tools` care definește funcțiile ca tool descriptors.

---

## 9. Plan de implementare

### Faza 1 — Fundație Rust (2-3 săptămâni) · Complexitate: **L**

| # | Task | Fișier | Complexitate |
|---|------|--------|--------------|
| 1 | Adaugă `crates/fff-emacs` în workspace | `Cargo.toml` | S |
| 2 | Implementează `instance.rs` — lifecycle management | `fff-emacs/src/instance.rs` | M |
| 3 | Implementează `types.rs` — conversii `SearchResult` → Elisp plist | `fff-emacs/src/types.rs` | M |
| 4 | Implementează `search.rs` — `fff_search_files` sync | `fff-emacs/src/search.rs` | M |
| 5 | Implementează `grep.rs` — `fff_grep` sync | `fff-emacs/src/grep.rs` | M |
| 6 | Implementează `frecency.rs` — record + list | `fff-emacs/src/frecency.rs` | S |
| 7 | `error.rs` — mapare erori Rust → Emacs error signals | `fff-emacs/src/error.rs` | S |
| 8 | `lib.rs` — `emacs_module_init`, toate `#[defun]` exports | `fff-emacs/src/lib.rs` | M |
| 9 | CI: build job pentru `fff-emacs.so` pe Linux/macOS | `.github/workflows/` | M |
| 10 | Makefile target: `make emacs-module` | `Makefile` | S |

### Faza 2 — Core Elisp (1-2 săptămâni) · Complexitate: **M**

| # | Task | Fișier | Complexitate |
|---|------|--------|--------------|
| 11 | `fff.el` — module loading, `fff-setup`, autoloads | `lisp/fff.el` | M |
| 12 | `fff-ui.el` — `completing-read` wrapper, candidate formatting | `lisp/fff-ui.el` | M |
| 13 | `fff-frecency.el` — `recentf` sync, find-file hooks | `lisp/fff-frecency.el` | S |
| 14 | `fff-mcp.el` — gptel/mcp.el setup helpers | `lisp/fff-mcp.el` | S |
| 15 | `install-emacs.sh` — descarcă prebuilt `.so` sau compilează | `scripts/install-emacs.sh` | M |

### Faza 3 — Integrări avansate (2-3 săptămâni) · Complexitate: **M-L**

| # | Task | Fișier | Complexitate |
|---|------|--------|--------------|
| 16 | `fff-consult.el` — async consult source cu pipeline | `lisp/fff-consult.el` | L |
| 17 | Async search în Rust (channel-based) pentru consult pipeline | `fff-emacs/src/search.rs` | L |
| 18 | `fff-embark.el` — keymap + actions | `lisp/fff-embark.el` | M |
| 19 | `marginalia` annotations pentru candidați fff | `lisp/fff-ui.el` | S |
| 20 | MELPA packaging: `Package-Requires`, `Version` header | `lisp/fff.el` | S |

### Faza 4 — Polish & distribution (1 săptămână) · Complexitate: **S-M**

| # | Task | Fișier | Complexitate |
|---|------|--------|--------------|
| 21 | README-EMACS.md cu instrucțiuni de instalare | `README-EMACS.md` | S |
| 22 | AUR PKGBUILD pentru Arch Linux | `packaging/PKGBUILD-emacs` | M |
| 23 | Nix flake overlay pentru `fff-emacs` | `flake.nix` | M |
| 24 | ERT tests pentru `fff.el` | `tests/fff-test.el` | M |

---

## 10. Ce NU facem (features Neovim fără sens în Emacs)

### ❌ `picker_ui.lua` — UI custom cu floating windows

fff.nvim implementează un picker UI complet (97KB de Lua) cu floating windows, scrollbar custom, treesitter highlighting în preview, și animații. **Emacs are deja UI frameworks mature** (`vertico`, `consult`, `helm`, `ivy`). Reimplementarea ar fi redundantă și ar crea competiție cu ecosistemul existent. Folosim `completing-read` standard + `consult` pentru async.

### ❌ `combo_renderer.lua` / `list_renderer.lua`

Renderingul custom al listelor este Neovim-specific (namespace highlights, extmarks). Emacs folosește text properties și overlay-uri — delegate complet `marginalia` și `consult`-preview.

### ❌ `treesitter_hl.lua` în preview

Preview-ul cu treesitter highlighting în floating window nu există în Emacs ca concept. `consult--file-preview` cu `font-lock` oferă highlighting nativ, fără niciun cod special.

### ❌ `hex_dump.rs`

Hex dump pentru fișiere binare în preview este o funcționalitate de nișă Neovim. Nu are un corespondent natural în Emacs (există pachete dedicate ca `nhexl-mode`).

### ❌ `fff-nvim` Neovim-specific bindings

Funcții ca `set_current_buf`, `nvim_open_win`, `vim.schedule` nu există în Emacs. Le omitem complet — `fff-emacs` nu va depinde de `fff-nvim`.

### ⚠️ `download.lua` (adaptat, nu portat)

Logica de download prebuilt binaries este necesară dar se rescrie în shell/Elisp — nu portăm Lua.

---

## 11. Considerente pentru packaging

### MELPA

`fff.el` va fi un pachet MELPA standard cu:

```elisp
;; lisp/fff.el header
;; Package-Version: 0.6.0
;; Package-Requires: ((emacs "27.1") (consult "1.0") (vertico "1.0")
;;                    (marginalia "1.0") (embark "1.0"))
;; URL: https://github.com/dmtrKovalenko/fff.nvim
```

**Problema modulului dinamic**: MELPA nu poate distribui binare `.so`. Soluții:
1. `fff-setup` descarcă binar-ul pre-compilat din GitHub Releases (ca `pdf-tools`)
2. `fff-build-module` compilează local cu `cargo` (ca `vterm`)
3. Ambele: setup detectează dacă `cargo` este disponibil și alege

### Arch Linux (AUR)

```
# PKGBUILD
pkgname=emacs-fff
pkgver=0.6.0
depends=('emacs>=27' 'fff-nvim')  # reutilizează LMDB database
makedepends=('rust' 'cargo')
source=("https://github.com/dmtrKovalenko/fff.nvim/archive/v${pkgver}.tar.gz")

build() {
    cargo build --package fff-emacs --release --features zlob
}

package() {
    install -Dm755 target/release/libfff_emacs.so \
        "${pkgdir}/usr/share/emacs/site-lisp/fff/fff-emacs.so"
    install -Dm644 lisp/*.el \
        "${pkgdir}/usr/share/emacs/site-lisp/fff/"
}
```

### Nix / NixOS

Se adaugă în `flake.nix` existent ca output suplimentar:

```nix
packages.fff-emacs = pkgs.rustPlatform.buildRustPackage {
  pname = "fff-emacs";
  inherit version src cargoLock;
  buildPhase = ''cargo build --package fff-emacs --release'';
  installPhase = ''
    install -Dm755 target/release/libfff_emacs.so \
      $out/share/emacs/site-lisp/fff/fff-emacs.so
    install -Dm644 lisp/*.el $out/share/emacs/site-lisp/fff/
  '';
};
```

### Windows

Emacs dinamic modules pe Windows necesită `.dll` în loc de `.so`. Cargo produce `fff_emacs.dll` automat pe Windows. Necesită testing separat — **out of scope pentru v1**.

---

## 12. Detalii tehnice critice

### Numele fișierului `.so`

Emacs `module-load` caută `fff-emacs.so` (cu crateful-name cu liniuțe). Cargo produce `libfff_emacs.so` (cu underscore). Soluție în `Makefile`:

```makefile
emacs-module:
    cargo build --package fff-emacs --release
    cp target/release/libfff_emacs.so lisp/fff-emacs.so
```

### Thread safety

`emacs` crate impune că funcțiile `#[defun]` rulează pe thread-ul Emacs. Căutările intensive le delegăm lui `rayon` threadpool (ca în `fff-nvim`), iar rezultatele le returnăm sincron. Pentru async real (faza 3), folosim `emacs::CallEnv` + channels, similar cu cum `consult` gestionează procese async.

### Memory management

Instanțele `FilePicker` sunt stocate într-un `HashMap<i64, Arc<Mutex<FilePicker>>>` global (protejat cu `parking_lot::RwLock`). Handle-urile sunt i64 pentru a fi compatibile cu tipul integer Emacs. `fff_destroy_instance` scoate din map și droppează Arc-ul.

### Versioning compatibilitate

`emacs` crate funcționează cu Emacs ≥ 25.1 la nivel de API, dar recomandăm ≥ 27.1 pentru stabilitate. `emacs_module_init` este simbolul căutat de `module-load` — `emacs` crate îl exportă automat via macro.

---

## Referințe

- [emacs Rust crate](https://github.com/ubolonton/emacs-module-rs) — API de referință
- [Emacs Module API](https://www.gnu.org/software/emacs/manual/html_node/elisp/Writing-Dynamic-Modules.html)
- [consult async sources](https://github.com/minad/consult#asynchronous-search) — pattern pentru fff-consult
- [vterm](https://github.com/akermu/emacs-libvterm) — exemplu de packaging dynamic module cu MELPA
- [tree-sitter.el](https://github.com/emacs-tree-sitter/elisp-tree-sitter) — exemplu de download prebuilt binaries
- `crates/fff-nvim/src/lib.rs` — model direct pentru fff-emacs (pattern identic cu mlua)
