import type { InjectionKey } from "vue";

// What a Field tells the control inside it: the id its <label for> names, and
// the id of its help or error text for `aria-describedby`.
export type FieldContext = { id: string; describedBy: () => string | undefined; invalid: () => boolean };
// A group field provides `null`, so the several controls inside it name
// themselves rather than share one id.
export const fieldKey: InjectionKey<FieldContext | null> = Symbol("mv-field");
