import { ApplicationConfig, provideBrowserGlobalErrorListeners } from '@angular/core';
import { FileDialog } from './import/file-dialog';
import { TauriFileDialog } from './import/tauri-file-dialog';

export const appConfig: ApplicationConfig = {
  providers: [
    provideBrowserGlobalErrorListeners(),
    { provide: FileDialog, useExisting: TauriFileDialog },
  ],
};
