/** El diálogo nativo lo abre el plugin de Tauri. No lee el contenido de los archivos. */
export abstract class FileDialog {
  abstract openFiles(): Promise<string | string[] | null>;
  abstract openFolder(): Promise<string | string[] | null>;
}
