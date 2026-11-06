/** Texto legible de un rechazo; Tauri rechaza con el texto de error de Rust, no con un `Error`. */
export function describeError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
