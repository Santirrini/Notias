# Migración profesional de `src/lib/ipc.ts` a `safeInvoke`

## Objetivo

Que **todas** las llamadas a Tauri pasen por `safeInvoke`, con dos propiedades garantizadas:

1. La firma pública de cada función en `$lib/ipc` devuelve `Promise<InvokeResult<T>>` (discriminated union). TypeScript obliga a los call-sites a manejar `r.ok === false`.
2. La señal `r.offline` distingue "no hay runtime Tauri" de "error real del backend" — los stores con fallback local usan esa señal para enrutar; los call-sites sin fallback muestran el error al usuario vía toast.

## Cambios

### 1. `src/lib/ipc.ts` — reescritura completa

Cada función pasa de `invoke<T>(...)` a `safeInvoke<T>(...)`. Las dos excepciones:

- `aiChat(req)` → `safeInvoke<StreamHandle>('ai_chat', { req })`.
- `ping()` se queda igual de forma explícita (`safeInvoke<string>('ping')`).

```ts
// patrón para todo el archivo
export const listNotes = (tag?: string) =>
  safeInvoke<NoteSummary[]>('list_notes', { tag });

export const getNote = (id: string) =>
  safeInvoke<Note>('get_note', { id });

// … uno por cada export actual, ~40 funciones
```

`safeInvoke` se importa de `$lib/stores/backend.svelte`. Los types (`WireError`, `Note`, `StreamHandle`, etc.) no cambian. El comentario JSDoc en la cabecera del archivo explica el contrato nuevo y la diferencia entre `offline` y `error`.

### 2. `src/lib/stores/backend.svelte.ts` — refinamiento

- Exportar también el tipo `InvokeResult<T>` desde aquí (ya está; sólo confirmar que se re-exporta desde `$lib/ipc` para no obligar a importar dos paths).
- Añadir un helper `unwrap<T>(r: InvokeResult<T>): T | null` que colapsa a `null` para los call-sites que no les importa la causa (idéntico a `safeAiComplete`, etc., pero genérico).
- Pequeña mejora: en `safeInvoke`, cuando `cmd` falla con `r.ok === false && !r.offline`, despachar también el evento `notias:error` con un `level: "warning"` para que `ErrorOverlay` lo registre (gated por `import.meta.env.DEV` igual que ahora).

### 3. Stores — migración call-site por call-site

**`src/lib/stores/notes.svelte.ts`** (el más complejo, tiene fallback local)

Patrón nuevo:

```ts
async load() {
  const r = await safeInvoke<NoteSummary[]>('list_notes');
  if (r.ok) { this.items = r.value; return; }
  if (r.offline) {
    this.items = readLocalNotes().map(noteFromLocal);
    return;
  }
  this.error = r.error;
  // toast.error(r.error) o similar
}
```

Mismo patrón en `search`, `getOne`, `create`, `update`, `remove`. Cuando hay error real y no offline, mostrar toast (`$lib/stores/toast` o sonner). El campo `error` del store queda para errores "estructurales" (validación, etc.).

**`src/lib/stores/srs.svelte.ts`**

3 paths sin try/catch (`suspendCard`, `generate`, `saveBatch`) reciben el envoltorio explícito. `queue/grade/review` ya tienen try/catch — se actualiza la captura para usar `r.error` en vez de `(e as { message: string }).message`.

**`src/lib/stores/study.svelte.ts`**

`PlanStore.persist()` y `StudyStore.submit()` se blindan; el resto ya tiene estructura, sólo cambia la fuente del mensaje.

**`src/lib/stores/chat.svelte.ts`**

Dos cambios:

- `ragSearch` y `aiChat` pasan a usar `safeInvoke`.
- Re-suscripción por stream: en `send()` se cancela el listener anterior (`this.unlisten?.()`) y se registra uno nuevo con `safeListen<StreamEvent>(`provider://stream/${handle.stream_id}`, handler)` usando el id concreto. Esto restaura el filtrado por stream_id que perdimos en la ronda anterior.

### 4. Componentes — unificación de try/catch

`RecoveryBanner.svelte` (bug latente — no tenía catch), `SyncSettings.svelte`, `ProviderCard.svelte`, `CalendarSettings.svelte`, `CommandPalette.svelte`, `NoteCanvas.svelte`, `PageList.svelte`, `SRSReviewer.svelte`, `WeeklyPlan.svelte` (vía store) — todos pasan a leer `r.error` directamente cuando `!r.ok`. Los que tenían fallback local ahora ramifican en `r.offline` vs `r.error` según la decisión de diseño.

`+page.svelte` (home) — sustituye el `ping()` + llamadas manuales a `backend.setAvailable/setUnavailable` por `safeInvoke<string>('ping')`. El store `backend` se actualiza solo.

`+layout.svelte` — `recoveryRequired` cambia a `safeInvoke<boolean>('recovery_required')`. Si `r.offline`, sigue mostrando el banner de recovery sólo si se quiere (probablemente no: el recovery sólo tiene sentido con backend). Si `!r.ok && !r.offline`, log.

`src/routes/settings/+page.svelte` — `toggle` (sin try/catch) se arregla.

`src/routes/calendar/+page.svelte` — tres llamadas se unifican.

### 5. Test mínimo del contrato

Añadir `src/lib/stores/backend.test.ts` (vitest). Sólo dos tests, suficientes para anclar el contrato:

```ts
- safeInvoke returns { ok: true, value } when invoke resolves
- safeInvoke returns { ok: false, offline: true } when invoke is undefined
- safeInvoke returns { ok: false, offline: false, error } when invoke throws
```

Esto bloquea regresiones futuras. No requiere tocar el resto del repo.

### 6. CI (ya está)

El workflow existente en `.github/workflows/ci.yml` ejecuta `pnpm check` y `pnpm build` — el refactor tiene que pasar ambos. Añado `pnpm test` cuando se añada vitest.

## Archivos modificados / creados

**Modificados (~20):**
- `src/lib/ipc.ts` (reescrito)
- `src/lib/stores/backend.svelte.ts` (helpers)
- `src/lib/stores/notes.svelte.ts`
- `src/lib/stores/srs.svelte.ts`
- `src/lib/stores/study.svelte.ts`
- `src/lib/stores/chat.svelte.ts` (re-suscripción por stream)
- `src/lib/components/RecoveryBanner.svelte`
- `src/lib/components/SyncSettings.svelte`
- `src/lib/components/ProviderCard.svelte`
- `src/lib/components/CalendarSettings.svelte`
- `src/lib/components/CommandPalette.svelte`
- `src/lib/components/NoteCanvas.svelte`
- `src/lib/components/PageList.svelte`
- `src/lib/components/SRSReviewer.svelte`
- `src/routes/+layout.svelte`
- `src/routes/+page.svelte`
- `src/routes/calendar/+page.svelte`
- `src/routes/settings/+page.svelte`
- `package.json` (añadir `vitest` + `test` script)
- `tsconfig.json` (types de vitest si aplica)

**Creados:**
- `src/lib/stores/backend.test.ts`

## Verificación

1. `pnpm check` — 0 errores.
2. `pnpm test` — 3 tests pasan.
3. `pnpm build` — 0 errores.
4. Arranco `pnpm dev` y recorro las 7 rutas con Chrome headless + CDP: 0 excepciones no capturadas, todas las páginas renderizan.
5. `pnpm tauri dev` arranca, compila incremental y la ventana abre.
6. (Manual) Abro `/notes` con Tauri corriendo, creo/edito/borro una nota — confirma que el flujo online sigue funcionando idéntico al actual.

## Riesgos y mitigaciones

- **Riesgo de regresión en flujos online.** Mitigación: la firma nueva de `ipc.ts` cambia *cómo* se llama, no *qué* hace. Cada call-site se revisa individualmente y los stores con fallback local siguen funcionando offline como hoy. La verificación manual con Tauri abierto es la prueba de fuego.
- **Chat: re-suscripción por stream**. Si el backend emite eventos con un id distinto al que `ai_chat` devolvió, los chunks no llegan. Mitigación: el backend usa `id.clone()` para el canal, así que la coincidencia es exacta.
- **Complejidad del cambio.** ~20 archivos tocados, ~40 funciones migradas. Por eso es mecánico y se hace call-site por call-site sin refactors ad-hoc. Si en algún sitio el patrón no encaja, lo discutimos antes de tocarlo.

## Tiempo estimado

~30-40 minutos de ejecución si todo va bien. La mayor parte del tiempo es reescribir `ipc.ts` y los stores con fallback local.