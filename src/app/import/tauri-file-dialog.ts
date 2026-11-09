import { Injectable } from '@angular/core';
import { open } from '@tauri-apps/plugin-dialog';
import { FileDialog } from './file-dialog';

@Injectable({ providedIn: 'root' })
export class TauriFileDialog extends FileDialog {
  openFiles(): Promise<string | string[] | null> {
    return open({
      title: 'Elegir archivos',
      multiple: true,
      directory: false,
    });
  }

  openFolder(): Promise<string | string[] | null> {
    return open({
      title: 'Elegir carpeta',
      multiple: false,
      directory: true,
    });
  }
}
