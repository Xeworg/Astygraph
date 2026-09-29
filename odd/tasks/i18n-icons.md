# i18n + Heroicons Implementation Tasks

## Task 3: Spanish/English localization and Heroicons outline assets

### 3.1 Create i18n module (src/i18n/mod.rs)
- [x] `Locale` enum: `En`, `Es`
- [x] Translation table with all UI strings and `I18n::t`/`I18n::t_args`
- [x] `I18n::new(locale)` constructor
- [x] Locale fallback: missing key returns key itself (no panic)
- [x] All required translation keys covered by tests

### 3.2 Translation storage
- [x] English and Spanish translation tables present for every required key
- [x] Storage is compiled Rust tables rather than TOML files, avoiding a runtime loader while preserving key coverage

### 3.3 Create icons module (src/icons/mod.rs)
- [x] `Icon` enum with all outline SVG icon variants
- [x] Inline SVG path data with Heroicons outline geometry
- [x] Heroicons MIT license notice preserved as doc comment
- [x] All essential UI icons: folder, document, link, arrow-up, folder-open, x-mark
- [x] Arc commands and negative coordinates handled by the path parser

### 3.4 Update AppState (src/app_state.rs)
- [x] Add `locale: Locale` field to `AppState`
- [x] Default to `Locale::En`
- [x] Add `AppState::with_locale(self, locale) -> Self`
- [x] Add `AppState::locale(&self) -> Locale`

### 3.5 Update app.rs
- [x] Import i18n and icons modules
- [x] Replace all hardcoded UI strings with `i18n.t("key")`
- [x] Replace text prefixes and action glyphs with SVG icon rendering
- [x] Add language toggle button (EN/ES switcher)
- [x] SVG rendering compatible with egui using `Shape::Path`

### 3.6 Add tests (tests/i18n_icons.rs)
- [x] Locale switching changes translations
- [x] Missing key returns key itself (fallback)
- [x] All keys present in both locales
- [x] Icon module loads without panic
- [x] All icon variants return valid SVG path data
- [x] AppState locale field transitions work correctly

### 3.7 MIT License notice
- [x] Add Heroicons MIT license text in `src/icons/heroicons_license.txt`

### 3.8 Validation
- [x] `cargo test --workspace --all-targets` — 91 passed
- [x] `cargo fmt --all -- --check`
- [x] `git diff --check`

### Evidence
- Independent verification found no blockers.
- Manual GUI smoke and Windows execution were not exercised.
- Work-unit commit: `b6cf59c feat: add localized heroicons desktop UI`.
