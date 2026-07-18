# Notias

> **Aplicación de escritorio de notas Markdown con IA integrada, local-first.**
> Markdown en disco · búsqueda FTS5 · wiki-links · LLM local (Ollama) · fallback a LLM en la nube (OpenAI, Groq) · transcripción Whisper · estudio con repetición espaciada (SM-2) · tablero de tareas · espejo de Google Calendar · sincronización por zip.
> Construida como **Tauri 2** con frontend **SvelteKit (SPA)** y núcleo **Rust** sobre **SQLite** embebido (WAL, FTS5).

---

## Tabla de contenidos

- [Resumen del proyecto](#resumen-del-proyecto)
- [Alcance (fases)](#alcance-fases)
- [Arquitectura](#arquitectura)
- [Stack tecnológico](#stack-tecnológico)
- [Pros y contras del stack](#pros-y-contras-del-stack)
- [Cómo ejecutarlo en desarrollo](#cómo-ejecutarlo-en-desarrollo)
- [Cómo compilar](#cómo-compilar)
- [Verificación y estado actual](#verificación-y-estado-actual)
- [Google Calendar (OAuth PKCE)](#google-calendar-oauth-pkce)
- [Estructura del repositorio](#estructura-del-repositorio)
- [Limitaciones conocidas y fuera de alcance](#limitaciones-conocidas-y-fuera-de-alcance)

---

## Resumen del proyecto

**Notias** es una aplicación de escritorio pensada como un *cuaderno personal* que vive enteramente en tu máquina. Toda la información que generas — notas, embeddings, tarjetas SRS, planes, tareas, caché del calendario — se persiste localmente; las integraciones externas (IA en la nube, transcripción Whisper, Google Calendar) son **opt-in** y funcionan sobre APIs estándar (HTTP/REST + OAuth) sin servicios propietarios de terceros.

Principios de diseño:

| Principio | Qué significa |
|-----------|---------------|
| **Local-first** | El Markdown es la fuente de verdad (`notes/*.md`); SQLite es un índice derivado. Puedes dejar de usar la app y conservar tus notas. |
| **Sin dependencia de proveedor de IA** | Trait `Provider` en Rust. Arranca con **Ollama** local; escala a **OpenAI** o **Groq** con solo cambiar la prioridad. |
| **Privacidad por defecto** | Las claves de API y los tokens OAuth viven en el **keyring del sistema operativo**. El binario nunca contiene un `client_secret`. |
| **Recuperable** | Sidecar `notias.meta` con `db_hash` + `schema_version`; si el índice se desincroniza, `RecoveryBanner` ofrece reconstruir desde los `.md`. |
| **Sincronización opcional** | Exportación/importación `.zip` de las notas; el usuario decide dónde alojarlo (su nube preferida). El `.db` nunca se sincroniza. |

---

## Alcance (fases)

Las 6 fases del diseño (`specs/2026-07-03-notias-design.md`) están **completas, etiquetadas y commiteadas**, más una fase 7 (*UI redesign*) posterior al MVP. Total: **52 commits** en `main`.

| Fase | Etiqueta | Alcance |
|------|----------|---------|
| **0 · Cimientos** | `phase-0-cimientos` | Tauri 2 + SvelteKit SPA + SQLite embebido + migraciones (`0001`–`0006`). |
| **1 · MVP Notas** | `phase-1-mvp-notas` | Editor Markdown + persistencia en disco + FTS5 + wiki-links `[[…]]`. |
| **2 · IA local** | `phase-2-ia-local` | Ollama vía HTTP + chat con RAG + worker de embeddings en segundo plano. |
| **3 · IA en la nube + STT** | `phase-3-cloud-stt` | OpenAI + Groq + Whisper (Groq); router con fallback local→nube; claves de API en keyring. |
| **4 · Estudio autónomo** | `phase-4-estudio` | SRS con SM-2 y grader en UI + generador de cuestionarios (mc / short / cloze) + plan semanal asistido por IA + tablero kanban de tareas. |
| **5 · Sincronización opcional** | `phase-5-sync` | Export/import manual por zip + rebuild al arrancar para archivos modificados externamente + watcher en vivo de la carpeta. |
| **6 · Google Calendar** | `phase-6-calendar` | OAuth 2.0 PKCE + Calendar API v3 + espejo local con last-write-wins. Sin `client_secret` en el binario. |
| **7 · Rediseño de UI** *(post-MVP)* | `phase-7-ui-redesign` | Chrome de 3 paneles (NavRail · Sections · Pages · NoteCanvas) + CommandPalette (Ctrl+K) + cambio de tema + RecoveryBanner + secretos centralizados en `commands.rs`. |

Verifica cada fase con `pnpm tauri dev` y `cargo run --example selfcheck` (ejercita el esquema, SM-2, round-trip del plan, round-trip del zip + detección de cambios externos, PKCE, espejo).

---

## Arquitectura

```
┌─────────────────────────────────────────────────────────┐
│  SvelteKit SPA (Svelte 5 runes · TS estricto)          │
│   routes/    layout, notas, chat, calendario,           │
│              settings, study, tasks                     │
│   lib/       components (chrome 3 paneles, palette, …)  │
│              stores (notes/chat/srs/study/tasks/…)      │
│              ipc.ts  ──► @tauri-apps/api invoke        │
└────────────────────────┬────────────────────────────────┘
                         │ IPC de Tauri (puente tipado)
┌────────────────────────┴────────────────────────────────┐
│  Núcleo en Rust (notias_lib)                            │
│  ├─ notes/   store · index · wikilinks · sync · watcher │
│  ├─ ai/      Trait Provider + Ollama/OpenAI/Groq +      │
│  │           Router (fallback local→nube) + embed jobs  │
│  ├─ tasks/   CRUD + resumen kanban                      │
│  ├─ srs/     Planificador SM-2                          │
│  ├─ study/   Calificación de cuestionarios + generador  │
│  │           de plan semanal                            │
│  ├─ calendar/  Cliente Calendar API + espejo LWW        │
│  ├─ oauth/   Flujo PKCE de Google, tokens en keyring    │
│  ├─ db/      rusqlite, migraciones v1–v6, FTS5, WAL     │
│  ├─ secrets/ Wrapper del keyring del SO                 │
│  └─ error/   AppError → formato alambre JSON            │
└────────────────────────┬────────────────────────────────┘
                         │
   ┌─────────────────────┼─────────────────────┬──────────────┐
   │                     │                     │              │
SQLite              Archivos             Keyring del SO   HTTP/REST
notias.db           notes/*.md           provider_*       Ollama / OpenAI
WAL, solo           (fuente de verdad    google_*         Groq / Google Cal.
índice derivado)    + frontmatter YAML)  (tokens/claves)
                                       notias.meta       (db_hash + schema_ver)
```

**Reglas de oro:**

1. `notes/*.md` es la fuente de verdad. `notias.db` se puede borrar y reconstruir.
2. `notias.meta` acompaña a la base: detecta corrupción sin abrir todo el esquema.
3. Las claves viven en el keyring; nunca en el binario ni en el repositorio.
4. La UI nunca accede al sistema de archivos directamente: todo pasa por `invoke` (tipado en `ipc.ts`).

---

## Stack tecnológico

### Frontend

| Capa | Tecnología | Por qué |
|------|-----------|---------|
| Framework | **SvelteKit 2** (modo SPA con `adapter-static`) | Bundle pequeño, sin servidor embebido (Tauri provee la ventana), reactividad fina con **Svelte 5 runes**. |
| Lenguaje | **TypeScript** estricto (`strict: true`, `checkJs`) | Refleja los tipos del puente IPC desde Rust. |
| Estilos | **Tailwind CSS v4** + `tailwind-variants` + `clsx` / `tailwind-merge` | Utility-first + variantes tipadas; consistencia entre componentes. |
| Componentes | **bits-ui** (primitivos accesibles) + **shadcn-svelte** `components.json` | Primitivos sin estilos impuestos; accesibles por defecto. |
| Iconos | **lucide-svelte** | Set coherente, tree-shakeable. |
| Editor | **@milkdown/core** + `preset-commonmark` + `theme-nord` sobre **ProseMirror** | Editor WYSIWYG/Markdown con esquema extensible; ProseMirror gestiona los wiki-links. |
| Estado | **Svelte stores `.svelte.ts`** con runes | Sin librería extra (Redux/Zustand). |
| Miscelánea | `mode-watcher`, `svelte-sonner`, `paneforge` | Modo claro/oscuro, toasts, paneles redimensionables. |

### Backend (Rust)

| Capa | Tecnología | Por qué |
|------|-----------|---------|
| Shell | **Tauri 2** | Binarios pequeños, WebView nativo, IPC tipado. |
| Base de datos | **rusqlite** (`bundled`) + **sqlite-vec** + **FTS5** | Embebido (sin DLL externa), búsqueda full-text, embeddings vectoriales. |
| HTTP | **reqwest** con `rustls-tls`, `json`, `multipart` | Sin dependencia de OpenSSL nativo; soporta multipart de Whisper. |
| Async | **tokio** (multi-thread, macros, time, sync) | Embed worker + watcher en segundo plano. |
| Watcher | **notify** v6 | Cambios de archivos en `notes/` en vivo. |
| Zip | **zip** (deflate) | Export/import para sincronización. |
| Secretos | **keyring** v2 | Keyring del SO (Windows Credential Manager / macOS Keychain / Secret Service). |
| Cripto / IDs | **sha2**, **hex**, **ulid**, **rand**, **base64** | `db_hash`, IDs ordenables. |
| Errores | **thiserror** + tipo `AppError` con `serde` | Errores estructurados hacia el frontend. |
| Fechas | **time** (formatting) | Sin `chrono`. |
| Configuración | `tauri.conf.json` + `capabilities/default.json` | Modelo de permisos explícito. |

### Empaquetado

| Plataforma | Bundle |
|------------|--------|
| Windows | **MSI + NSIS** (producidos con `pnpm tauri build`). |
| macOS / Linux | Targets nativos de Tauri 2. |

---

## Pros y contras del stack

### ✅ A favor

**Tauri 2 (vs Electron)**
- Binarios **5–20× más pequeños** (no incluye Chromium).
- **Menor uso de RAM** (WebView nativo ya presente en el sistema).
- Modelo de **permisos explícito** por *capability*: el frontend solo invoca lo declarado.
- IPC tipado de extremo a extremo (Rust → TS).

**SvelteKit SPA + Svelte 5 runes**
- Bundle de JS significativamente menor que alternativas en React/Vue.
- Reactividad sin boilerplate; los `rune` granulares evitan renders innecesarios.
- `adapter-static` produce una SPA pura, sencilla de empaquetar dentro de Tauri.

**Núcleo en Rust + SQLite embebido (rusqlite `bundled`)**
- Sin DLL que desplegar, sin diferencia entre máquinas del equipo.
- **FTS5** + **sqlite-vec** dan búsqueda full-text y embeddings en el mismo motor.
- Migraciones versionadas en `.sql` puros bajo control de versiones.

**Markdown como fuente de verdad**
- Portabilidad total (cualquier editor abre `notes/*.md`).
- *Diff/merge* triviales; amigable con el control de versiones.
- Resistente a corrupción (se reconstruye el índice desde disco).

**Local-first con IA opcional**
- Funciona **sin nube**: Ollama local basta.
- Fallback automático a OpenAI/Groq si Ollama no responde.
- Las claves de API **nunca** tocan el disco de la app (keyring).

**OAuth PKCE (sin `client_secret`)**
- El binario es redistribuible sin filtrar secretos.
- Cada usuario aporta su propio `client_id` de Google.

### ⚠️ En contra / compromisos

**Tauri 2 (vs Electron)**
- WebView por sistema operativo: pequeñas inconsistencias entre Windows (WebView2), macOS (WebKit) y Linux (WebKitGTK). Hay que probar en cada uno.
- Ecosistema de plugins más joven que el de Electron.
- Costo inicial en Rust: el equipo necesita manejar *ownership*, *async* y *lifetimes*.

**Svelte 5 runes**
- API todavía reciente (2026); hay menos material acumulado que para React.
- Algunas librerías aún apuntan a Svelte 4; conviene verificar compatibilidad (`bits-ui` ya soporta 5).

**SQLite embebido**
- No escala a múltiples procesos escritores (en este proyecto el watcher usa **su propia** `Connection` para evitar bloqueos en el `Mutex<AppState>`).
- Los respaldos concurrentes requieren la API `sqlite3 .backup` o `VACUUM INTO` para un *snapshot* consistente.
- Las migraciones destructivas hay que escribirlas a mano con cuidado.

**Markdown en disco + DB como índice**
- Dos fuentes de verdad en potencia: exige `rebuild_if_stale` al arrancar más un watcher.
- El editor sobre disco puede ir lento con notas muy grandes (no hay virtualización todavía).
- Los wiki-links y el frontmatter se parsean en cada indexación; con corpus muy grandes conviene memoizar.

**LLM local con Ollama**
- Calidad variable según el modelo descargado (lo elige el usuario).
- *Drift* del modelo de embeddings: si cambias de modelo, los embeddings viejos quedan obsoletos (mitigado con `re-embed` y la columna `model` en `note_embeddings`).
- Whisper vía Groq requiere subir audio a la nube; la alternativa local exige Whisper.cpp, fuera del MVP.

**OAuth PKCE + `client_id` por usuario**
- Cada usuario debe crear su propio cliente OAuth en Google Cloud Console (fricción en el primer arranque).
- Aceptable para uso personal/educativo; para distribución empresarial habría que pasar a un *backend-proxy* (ver *Riesgos* en `docs/PROJECT_STATUS.md`).

**Sincronización manual (zip)**
- Sin CRDT ni *version vectors*; **last-write-wins** sobre `.md` por mtime.
- Sin UI de resolución de conflictos (diferido por diseño).
- Adecuado para uso individual; no para colaboración en tiempo real.

**Observabilidad**
- Sin telemetría; los errores se registran con `tracing` + `ErrorOverlay.svelte`. Para diagnosticar en producción hay que reproducir localmente o agregar un *sink* de logs opcional.

---

## Cómo ejecutarlo en desarrollo

Requisitos: **Rust stable**, **pnpm**, **Node 20+**; **Ollama** opcional (para las funciones de IA local).

```bash
pnpm install
pnpm tauri dev
```

El primer arranque crea `notias.db`, aplica las migraciones y levanta el watcher de `notes/`.

---

## Cómo compilar

```bash
pnpm tauri build
```

Artefactos en `src-tauri/target/release/bundle/`:
- **Windows:** `.msi` y `.exe` (NSIS).
- **macOS:** `.app` y `.dmg`.
- **Linux:** `.deb` / `.AppImage` / `.rpm` según el target.

Build sólo del frontend:

```bash
pnpm build         # svelte-kit build → ../build/
```

---

## Verificación y estado actual

**Windows (línea base 2026-07-06, esta máquina):** todo pasa.

| Check | Resultado | Notas |
|-------|-----------|-------|
| `cargo check` | ✅ PASS | 0 errores, 3 *warnings* (imports/variables sin usar). |
| `cargo test --lib` | ✅ PASS | **62/62**, 1 ignorado (smoke de keyring requiere DPAPI). |
| `cargo run --example selfcheck` | ✅ PASS | Invariantes de las fases 3–6 verificadas. |
| `pnpm check` | ✅ PASS | 0 errores, 1 *warning* intencional. |
| `pnpm build` | ✅ PASS | SvelteKit static build. |
| `pnpm tauri build` | ✅ PASS | MSI + NSIS producidos. |
| Smoke del `.exe` | ✅ PASS | WebView2 arranca sin crashear. |
| `pnpm tauri dev` | ✅ PASS | Sidebar + Notas/Chat/Calendario/Settings/Tasks/Study renderizan. |

Toolchain confirmado: **MSVC v14.44.35207** + **Windows SDK 10.0.28000** (completo, incluye `Lib`). Setup detallado en `docs/WINDOWS_BUILD.md`. Resumen por fase en `docs/PROJECT_STATUS.md`.

---

## Google Calendar (OAuth PKCE)

Notias usa OAuth 2.0 con **PKCE**, por lo que **no hay `client_secret` en el binario**. Para activarlo:

1. Crear un cliente OAuth en <https://console.cloud.google.com/>.
   - **Application type:** `Web application`
   - **Authorized redirect URI:** `http://127.0.0.1:PORT/callback` (cualquier puerto; Notias vincula uno efímero).
2. Copia el `client_id` y edita `src-tauri/src/oauth/google.rs`, reemplazando el placeholder en `OAUTH_CLIENT_ID`.
3. Recompila (`pnpm tauri build`). El primer *Connect…* abre tu navegador en la página de autorización de Google; tras autorizar, el redirect a `127.0.0.1` lo captura automáticamente.

Los tokens se guardan en el keyring del SO. El espejo local usa **last-write-wins** sobre `db_hash` + `schema_version` por evento.

---

## Estructura del repositorio

```
notias/
├── src-tauri/                          Núcleo en Rust (notias_lib)
│   ├── Cargo.toml, Cargo.lock
│   ├── tauri.conf.json, build.rs
│   ├── capabilities/default.json       Permisos explícitos
│   ├── migrations/                    0001..0006 .sql
│   ├── examples/selfcheck.rs           Invariantes ejecutables
│   └── src/                            lib.rs + 11 módulos
├── src/                                SvelteKit SPA
│   ├── routes/                         +layout, layout.ts, +page, notes/[id],
│   │                                   chat/, calendar/, settings/,
│   │                                   study/, tasks/
│   ├── lib/
│   │   ├── components/                 Chrome 3 paneles, palette, editor, …
│   │   ├── stores/                     runes .svelte.ts
│   │   ├── ipc.ts                      Puente tipado con Tauri
│   │   ├── editor/  hooks/  utils.js
│   │   └── types.ts, types/
│   ├── app.css, app.html
│   └── hooks.client.ts
├── docs/
│   ├── PROJECT_STATUS.md               Resumen por fase + matriz de verificación
│   ├── WINDOWS_BUILD.md                Setup del toolchain en Windows
│   ├── test-checklist.md
│   ├── test-checklist-ui.md
│   └── superpowers/{specs,plans}/      Diseño y planes por fase
├── components.json                     shadcn-svelte
├── svelte.config.js                    adapter-static SPA
├── vite.config.js                      Amigable con Tauri (puerto 1420)
├── tsconfig.json                       Estricto
├── package.json, pnpm-lock.yaml
└── README.md
```

---

## Limitaciones conocidas y fuera de alcance

**Diferidos por diseño (§13, §15 del spec):**

| Ítem | Por qué se difiere |
|------|--------------------|
| Matriz de CI multi-OS | No hay *runner* configurado. |
| `Remove` del watcher → borrado lógico del índice | Requiere UI de conflictos que aún no existe. |
| Editar/borrar eventos de Calendar desde la UI | Primero se entrega lectura + creación. |
| *Merge* a 3 bandas en Calendar LWW | Sin ediciones locales compitiendo, no aporta. |
| Refresco periódico en segundo plano de Calendar | *Sync now* manual alcanza para el MVP. |
| Mobile (iOS/Android) | Fuera de alcance. |
| Colaboración en tiempo real / CRDT | Fuera de alcance. |
| Cifrado E2E | Fuera de alcance. |
| Backend en la nube propio | Fuera de alcance — el sync es tu nube (Drive/Dropbox/iCloud) por encima del zip. |
| LLM/Whisper empaquetados en el binario | Tamaño y licencias; el usuario aporta su *runtime* (Ollama) o usa la nube. |
| Marketplace de temas | Fuera de alcance. |

**Riesgos reconocidos (spec §14):**

- *`client_id` de OAuth en el binario* — aceptable para uso personal/educativo; documentar *backend-proxy* antes de distribución empresarial.
- *Drift* del modelo de embeddings — mitigable con `re-embed` y la columna `model` por fila (ya en el esquema).
- *Corrupción de la DB* — sidecar `notias.meta` + `RecoveryBanner` (reconstruye desde `.md`).
- *Conflictos de sync* — LWW sobre `.md` por mtime; CRDT explícitamente fuera del alcance.

---

**Hecho con:** Tauri 2 · SvelteKit 2 · Svelte 5 · TypeScript · Tailwind v4 · bits-ui · Milkdown · ProseMirror · Rust · rusqlite · FTS5 · sqlite-vec · reqwest · Ollama · OpenAI · Groq · Whisper · keyring · notify · zip · Google Calendar API · OAuth 2.0 PKCE.
