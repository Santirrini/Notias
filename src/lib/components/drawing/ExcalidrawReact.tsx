/**
 * ExcalidrawReact — thin React wrapper around `@excalidraw/excalidraw` that
 * mounts inside a Svelte 5 host element via `createRoot`. This file is the
 * only place that knows Excalidraw is React-based; the Svelte side talks to
 * it through a plain imperative API (`drawRef.current.{exportSvg, hasContent}`).
 *
 * Why React inside Svelte?
 * - `@excalidraw/excalidraw` ships exclusively as a React component (no
 *   vanilla build). The bundle is loaded lazily behind the modal so it does
 *   not affect startup cost.
 * - The Svelte-side `ExcalidrawModal.svelte` mounts this component, then calls
 *   the imperative helpers exposed on the forwarded ref.
 */
import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
} from "react";
import { Excalidraw, exportToSvg } from "@excalidraw/excalidraw";

/** The element type the canvas stores. Excalidraw does not re-export
 *  `ExcalidrawElement` from its public entry, so we derive it from the
 *  type of `excalidrawAPI`'s first argument using the public API surface. */
type ImperativeAPI = Parameters<
  Extract<
    Parameters<typeof Excalidraw>[0]["excalidrawAPI"],
    (...args: any[]) => any
  >
>[0];
type ExcalidrawElement = NonNullable<
  ReturnType<ImperativeAPI["getSceneElementsIncludingDeleted"]>
>[number];
/** What `initialData.elements` accepts (Excalidraw wraps each element with an
 *  ordering id on restore). */
type OrderedExcalidrawElement = Parameters<
  ImperativeAPI["updateScene"]
>[0]["elements"] extends readonly (infer E)[] | undefined ? E : never;

export type ExcalidrawDrawingHandle = {
  /** Returns the SVG serialized bytes (for the in-doc preview) plus the
   *  `.excalidraw` JSON bytes (for re-opening the editor later). */
  serialize: () => Promise<{ svg: Uint8Array; state: Uint8Array }>;
  /** True when the user has drawn at least one element — guards against
   *  saving empty drawings. */
  hasContent: () => boolean;
};

export type ExcalidrawDrawingProps = {
  /** Elements to load on first render. Pass `null` for a blank canvas. */
  initialElements?: readonly ExcalidrawElement[] | null;
  /** Called whenever the scene changes — wired from the modal for autosave. */
  onChange?: (elements: readonly ExcalidrawElement[]) => void;
  /** Called when the user clicks Save in our custom footer. */
  onSave: () => void;
  /** Called when the user clicks Cancel / dismisses the modal. */
  onCancel: () => void;
  /** Localized labels injected by the Svelte parent (Paraglide strings). */
  labels: {
    save: string;
    cancel: string;
    saveAndClose: string;
    empty: string;
  };
};

export const ExcalidrawDrawing = forwardRef<
  ExcalidrawDrawingHandle,
  ExcalidrawDrawingProps
>(function ExcalidrawDrawing(props, ref) {
  const { initialElements, onChange, onSave, onCancel, labels } = props;
  // Use the same API surface as `excalidrawAPI` callback receives. We type it
  // as `unknown` here so we don't pull in Excalidraw's internal types, and
  // narrow per-call where needed.
  const apiRef = useRef<ImperativeAPI | null>(null);
  const [hasContent, setHasContent] = useState(Boolean(initialElements?.length));

  useImperativeHandle(
    ref,
    (): ExcalidrawDrawingHandle => ({
      serialize: async () => {
        if (!apiRef.current) {
          return { svg: new Uint8Array(), state: new Uint8Array() };
        }
        const elements = apiRef.current.getSceneElementsIncludingDeleted();
        const appState = apiRef.current.getAppState();
        const files = apiRef.current.getFiles();
        // exportToSvg is the official helper — it produces a complete <svg>
        // with inlined fonts (Virgil, Cascadia) so the preview renders the
        // same hand-drawn aesthetic without web-font dependencies.
        const svg = await exportToSvg({
          elements,
          appState: { ...appState, exportBackground: true },
          files,
        });
        // Serialize SVG: prepend XML decl so the backend writes a valid file.
        const serializer = new XMLSerializer();
        const svgString =
          '<?xml version="1.0" encoding="UTF-8"?>\n' +
          serializer.serializeToString(svg);
        const svgBytes = new TextEncoder().encode(svgString);
        // State JSON — what we persist as `.excalidraw` to re-open the scene
        // later. Includes only elements (locked + regular); appState is
        // intentionally omitted to keep the file small and to discard view
        // noise (zoom, scroll, selection).
        const stateJson = JSON.stringify(
          { version: 2, source: "notias", elements },
        );
        const stateBytes = new TextEncoder().encode(stateJson);
        return { svg: svgBytes, state: stateBytes };
      },
      hasContent: () => hasContent,
    }),
    [hasContent],
  );

  const handleChange = useCallback(
    (elements: readonly ExcalidrawElement[]) => {
      setHasContent(elements.length > 0);
      onChange?.(elements);
    },
    [onChange],
  );

  // Excalidraw's initialData expects appState too. We pass minimal defaults
  // so the canvas opens with a sensible view (no grid, transparent bg).
  const initialData = initialElements
    ? {
        elements: initialElements as readonly OrderedExcalidrawElement[],
        appState: { viewBackgroundColor: "transparent" },
      }
    : undefined;

  // Excalidraw uses the global `dark` class to switch theme — we observe the
  // <html> class to keep the canvas in sync with mode-watcher.
  const [dark, setDark] = useState(
    () =>
      typeof document !== "undefined" &&
      document.documentElement.classList.contains("dark"),
  );
  useEffect(() => {
    if (typeof document === "undefined") return;
    const obs = new MutationObserver(() => {
      setDark(document.documentElement.classList.contains("dark"));
    });
    obs.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => obs.disconnect();
  }, []);

  return (
    <div className="exc-shell" data-dark={dark ? "true" : "false"}>
      <Excalidraw
        excalidrawAPI={(api) => {
          apiRef.current = api;
        }}
        initialData={initialData}
        onChange={handleChange}
        theme={dark ? "dark" : "light"}
        UIOptions={{
          canvasActions: {
            saveToActiveFile: false,
            loadScene: false,
            export: false,
            clearCanvas: false,
          },
        }}
      />
      <div className="exc-footer">
        {hasContent ? (
          <span className="exc-hint">{labels.saveAndClose}</span>
        ) : (
          <span className="exc-hint exc-hint--empty">{labels.empty}</span>
        )}
        <div className="exc-actions">
          <button type="button" className="exc-btn exc-btn--ghost" onClick={onCancel}>
            {labels.cancel}
          </button>
          <button
            type="button"
            className="exc-btn exc-btn--primary"
            onClick={onSave}
            disabled={!hasContent}
          >
            {labels.save}
          </button>
        </div>
      </div>
    </div>
  );
});
