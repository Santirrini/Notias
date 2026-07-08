// shadcn-svelte expects this file at $lib/utils.js.
// We keep runtime exports plus JSDoc typedefs so TypeScript can resolve
// type imports from this .js module (allowJs: true in tsconfig).

import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

/**
 * Merge Tailwind class names with conflict resolution.
 * @param  {...any} inputs
 * @returns {string}
 */
export function cn(...inputs) {
  return twMerge(clsx(inputs));
}

/**
 * Augment any element-props object with an optional `ref` binding of
 * the matching HTML element type.
 * @template T
 * @template {HTMLElement} [U=HTMLElement]
 * @typedef {T & { ref?: U | null }} WithElementRef
 */

/**
 * Strip the `child` prop from a base.
 * @template T
 * @typedef {Omit<T, 'child'>} WithoutChild
 */

/**
 * Strip `children` and `child` from a base.
 * @template T
 * @typedef {Omit<T, 'child' | 'children'>} WithoutChildren
 */

/**
 * Strip `children` and `child`, keep optional `children` as Snippet.
 * (shadcn-svelte variant)
 * @template T
 * @typedef {Omit<T, 'child' | 'children'> & { children?: any }} WithoutChildrenOrChild
 */

/**
 * Strip everything children-related and bind a ref.
 * @template T
 * @template {HTMLElement} [U=HTMLElement]
 * @typedef {WithElementRef<WithoutChildren<T>, U>} WithElementRefChildren
 */
