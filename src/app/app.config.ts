import { ApplicationConfig, provideBrowserGlobalErrorListeners } from '@angular/core';
import { FileDialog } from './import/file-dialog';
import { ImportTransport } from './import/import-transport';
import { TauriFileDialog } from './import/tauri-file-dialog';
import { TauriImportTransport } from './import/tauri-import-transport';

export const appConfig: ApplicationConfig = {
  providers: [
    provideBrowserGlobalErrorListeners(),
    { provide: FileDialog, useExisting: TauriFileDialog },
    { provide: ImportTransport, useExisting: TauriImportTransport },
  ],
};
