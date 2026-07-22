# Modo dibujo (Excalidraw) — Modal + inline preview

## Decisiones de diseño (mejor criterio, dado que no hubo respuesta)

1. **Librería**: **Excalidraw** (React). Es la que pediste originalmente al elegir el item diferido. Añade `react` + `react-dom` + `@vitejs/plugin-react`, pero se carga **lazy** (solo cuando se abre el modal) — no afecta el startup.
2. **Forma UX**: **Modal full-screen + inline preview**. Razones:
   - El codebase evita a propósito la API de plugins de Milkdown (cero nodeViews custom, comentario explícito en `commands.ts:63-67`). "Bloques embebidos inline" rompería esa filosofía y añadiría mucho código ProseMirror.
   - El patrón `Dialog` ya existe (lo usa `CommandPalette`).
   - Coexiste con texto, soporta múltiples dibujos por nota, y **no toca el schema de Milkdown**.
   - Aprovecha la convención `attachments/` **que ya existe pero no se usa** (`wikilinks.rs:32-56`) y la tabla `note_attachments` vacía (`migrations/0002_notes_initial.sql:29-34`) — fue diseñada para esto.

## Flujo de usuario

1. Botón **PenTool** en toolbar → abre modal Excalidraw vacío.
2. Usuario dibuja → clic **Save**.
3. Backend escribe 2 archivos en `notes/attachments/{note_id}/{drawing-id}.svg` + `.excalidraw` (estado editable).
4. Se inserta markdown `![](attachments/{note_id}/{drawing-id}.svg)` en el cursor. El SVG se renderiza como imagen común gracias al commonmark `image` node.
5. Click sobre la imagen → modal se reabre en modo edición, lee el `.excalidraw` para restaurar el estado. Al guardar, sobreescribe ambos archivos (mismo id).

---

## Parte 1 — Backend Rust: pipeline de attachments

**Archivo nuevo**: `src-tauri/src/notes/attachments.rs`
- `save(note_id: &str, drawing_id: &str, svg: &[u8], state: &[u8]) -> AppResult<String>` — escribe a `notes_dir/attachments/{note_id}/{drawing-id}.{svg,excalidraw}`, crea `attachments/` con `create_dir_all`. Devuelve ruta relativa.
- `read_svg(rel_path: &str) -> AppResult<Vec<u8>>` y `read_state(rel_path: &str) -> AppResult<Vec<u8>>`.
- `delete(rel_path: &str) -> AppResult<()>`.
- **Path traversal safety**: toda función valida que el path canónico resuelva dentro de `notes_dir/attachments/`.

**Comandos nuevos** en `src-tauri/src/notes/mod.rs` (siguiendo el estilo sync + `AppResult<T>`):
```rust
#[tauri::command]
pub fn save_drawing(note_id: String, drawing_id: String, svg: Vec<u8>, state: Vec<u8>, state: State<'_, AppState>) -> AppResult<String>

#[tauri::command]
pub fn read_drawing(path: String, state: State<'_, AppState>) -> AppResult<Vec<u8>>  // SVG

#[tauri::command]
pub fn read_drawing_state(path: String, state: State<'_, AppState>) -> AppResult<Vec<u8>>  // .excalidraw JSON

#[tauri::command]
pub fn delete_drawing(path: String, state: State<'_, AppState>) -> AppResult<()>
```

**Registro**:
- 4 entradas en `src-tauri/src/lib.rs:97-148` `generate_handler!`.
- 4 archivos `src-tauri/permissions/allow-{save-drawing,read-drawing,read-drawing-state,delete-drawing}.toml` (mirror de `allow-sync-export-zip.toml`).
- 4 entradas en `src-tauri/capabilities/default.json`.

**Sync zip**: `notes/sync.rs:walk_md` solo recoge `.md`, así que los assets en `attachments/` se ignoran en el index — perfecto. Pero el `export_zip` (`sync.rs:17-45`) usa `strip_prefix(notes_dir)` y recorre todo; revisar si hay que añadirlo explícitamente al walker de export (probablemente sí, para que los dibujos sobrevivan export/import).

**Tests Rust** (en `attachments.rs`): `write_then_read_roundtrip`, `path_traversal_rejected`, `delete_removes_both_files`.

## Parte 2 — Frontend: deps + Vite

**package.json** (nuevas):
- `react`, `react-dom` (runtime)
- `@vitejs/plugin-react` (dev)
- `@excalidraw/excalidraw` (runtime, ~1MB pero lazy)

**`vite.config.js`**: añadir `react()` al array de plugins. SvelteKit + React coexisten — el plugin React solo procesa `.jsx`/`.tsx`, Svelte sigue procesando `.svelte`. SPA mode (`adapter-static` con fallback), no hay SSR así que no hay conflicto.

## Parte 3 — Wrapper React de Excalidraw

**Archivo nuevo**: `src/lib/components/drawing/ExcalidrawReact.tsx`
- Componente React puro con `<Excalidraw initialData={...} onChange={...} UIOptions={...} />`.
- Recibe `initialElements` (del `.excalidraw`) y callbacks `onSave(elements, appState)`.
- Botones custom de Save/Cancel exportados via `renderFooter` (Excalidraw soporta UI customization).
- Expone `exportToSvg()` vía la API oficial de Excalidraw.

## Parte 4 — Modal Svelte que monta React

**Archivo nuevo**: `src/lib/components/drawing/ExcalidrawModal.svelte`
- Patrón **idéntico a `Milkdown.svelte`**: `bind:this={host}` + `onMount` crea React root con `createRoot(host).render(<ExcalidrawReact .../>)` + `onDestroy` llama `root.unmount()`.
- Props: `open` ($bindable), `noteId`, `drawingId` (null = nuevo), `onSaved(svgPath)`, `onClose`.
- Shell visual: `<Dialog.Root open>` + `<Dialog.Content class="drawing-modal">` full-screen (96vw × 92vh).
- Cuando se abre con `drawingId != null`: llama `readDrawingState` para cargar el `.excalidraw` en el estado inicial.
- Al guardar: llama `exportToSvg()` → bytes → `saveDrawing(noteId, drawingId, svgBytes, stateJson)` → `onSaved(relPath)`.

## Parte 5 — EditorToolbar + NoteCanvas wiring

**`EditorToolbar.svelte`**:
- Nuevo botón PenTool (lucide `PenTool` icon) en la sección de block commands, antes del overflow dropdown.
- Props nuevas: `onOpenDrawing: () => void` (modo nuevo).
- Sigue el patrón `Tooltip.Root > Tooltip.Trigger > Button` existente.

**`NoteCanvas.svelte`**:
- Importa `ExcalidrawModal` con import dinámico para lazy-load: `const ExcalidrawModal = (await import("./drawing/ExcalidrawModal.svelte")).default` (cuando se abre por primera vez).
- Estado nuevo: `drawingModalOpen`, `editingDrawingId` (string | null).
- Click handler en `.editor-host` (event delegation): si `event.target` es `<img>` con `src` que matchea `attachments/{note_id}/{id}.svg`, extrae `{id}` y abre modal en modo edición.
- `onSaved(svgPath)`:
  - Si era dibujo nuevo: `editorHandle?.insertTextAtCursor("\n\n![](" + svgPath + ")\n\n")`.
  - Si era edición: la imagen ya está en el body, solo actualiza el markdown para forzar re-render del ProseMirror (setMarkdown con mismo content, o noop si el src no cambió).

## Parte 6 — IPC + i18n

**`src/lib/ipc.ts`**: 4 wrappers nuevos — `saveDrawing`, `readDrawing`, `readDrawingState`, `deleteDrawing`. Mismo patrón `safeInvoke` + `InvokeResult<T>` que el resto.

**`messages/en.json` + `messages/es.json`** (~12 claves nuevas):
- `drawing_toolbar_label`, `drawing_modal_title_new`, `drawing_modal_title_edit`, `drawing_save`, `drawing_cancel`, `drawing_empty_hint`, `drawing_export_svg`, `drawing_error_save`, `drawing_error_load`, `drawing_confirm_delete`, `settings_drawings_section` (para futuro panel de gestión), `drawing_aria_modal`.

## Parte 7 — Verificación

- `pnpm svelte-check` → 0 errores.
- `pnpm test` → todos los tests pasan (actuales + 3 nuevos Rust).
- `cargo test --manifest-path src-tauri/Cargo.toml notes::attachments` → 4 pasan.
- `cargo clippy` → sin warnings nuevos.
- Build de producción exitoso (`pnpm tauri build` o al menos `pnpm build`): confirma que React + SvelteKit coexisten en el bundle.
- **Prueba manual** (te la dejo a ti):
  1. `pnpm tauri dev`
  2. Abre una nota → toolbar → botón PenTool → dibuja → Save.
  3. Verifica que el SVG aparece inline en el documento.
  4. Click sobre el SVG → modal reabre con el dibujo cargado.
  5. Verifica en disco: `notes/attachments/{note-id}/{drawing-id}.svg` + `.excalidraw`.

---

## Riesgos y mitigaciones

| Riesgo | Mitigación |
|---|---|
| Excalidraw añade ~1MB al bundle | Import dinámico — solo carga cuando se abre el modal |
| React + Svelte coexistencia | `@vitejs/plugin-react` solo procesa `.tsx`; Svelte sigue con `.svelte`. SPA mode = sin SSR conflict |
| Path traversal en backend | Validación canónica dentro de `notes_dir/attachments/` en cada comando |
| `extract_attachments` ya scanea el body | Ya filtra por prefijo `attachments/` — nuestros paths matchean, así que se trackean automáticamente en `frontmatter.references` (bonus) |
| CSP bloquea canvas/blob | `tauri.conf.json:27-29` tiene `csp: null` — sin bloqueo |

## Archivos a tocar

**Nuevos (6)**:
- `src-tauri/src/notes/attachments.rs`
- `src-tauri/permissions/allow-save-drawing.toml`
- `src-tauri/permissions/allow-read-drawing.toml`
- `src-tauri/permissions/allow-read-drawing-state.toml`
- `src-tauri/permissions/allow-delete-drawing.toml`
- `src/lib/components/drawing/ExcalidrawReact.tsx`
- `src/lib/components/drawing/ExcalidrawModal.svelte`

**Modificados (7)**:
- `package.json` (deps)
- `vite.config.js` (plugin react)
- `src-tauri/src/lib.rs` (registro handlers)
- `src-tauri/src/notes/mod.rs` (comandos)
- `src-tauri/capabilities/default.json` (permisos)
- `src/lib/ipc.ts` (wrappers)
- `src/lib/components/EditorToolbar.svelte` (botón)
- `src/lib/components/NoteCanvas.svelte` (modal + click handler)
- `messages/en.json` + `messages/es.json` (i18n)